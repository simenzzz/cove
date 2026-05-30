use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<RecordId>,
    pub content: String,
    pub author: RecordId,
    pub channel: RecordId,
    pub created_at: Option<DateTime<Utc>>,
    pub edited_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAuthor {
    pub id: RecordId,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageWithAuthor {
    pub id: Option<RecordId>,
    pub content: String,
    pub author: MessageAuthor,
    pub channel: RecordId,
    pub created_at: Option<DateTime<Utc>>,
    pub edited_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessage {
    pub content: String,
}
