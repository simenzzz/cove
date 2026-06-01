use serde::{Deserialize, Serialize};

use crate::models::channel::Channel;
use crate::models::user::User;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessageSummary {
    pub channel: Channel,
    pub friend: User,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenDirectMessageInput {
    pub user_id: String,
}
