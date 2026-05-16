use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::repositories::watch::WatchRepo;
use crate::ws::watch_room::spawn_watch_room;
use crate::ws::watch_types::WatchCommand;

/// Owns the set of live `WatchRoomActor` mpsc senders, keyed by channel id.
/// Mirrors `RoomManager` for chat rooms — lazy spawn on first subscribe,
/// removed when the actor reports it has gone idle.
#[derive(Clone)]
pub struct WatchRoomManager {
    rooms: Arc<RwLock<HashMap<String, mpsc::Sender<WatchCommand>>>>,
    watch_repo: Arc<dyn WatchRepo>,
}

impl WatchRoomManager {
    pub fn new(watch_repo: Arc<dyn WatchRepo>) -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            watch_repo,
        }
    }

    pub async fn get_or_create(&self, channel_id: &str) -> mpsc::Sender<WatchCommand> {
        {
            let rooms = self.rooms.read().await;
            if let Some(sender) = rooms.get(channel_id) {
                return sender.clone();
            }
        }

        let mut rooms = self.rooms.write().await;
        // Race-check after upgrading to write lock.
        if let Some(sender) = rooms.get(channel_id) {
            return sender.clone();
        }

        let sender = spawn_watch_room(
            channel_id.to_string(),
            self.clone(),
            self.watch_repo.clone(),
        );
        rooms.insert(channel_id.to_string(), sender.clone());
        sender
    }

    pub async fn get_room(&self, channel_id: &str) -> Option<mpsc::Sender<WatchCommand>> {
        let rooms = self.rooms.read().await;
        rooms.get(channel_id).cloned()
    }

    pub async fn remove(&self, channel_id: &str) {
        let mut rooms = self.rooms.write().await;
        rooms.remove(channel_id);
    }
}
