use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use crate::ws::voice_room::spawn_voice_room;
use crate::ws::voice_types::VoiceCommand;

#[derive(Clone)]
pub struct VoiceRoomManager {
    rooms: Arc<RwLock<HashMap<String, mpsc::Sender<VoiceCommand>>>>,
}

impl VoiceRoomManager {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_or_create(&self, channel_id: &str) -> mpsc::Sender<VoiceCommand> {
        {
            let rooms = self.rooms.read().await;
            if let Some(sender) = rooms.get(channel_id) {
                return sender.clone();
            }
        }

        let mut rooms = self.rooms.write().await;
        if let Some(sender) = rooms.get(channel_id) {
            return sender.clone();
        }

        let sender = spawn_voice_room(channel_id.to_string(), self.clone());
        rooms.insert(channel_id.to_string(), sender.clone());
        sender
    }

    pub async fn get_room(&self, channel_id: &str) -> Option<mpsc::Sender<VoiceCommand>> {
        let rooms = self.rooms.read().await;
        rooms.get(channel_id).cloned()
    }

    pub async fn remove(&self, channel_id: &str) {
        let mut rooms = self.rooms.write().await;
        rooms.remove(channel_id);
    }
}
