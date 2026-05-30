use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use serde::Serialize;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

use crate::error::AppError;
use crate::models::message::{Message, MessageWithAuthor};

#[derive(Debug, Serialize)]
pub struct CreateMessageDb {
    pub content: String,
    pub author: surrealdb::RecordId,
    pub channel: surrealdb::RecordId,
    pub created_at: surrealdb::Datetime,
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait MessageRepo: Send + Sync {
    async fn create(
        &self,
        content: String,
        author_id: &str,
        channel_id: &str,
    ) -> Result<Message, AppError>;
    async fn create_with_id(
        &self,
        id: &str,
        content: String,
        author_id: &str,
        channel_id: &str,
    ) -> Result<Message, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Message>, AppError>;
    async fn list_for_channel(
        &self,
        channel_id: &str,
        before: Option<String>,
        limit: u32,
    ) -> Result<Vec<MessageWithAuthor>, AppError>;
}

pub struct SurrealMessageRepo {
    db: Surreal<Client>,
}

impl SurrealMessageRepo {
    pub fn new(db: Surreal<Client>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MessageRepo for SurrealMessageRepo {
    async fn create(
        &self,
        content: String,
        author_id: &str,
        channel_id: &str,
    ) -> Result<Message, AppError> {
        let record: Option<Message> = self
            .db
            .create("message")
            .content(CreateMessageDb {
                content,
                author: surrealdb::RecordId::from(("user", author_id)),
                channel: surrealdb::RecordId::from(("channel", channel_id)),
                created_at: chrono::Utc::now().into(),
            })
            .await?;
        record.ok_or_else(|| AppError::Internal("Failed to create message".into()))
    }

    async fn create_with_id(
        &self,
        id: &str,
        content: String,
        author_id: &str,
        channel_id: &str,
    ) -> Result<Message, AppError> {
        let record: Option<Message> = self
            .db
            .create(("message", id))
            .content(CreateMessageDb {
                content,
                author: surrealdb::RecordId::from(("user", author_id)),
                channel: surrealdb::RecordId::from(("channel", channel_id)),
                created_at: chrono::Utc::now().into(),
            })
            .await?;
        record.ok_or_else(|| AppError::Internal("Failed to create message".into()))
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Message>, AppError> {
        let message: Option<Message> = self.db.select(("message", id)).await?;
        Ok(message)
    }

    async fn list_for_channel(
        &self,
        channel_id: &str,
        before: Option<String>,
        limit: u32,
    ) -> Result<Vec<MessageWithAuthor>, AppError> {
        let query = match before {
            Some(_) => {
                "SELECT id, content, channel, created_at, edited_at, \
                 { id: author, username: author.username, display_name: author.display_name, avatar_url: author.avatar_url } AS author \
                 FROM message WHERE channel = $channel \
                 AND created_at < (SELECT created_at FROM message WHERE id = $before LIMIT 1) \
                 ORDER BY created_at DESC LIMIT $limit"
            }
            None => {
                "SELECT id, content, channel, created_at, edited_at, \
                 { id: author, username: author.username, display_name: author.display_name, avatar_url: author.avatar_url } AS author \
                 FROM message WHERE channel = $channel \
                 ORDER BY created_at DESC LIMIT $limit"
            }
        };

        let mut q = self
            .db
            .query(query)
            .bind((
                "channel",
                surrealdb::RecordId::from(("channel", channel_id)),
            ))
            .bind(("limit", limit));

        if let Some(before_id) = before {
            q = q.bind((
                "before",
                surrealdb::RecordId::from(("message", before_id.as_str())),
            ));
        }

        let mut result = q.await?;
        let messages: Vec<MessageWithAuthor> = result.take(0)?;
        Ok(messages)
    }
}
