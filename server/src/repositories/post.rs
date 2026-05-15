use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;

use crate::error::AppError;
use crate::models::post::Post;

#[derive(Debug, Serialize)]
struct CreatePostDb {
    author: surrealdb::RecordId,
    title: String,
    state_b64: String,
    state_vector_b64: String,
    published: bool,
    published_content: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait PostRepo: Send + Sync {
    async fn create_draft(&self, author_id: &str, title: String) -> Result<Post, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Post>, AppError>;
    async fn save_snapshot(
        &self,
        id: &str,
        state_b64: String,
        state_vector_b64: String,
    ) -> Result<(), AppError>;
    async fn publish(&self, id: &str, content: String) -> Result<Post, AppError>;
}

pub struct SurrealPostRepo {
    db: Surreal<Client>,
}

impl SurrealPostRepo {
    pub fn new(db: Surreal<Client>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl PostRepo for SurrealPostRepo {
    async fn create_draft(&self, author_id: &str, title: String) -> Result<Post, AppError> {
        let now = chrono::Utc::now();
        let record: Option<Post> = self
            .db
            .create("post")
            .content(CreatePostDb {
                author: surrealdb::RecordId::from(("user", author_id)),
                title,
                state_b64: String::new(),
                state_vector_b64: String::new(),
                published: false,
                published_content: None,
                created_at: now,
                updated_at: now,
            })
            .await?;
        record.ok_or_else(|| AppError::Internal("Failed to create post draft".into()))
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Post>, AppError> {
        let post: Option<Post> = self.db.select(("post", id)).await?;
        Ok(post)
    }

    async fn save_snapshot(
        &self,
        id: &str,
        state_b64: String,
        state_vector_b64: String,
    ) -> Result<(), AppError> {
        self.db
            .query(
                "UPDATE $id SET state_b64 = $state, state_vector_b64 = $sv, updated_at = time::now()",
            )
            .bind(("id", surrealdb::RecordId::from(("post", id))))
            .bind(("state", state_b64))
            .bind(("sv", state_vector_b64))
            .await?;
        Ok(())
    }

    async fn publish(&self, id: &str, content: String) -> Result<Post, AppError> {
        let mut result = self
            .db
            .query(
                "UPDATE $id SET published = true, published_content = $content, \
                 updated_at = time::now() RETURN AFTER",
            )
            .bind(("id", surrealdb::RecordId::from(("post", id))))
            .bind(("content", content))
            .await?;
        let updated: Vec<Post> = result.take(0)?;
        updated
            .into_iter()
            .next()
            .ok_or_else(|| AppError::NotFound("Post not found".into()))
    }
}
