//! Handlers for the text-chat WS message group: `Subscribe`, `Unsubscribe`,
//! `ChatMessage`, `Typing`, `Resume`. Extracted from `connection.rs` to keep
//! the connection loop readable; each function maps 1:1 to a former match arm,
//! so arm-level `continue` became `return` (loop-internal `continue`s inside
//! `Resume` are preserved).
//!
//! These operate on the connection's `subscriptions` / `last_typing` state,
//! passed in by reference from `handle_socket`.

use std::collections::HashMap;
use std::time::Instant;

use tokio::sync::mpsc;

use crate::models::channel::ChannelType;
use crate::models::direct::DirectMessageSummary;
use crate::ws::connection_helpers::check_channel_type_access;
use crate::ws::protocol::{MessageAuthor, ServerMessage, SubscriptionLevel};
use crate::ws::room::RoomCommand;
use crate::ws::{replay, sequence};
use crate::AppState;

/// `Subscribe`: authorize text-channel membership, join the room actor, and
/// record the subscription level.
pub async fn subscribe(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    username: &str,
    subscriptions: &mut HashMap<String, SubscriptionLevel>,
    channel_id: String,
    level: SubscriptionLevel,
) {
    if !can_access_chat_channel(state, &channel_id, user_id).await {
        let _ = out_tx
            .send(
                ServerMessage::Error {
                    message: "Not a chat channel or not a member".into(),
                }
                .to_json(),
            )
            .await;
        return;
    }

    let room = state.room_manager.get_or_create(&channel_id).await;
    let _ = room
        .send(RoomCommand::Join {
            user_id: user_id.to_string(),
            username: username.to_string(),
            level: level.clone(),
            sender: out_tx.clone(),
        })
        .await;
    subscriptions.insert(channel_id, level);
}

/// `Unsubscribe`: leave the room actor and drop the subscription.
pub async fn unsubscribe(
    state: &AppState,
    user_id: &str,
    subscriptions: &mut HashMap<String, SubscriptionLevel>,
    channel_id: String,
) {
    if let Some(room) = state.room_manager.get_room(&channel_id).await {
        let _ = room
            .send(RoomCommand::Leave {
                user_id: user_id.to_string(),
            })
            .await;
    }
    subscriptions.remove(&channel_id);
}

/// `ChatMessage`: validate, sequence, persist, ack the sender, and broadcast
/// to the room (excluding the sender).
#[allow(clippy::too_many_arguments)]
pub async fn chat_message(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    username: &str,
    display_name: &str,
    avatar_url: &Option<String>,
    subscriptions: &HashMap<String, SubscriptionLevel>,
    channel_id: String,
    content: String,
    nonce: String,
) {
    // Authorization: must be subscribed to the channel
    if !subscriptions.contains_key(&channel_id) {
        let _ = out_tx
            .send(
                ServerMessage::Error {
                    message: "Not subscribed to this channel".into(),
                }
                .to_json(),
            )
            .await;
        return;
    }

    if !can_access_chat_channel(state, &channel_id, user_id).await {
        let _ = out_tx
            .send(
                ServerMessage::Error {
                    message: "Not a chat channel or not a member".into(),
                }
                .to_json(),
            )
            .await;
        return;
    }

    // Validate content length
    if content.is_empty() || content.len() > 4000 {
        let _ = out_tx
            .send(
                ServerMessage::Error {
                    message: "Message must be 1-4000 characters".into(),
                }
                .to_json(),
            )
            .await;
        return;
    }

    let seq = match sequence::next_seq(&state.redis, &channel_id).await {
        Ok(s) => s,
        Err(_) => return,
    };

    let now_ms = chrono::Utc::now().timestamp_millis() as u64;
    let msg_id = uuid::Uuid::new_v4().to_string();

    let server_msg = ServerMessage::ChatMessage {
        seq,
        channel_id: channel_id.clone(),
        message_id: msg_id.clone(),
        author: MessageAuthor {
            id: user_id.to_string(),
            username: username.to_string(),
            display_name: display_name.to_string(),
            avatar_url: avatar_url.clone(),
        },
        content: content.clone(),
        ts: now_ms,
    };

    if let Err(e) = state
        .repos
        .messages
        .create_with_id(&msg_id, content.clone(), user_id, &channel_id)
        .await
    {
        tracing::error!(error = %e, "Failed to persist message to SurrealDB");
        let _ = out_tx
            .send(
                ServerMessage::Error {
                    message: "Failed to persist message".into(),
                }
                .to_json(),
            )
            .await;
        return;
    }

    let _ = replay::store_message(&state.redis, &channel_id, seq, &server_msg.to_json()).await;

    // ACK to sender
    let ack = ServerMessage::MessageAck {
        nonce,
        message_id: msg_id,
        seq,
        ts: now_ms,
    };
    let _ = out_tx.send(ack.to_json()).await;

    // Broadcast to room (excluding sender)
    if let Some(room) = state.room_manager.get_room(&channel_id).await {
        let _ = room
            .send(RoomCommand::Broadcast {
                message: server_msg.to_json(),
                exclude_user: Some(user_id.to_string()),
            })
            .await;
    }

    send_direct_updates_if_needed(
        state,
        &channel_id,
        &content,
        MessageAuthor {
            id: user_id.to_string(),
            username: username.to_string(),
            display_name: display_name.to_string(),
            avatar_url: avatar_url.clone(),
        },
        now_ms,
    )
    .await;
}

/// `Typing`: debounced (3s) typing indicator broadcast to the room.
pub async fn typing(
    state: &AppState,
    user_id: &str,
    username: &str,
    subscriptions: &HashMap<String, SubscriptionLevel>,
    last_typing: &mut HashMap<String, Instant>,
    channel_id: String,
) {
    if !subscriptions.contains_key(&channel_id) {
        return;
    }

    if !can_access_chat_channel(state, &channel_id, user_id).await {
        return;
    }

    let now = Instant::now();
    if let Some(last) = last_typing.get(&channel_id) {
        if now.duration_since(*last).as_secs() < 3 {
            return;
        }
    }
    last_typing.insert(channel_id.clone(), now);

    let typing_msg = ServerMessage::Typing {
        channel_id: channel_id.clone(),
        user_id: user_id.to_string(),
        username: username.to_string(),
    };
    if let Some(room) = state.room_manager.get_room(&channel_id).await {
        let _ = room
            .send(RoomCommand::Broadcast {
                message: typing_msg.to_json(),
                exclude_user: Some(user_id.to_string()),
            })
            .await;
    }
}

/// `Resume`: re-subscribe to each channel and replay missed messages (or send a
/// resync when the gap is too large). The per-channel `continue`s drive the
/// `for` loop, matching the original arm.
pub async fn resume(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    username: &str,
    subscriptions: &mut HashMap<String, SubscriptionLevel>,
    last_seq: HashMap<String, u64>,
) {
    for (channel_id, last) in last_seq {
        if !can_access_chat_channel(state, &channel_id, user_id).await {
            continue;
        }

        // Re-subscribe to the room
        let room = state.room_manager.get_or_create(&channel_id).await;
        let _ = room
            .send(RoomCommand::Join {
                user_id: user_id.to_string(),
                username: username.to_string(),
                level: SubscriptionLevel::Active,
                sender: out_tx.clone(),
            })
            .await;
        subscriptions.insert(channel_id.clone(), SubscriptionLevel::Active);

        // Replay missed messages
        match replay::get_missed_messages(&state.redis, &channel_id, last).await {
            Ok(Some(messages)) => {
                for msg in messages {
                    let _ = out_tx.send(msg).await;
                }
            }
            Ok(None) => {
                let resync = ServerMessage::Resync {
                    channel_id: channel_id.clone(),
                };
                let _ = out_tx.send(resync.to_json()).await;
            }
            Err(_) => {}
        }
    }
}

async fn can_access_chat_channel(state: &AppState, channel_id: &str, user_id: &str) -> bool {
    let channel = match state.repos.channels.find_by_id(channel_id).await {
        Ok(Some(channel)) => channel,
        _ => return false,
    };

    match channel.channel_type {
        ChannelType::Text => {
            check_channel_type_access(state, channel_id, user_id, ChannelType::Text).await
        }
        ChannelType::Direct => state
            .repos
            .direct_messages
            .can_access(channel_id, user_id, state.repos.social.as_ref())
            .await
            .unwrap_or(false),
        _ => false,
    }
}

async fn send_direct_updates_if_needed(
    state: &AppState,
    channel_id: &str,
    content: &str,
    from_user: MessageAuthor,
    ts: u64,
) {
    let Ok(Some(channel)) = state.repos.channels.find_by_id(channel_id).await else {
        return;
    };
    if channel.channel_type != ChannelType::Direct {
        return;
    }

    if let Err(e) = state
        .repos
        .direct_messages
        .mark_visible_for_members(channel_id)
        .await
    {
        tracing::warn!(error = %e, channel_id = %channel_id, "failed to mark direct message visible");
    }

    let sender_id = from_user.id.clone();
    let Ok(Some(peer)) = state
        .repos
        .direct_messages
        .peer_for_user(channel_id, &sender_id)
        .await
    else {
        return;
    };
    let Some(peer_id) = peer.id.as_ref().map(|id| id.key().to_string()) else {
        return;
    };

    let sender_user = state
        .repos
        .users
        .find_by_id(&sender_id)
        .await
        .ok()
        .flatten();

    let sender_dm = DirectMessageSummary {
        channel: channel.clone(),
        friend: peer,
    };
    let sender_payload = ServerMessage::DmChannelUpdated {
        dm: serde_json::to_value(sender_dm).unwrap_or(serde_json::Value::Null),
        last_message_preview: content.chars().take(120).collect(),
        from_user: from_user.clone(),
        ts,
    }
    .to_json();
    state
        .user_connections
        .send_to_user(&sender_id, sender_payload)
        .await;

    if let Some(sender_user) = sender_user {
        let peer_dm = DirectMessageSummary {
            channel,
            friend: sender_user,
        };
        let peer_payload = ServerMessage::DmChannelUpdated {
            dm: serde_json::to_value(peer_dm).unwrap_or(serde_json::Value::Null),
            last_message_preview: content.chars().take(120).collect(),
            from_user,
            ts,
        }
        .to_json();
        state
            .user_connections
            .send_to_user(&peer_id, peer_payload)
            .await;
    }
}
