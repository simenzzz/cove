use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: Option<RecordId>,
    pub author: RecordId,
    pub title: String,
    /// Serialized Yjs state (full snapshot, not the diff log). Persisted as
    /// a base64 string for portability across SurrealDB encodings.
    pub state_b64: String,
    /// Latest Yjs state vector (base64).
    pub state_vector_b64: String,
    pub published: bool,
    /// Snapshot of the document text at publish time. None while drafting.
    pub published_content: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePost {
    pub title: String,
}
