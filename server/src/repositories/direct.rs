use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

use crate::error::AppError;
use crate::models::channel::{Channel, ChannelType};
use crate::models::direct::DirectMessageSummary;
use crate::models::user::User;
use crate::repositories::social::SocialRepo;

#[derive(Debug, Serialize)]
struct CreateDirectChannelDb {
    name: String,
    channel_type: ChannelType,
    server: Option<surrealdb::RecordId>,
    created_at: surrealdb::Datetime,
}

#[derive(Debug, Deserialize)]
struct PeerRow {
    #[serde(rename = "in")]
    user: surrealdb::RecordId,
}

#[derive(Debug, Deserialize)]
struct VisibleDirectRow {
    out: surrealdb::RecordId,
}

const LIST_VISIBLE_DIRECTS_QUERY: &str =
    "SELECT out, created_at FROM direct_visible WHERE in = $user ORDER BY created_at DESC";

#[async_trait]
pub trait DirectMessageRepo: Send + Sync {
    async fn open(
        &self,
        user_id: &str,
        friend_id: &str,
        social: &dyn SocialRepo,
    ) -> Result<DirectMessageSummary, AppError>;
    async fn list_for_user(
        &self,
        user_id: &str,
        social: &dyn SocialRepo,
    ) -> Result<Vec<DirectMessageSummary>, AppError>;
    async fn can_access(
        &self,
        channel_id: &str,
        user_id: &str,
        social: &dyn SocialRepo,
    ) -> Result<bool, AppError>;
    async fn peer_for_user(
        &self,
        channel_id: &str,
        user_id: &str,
    ) -> Result<Option<User>, AppError>;
    async fn mark_visible_for_members(&self, channel_id: &str) -> Result<(), AppError>;
}

pub struct SurrealDirectMessageRepo {
    db: Surreal<Client>,
}

impl SurrealDirectMessageRepo {
    pub fn new(db: Surreal<Client>) -> Self {
        Self { db }
    }
}

fn direct_channel_id(a: &str, b: &str) -> String {
    let (left, right) = if a <= b { (a, b) } else { (b, a) };
    let mut hash = Sha256::new();
    hash.update(left.as_bytes());
    hash.update(b":");
    hash.update(right.as_bytes());
    format!("dm_{}", hex::encode(hash.finalize()))
}

async fn assert_allowed(
    user_id: &str,
    friend_id: &str,
    social: &dyn SocialRepo,
) -> Result<(), AppError> {
    if user_id == friend_id {
        return Err(AppError::BadRequest("Cannot message yourself".into()));
    }
    if social.is_blocked(user_id, friend_id).await? || social.is_blocked(friend_id, user_id).await?
    {
        return Err(AppError::Forbidden("Cannot message this user".into()));
    }
    if !social.are_friends(user_id, friend_id).await? {
        return Err(AppError::Forbidden(
            "Private messages require friendship".into(),
        ));
    }
    Ok(())
}

#[async_trait]
impl DirectMessageRepo for SurrealDirectMessageRepo {
    async fn open(
        &self,
        user_id: &str,
        friend_id: &str,
        social: &dyn SocialRepo,
    ) -> Result<DirectMessageSummary, AppError> {
        assert_allowed(user_id, friend_id, social).await?;

        let friend = self
            .db
            .select::<Option<User>>(("user", friend_id))
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".into()))?;

        let channel_id = direct_channel_id(user_id, friend_id);
        let channel = match self
            .db
            .select::<Option<Channel>>(("channel", channel_id.as_str()))
            .await?
        {
            Some(channel) => channel,
            None => self
                .db
                .create(("channel", channel_id.as_str()))
                .content(CreateDirectChannelDb {
                    name: "direct".into(),
                    channel_type: ChannelType::Direct,
                    server: None,
                    created_at: chrono::Utc::now().into(),
                })
                .await?
                .ok_or_else(|| AppError::Internal("Failed to create direct channel".into()))?,
        };

        let user = surrealdb::RecordId::from(("user", user_id));
        let friend_rid = surrealdb::RecordId::from(("user", friend_id));
        let channel_rid = surrealdb::RecordId::from(("channel", channel_id.as_str()));

        self.db
            .query(
                "DELETE direct_member WHERE out = $channel AND in IN [$user, $friend]; \
                 RELATE $user -> direct_member -> $channel SET created_at = time::now(); \
                 RELATE $friend -> direct_member -> $channel SET created_at = time::now(); \
                 DELETE direct_visible WHERE in = $user AND out = $channel; \
                 RELATE $user -> direct_visible -> $channel SET created_at = time::now()",
            )
            .bind(("user", user))
            .bind(("friend", friend_rid))
            .bind(("channel", channel_rid))
            .await?;

        Ok(DirectMessageSummary { channel, friend })
    }

    async fn list_for_user(
        &self,
        user_id: &str,
        social: &dyn SocialRepo,
    ) -> Result<Vec<DirectMessageSummary>, AppError> {
        let mut result = self
            .db
            .query(LIST_VISIBLE_DIRECTS_QUERY)
            .bind(("user", surrealdb::RecordId::from(("user", user_id))))
            .await?;
        let channels: Vec<VisibleDirectRow> = result.take(0)?;
        let mut summaries = Vec::new();

        for row in channels {
            let channel_id = row.out.key().to_string();
            if !self.can_access(&channel_id, user_id, social).await? {
                continue;
            }
            let Some(channel) = self
                .db
                .select::<Option<Channel>>(("channel", channel_id.as_str()))
                .await?
            else {
                continue;
            };
            let Some(friend) = self.peer_for_user(&channel_id, user_id).await? else {
                continue;
            };
            summaries.push(DirectMessageSummary { channel, friend });
        }

        Ok(summaries)
    }

    async fn can_access(
        &self,
        channel_id: &str,
        user_id: &str,
        social: &dyn SocialRepo,
    ) -> Result<bool, AppError> {
        let Some(channel) = self
            .db
            .select::<Option<Channel>>(("channel", channel_id))
            .await?
        else {
            return Ok(false);
        };
        if channel.channel_type != ChannelType::Direct {
            return Ok(false);
        }
        let Some(friend) = self.peer_for_user(channel_id, user_id).await? else {
            return Ok(false);
        };
        let friend_id = friend
            .id
            .as_ref()
            .map(|id| id.key().to_string())
            .unwrap_or_default();
        if friend_id.is_empty() {
            return Ok(false);
        }
        Ok(assert_allowed(user_id, &friend_id, social).await.is_ok())
    }

    async fn peer_for_user(
        &self,
        channel_id: &str,
        user_id: &str,
    ) -> Result<Option<User>, AppError> {
        let mut result = self
            .db
            .query("SELECT in FROM direct_member WHERE out = $channel AND in != $user LIMIT 1")
            .bind((
                "channel",
                surrealdb::RecordId::from(("channel", channel_id)),
            ))
            .bind(("user", surrealdb::RecordId::from(("user", user_id))))
            .await?;
        let rows: Vec<PeerRow> = result.take(0)?;
        let Some(peer_id) = rows.first().map(|row| row.user.key().to_string()) else {
            return Ok(None);
        };
        self.db
            .select::<Option<User>>(("user", peer_id.as_str()))
            .await
            .map_err(AppError::from)
    }

    async fn mark_visible_for_members(&self, channel_id: &str) -> Result<(), AppError> {
        self.db
            .query(
                "FOR $member IN (SELECT VALUE in FROM direct_member WHERE out = $channel) { \
                   DELETE direct_visible WHERE in = $member AND out = $channel; \
                   RELATE $member -> direct_visible -> $channel SET created_at = time::now(); \
                 }",
            )
            .bind((
                "channel",
                surrealdb::RecordId::from(("channel", channel_id)),
            ))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
pub struct MockDirectMessageRepo;

#[cfg(test)]
impl MockDirectMessageRepo {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(test)]
#[async_trait]
impl DirectMessageRepo for MockDirectMessageRepo {
    async fn open(
        &self,
        _user_id: &str,
        _friend_id: &str,
        _social: &dyn SocialRepo,
    ) -> Result<DirectMessageSummary, AppError> {
        Err(AppError::Internal(
            "MockDirectMessageRepo::open not configured".into(),
        ))
    }

    async fn list_for_user(
        &self,
        _user_id: &str,
        _social: &dyn SocialRepo,
    ) -> Result<Vec<DirectMessageSummary>, AppError> {
        Ok(vec![])
    }

    async fn can_access(
        &self,
        _channel_id: &str,
        _user_id: &str,
        _social: &dyn SocialRepo,
    ) -> Result<bool, AppError> {
        Ok(false)
    }

    async fn peer_for_user(
        &self,
        _channel_id: &str,
        _user_id: &str,
    ) -> Result<Option<User>, AppError> {
        Ok(None)
    }

    async fn mark_visible_for_members(&self, _channel_id: &str) -> Result<(), AppError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::social::MockSocialRepo;
    use mockall::predicate::eq;

    #[test]
    fn direct_channel_id_is_order_independent() {
        assert_eq!(
            direct_channel_id("alice", "bob"),
            direct_channel_id("bob", "alice")
        );
        assert!(direct_channel_id("alice", "bob").starts_with("dm_"));
    }

    #[test]
    fn list_visible_directs_query_orders_by_selected_rows() {
        // SurrealDB rejects `SELECT VALUE out ... ORDER BY created_at` because
        // `created_at` is not available in a VALUE projection.
        assert!(LIST_VISIBLE_DIRECTS_QUERY.starts_with("SELECT out, created_at FROM"));
        assert!(!LIST_VISIBLE_DIRECTS_QUERY.contains("SELECT VALUE"));
        assert!(LIST_VISIBLE_DIRECTS_QUERY.contains("ORDER BY created_at DESC"));
    }

    #[tokio::test]
    async fn assert_allowed_rejects_self_message() {
        let social = MockSocialRepo::new();
        let err = assert_allowed("u1", "u1", &social)
            .await
            .expect_err("self should fail");
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn assert_allowed_rejects_blocked_relationship() {
        let mut social = MockSocialRepo::new();
        social
            .expect_is_blocked()
            .with(eq("u1"), eq("u2"))
            .times(1)
            .returning(|_, _| Ok(true));

        let err = assert_allowed("u1", "u2", &social)
            .await
            .expect_err("blocked user should fail");
        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn assert_allowed_requires_accepted_friendship() {
        let mut social = MockSocialRepo::new();
        social
            .expect_is_blocked()
            .times(2)
            .returning(|_, _| Ok(false));
        social
            .expect_are_friends()
            .with(eq("u1"), eq("u2"))
            .times(1)
            .returning(|_, _| Ok(false));

        let err = assert_allowed("u1", "u2", &social)
            .await
            .expect_err("nonfriend should fail");
        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn assert_allowed_accepts_friends_without_blocks() {
        let mut social = MockSocialRepo::new();
        social
            .expect_is_blocked()
            .times(2)
            .returning(|_, _| Ok(false));
        social
            .expect_are_friends()
            .with(eq("u1"), eq("u2"))
            .times(1)
            .returning(|_, _| Ok(true));

        assert!(assert_allowed("u1", "u2", &social).await.is_ok());
    }
}
