use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::ws::protocol::ServerMessage;
use crate::ws::voice_room_manager::VoiceRoomManager;
use crate::ws::voice_types::{VoiceCommand, VoiceParticipant};

const GRACE_PERIOD: std::time::Duration = std::time::Duration::from_secs(30);
const MAX_PARTICIPANTS: usize = 8;

struct Subscriber {
    username: String,
    sender: mpsc::Sender<String>,
}

struct VoiceRoom {
    channel_id: String,
    clients: HashMap<String, Subscriber>,
    manager: VoiceRoomManager,
}

impl VoiceRoom {
    fn new(channel_id: String, manager: VoiceRoomManager) -> Self {
        Self {
            channel_id,
            clients: HashMap::new(),
            manager,
        }
    }

    fn participants(&self) -> Vec<VoiceParticipant> {
        self.clients
            .iter()
            .map(|(user_id, sub)| VoiceParticipant {
                user_id: user_id.clone(),
                username: sub.username.clone(),
            })
            .collect()
    }

    fn broadcast(&mut self, payload: String, exclude_user: Option<&str>) {
        let dead: Vec<String> = self
            .clients
            .iter()
            .filter(|(id, _)| exclude_user != Some(id.as_str()))
            .filter_map(|(id, sub)| match sub.sender.try_send(payload.clone()) {
                Ok(()) => None,
                Err(mpsc::error::TrySendError::Closed(_)) => Some(id.clone()),
                Err(mpsc::error::TrySendError::Full(_)) => None,
            })
            .collect();
        for id in dead {
            self.clients.remove(&id);
        }
    }

    fn send_error(tx: &mpsc::Sender<String>, channel_id: &str, code: &str, message: &str) {
        let _ = tx.try_send(
            ServerMessage::VoiceError {
                channel_id: channel_id.to_string(),
                code: code.to_string(),
                message: message.to_string(),
            }
            .to_json(),
        );
    }

    fn handle(&mut self, cmd: VoiceCommand) {
        match cmd {
            VoiceCommand::Join {
                user_id,
                username,
                sender,
            } => {
                if !self.clients.contains_key(&user_id) && self.clients.len() >= MAX_PARTICIPANTS {
                    Self::send_error(&sender, &self.channel_id, "room_full", "Voice room is full");
                    return;
                }

                self.clients.insert(
                    user_id.clone(),
                    Subscriber {
                        username: username.clone(),
                        sender: sender.clone(),
                    },
                );

                let state = ServerMessage::VoiceState {
                    channel_id: self.channel_id.clone(),
                    participants: serde_json::to_value(self.participants())
                        .unwrap_or(serde_json::Value::Array(vec![])),
                }
                .to_json();
                let _ = sender.try_send(state);

                let joined = ServerMessage::VoiceUserJoined {
                    channel_id: self.channel_id.clone(),
                    user_id: user_id.clone(),
                    username,
                }
                .to_json();
                self.broadcast(joined, Some(&user_id));
            }
            VoiceCommand::Leave { user_id } => {
                if self.clients.remove(&user_id).is_some() {
                    let left = ServerMessage::VoiceUserLeft {
                        channel_id: self.channel_id.clone(),
                        user_id: user_id.clone(),
                    }
                    .to_json();
                    self.broadcast(left, Some(&user_id));
                }
            }
            VoiceCommand::Signal {
                from_user,
                to_user,
                signal,
                reply_to,
            } => {
                if !self.clients.contains_key(&from_user) {
                    Self::send_error(
                        &reply_to,
                        &self.channel_id,
                        "not_joined",
                        "Not in voice room",
                    );
                    return;
                }
                let Some(target) = self.clients.get(&to_user) else {
                    Self::send_error(
                        &reply_to,
                        &self.channel_id,
                        "target_missing",
                        "Target is not in voice room",
                    );
                    return;
                };
                let _ = target.sender.try_send(
                    ServerMessage::VoiceSignal {
                        channel_id: self.channel_id.clone(),
                        from_user_id: from_user,
                        signal,
                    }
                    .to_json(),
                );
            }
        }
    }
}

pub fn spawn_voice_room(
    channel_id: String,
    manager: VoiceRoomManager,
) -> mpsc::Sender<VoiceCommand> {
    let (tx, mut rx) = mpsc::channel::<VoiceCommand>(256);

    tokio::spawn(async move {
        let mut room = VoiceRoom::new(channel_id.clone(), manager);
        while let Some(cmd) = rx.recv().await {
            room.handle(cmd);
            if room.clients.is_empty() {
                match tokio::time::timeout(GRACE_PERIOD, rx.recv()).await {
                    Ok(Some(cmd)) => {
                        room.handle(cmd);
                        continue;
                    }
                    _ => {
                        room.manager.remove(&channel_id).await;
                        break;
                    }
                }
            }
        }
    });

    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn forwards_signal_only_to_joined_target() {
        let manager = VoiceRoomManager::new();
        let mut room = VoiceRoom::new("c1".into(), manager);
        let (tx_a, _rx_a) = mpsc::channel(8);
        let (tx_b, mut rx_b) = mpsc::channel(8);
        let (reply, _rx_reply) = mpsc::channel(8);
        room.handle(VoiceCommand::Join {
            user_id: "u1".into(),
            username: "a".into(),
            sender: tx_a,
        });
        room.handle(VoiceCommand::Join {
            user_id: "u2".into(),
            username: "b".into(),
            sender: tx_b,
        });
        room.handle(VoiceCommand::Signal {
            from_user: "u1".into(),
            to_user: "u2".into(),
            signal: serde_json::json!({"type":"offer"}),
            reply_to: reply,
        });
        let mut forwarded = rx_b.recv().await.expect("signal");
        while !forwarded.contains("voice_signal") {
            forwarded = rx_b.recv().await.expect("signal");
        }
        assert!(forwarded.contains("voice_signal"));
        assert!(forwarded.contains("u1"));
    }
}
