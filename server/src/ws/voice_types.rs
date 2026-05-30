use serde::Serialize;
use tokio::sync::mpsc;

#[derive(Clone, Serialize)]
pub struct VoiceParticipant {
    pub user_id: String,
    pub username: String,
}

pub enum VoiceCommand {
    Join {
        user_id: String,
        username: String,
        sender: mpsc::Sender<String>,
    },
    Leave {
        user_id: String,
    },
    Signal {
        from_user: String,
        to_user: String,
        signal: serde_json::Value,
        reply_to: mpsc::Sender<String>,
    },
}
