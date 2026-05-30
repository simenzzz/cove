use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

use crate::collab::resource::{ResourceRef, ResourceStore, Snapshot};
use crate::models::channel::ChannelType;
use crate::repositories::{channel::ChannelRepo, server::ServerRepo};

use std::sync::Arc;

#[derive(Debug, Serialize)]
struct ChannelDocDb {
    channel: surrealdb::RecordId,
    state_b64: String,
    state_vector_b64: String,
    updated_at: surrealdb::Datetime,
}

#[derive(Debug, Deserialize)]
struct ChannelDocRow {
    state_b64: Option<String>,
    state_vector_b64: Option<String>,
}

pub struct ChannelDocStore {
    db: Surreal<Client>,
    channels: Arc<dyn ChannelRepo>,
    servers: Arc<dyn ServerRepo>,
}

impl ChannelDocStore {
    pub fn new(
        db: Surreal<Client>,
        channels: Arc<dyn ChannelRepo>,
        servers: Arc<dyn ServerRepo>,
    ) -> Self {
        Self {
            db,
            channels,
            servers,
        }
    }
}

#[async_trait]
impl ResourceStore for ChannelDocStore {
    async fn load(&self, r: &ResourceRef) -> Result<Snapshot, String> {
        let row: Option<ChannelDocRow> = self
            .db
            .select(("channel_doc", r.id.as_str()))
            .await
            .map_err(|e| e.to_string())?;
        Ok(row
            .map(|row| Snapshot {
                state_b64: row.state_b64.unwrap_or_default(),
                state_vector_b64: row.state_vector_b64.unwrap_or_default(),
            })
            .unwrap_or_else(Snapshot::empty))
    }

    async fn save(&self, r: &ResourceRef, snap: Snapshot) -> Result<(), String> {
        let now = surrealdb::Datetime::from(chrono::Utc::now());
        let saved: Option<ChannelDocRow> = self
            .db
            .upsert(("channel_doc", r.id.as_str()))
            .content(ChannelDocDb {
                channel: surrealdb::RecordId::from(("channel", r.id.as_str())),
                state_b64: snap.state_b64,
                state_vector_b64: snap.state_vector_b64,
                updated_at: now,
            })
            .await
            .map_err(|e| e.to_string())?;
        saved
            .map(|_| ())
            .ok_or_else(|| "Failed to save channel document".to_string())
    }

    async fn authorize(&self, r: &ResourceRef, user_id: &str) -> Result<(), String> {
        let channel = self
            .channels
            .find_by_id(&r.id)
            .await
            .map_err(|_| "Not authorized for this channel document".to_string())?
            .ok_or_else(|| "Not authorized for this channel document".to_string())?;

        if channel.channel_type != ChannelType::Collab {
            return Err("Not authorized for this channel document".to_string());
        }

        let server_id = channel.server.key().to_string();
        match self.servers.is_member(&server_id, user_id).await {
            Ok(true) => Ok(()),
            _ => Err("Not authorized for this channel document".to_string()),
        }
    }
}
