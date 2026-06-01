//! Handlers for the watch-together WS message group. Extracted from
//! `connection.rs` (each function maps 1:1 to a former match arm; arm-level
//! `continue` became `return`). They operate on the connection's
//! `watch_subscriptions` set and route into the per-channel watch room actor.

use std::collections::HashSet;

use tokio::sync::mpsc;

use crate::middleware::rate_limit::{
    check_rate_limit, watch_playback_control_key, watch_reaction_key, RateLimitConfig,
};
use crate::ws::connection_helpers::{
    check_watch_channel_access, check_watch_queue_rate, send_watch_not_subscribed,
};
use crate::ws::protocol::ServerMessage;
use crate::ws::watch_types::WatchCommand;
use crate::AppState;

/// Watch-queue title hard cap. Enforced at the connection boundary so we fail
/// fast with a typed error instead of silently truncating in the room.
const MAX_WATCH_TITLE_LEN: usize = 200;

pub async fn subscribe(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    username: &str,
    watch_subscriptions: &mut HashSet<String>,
    channel_id: String,
) {
    // Defense in depth: verify the channel is actually a Watch channel AND the
    // user is a server member. The frontend routes by type, but never trust
    // the client.
    if !check_watch_channel_access(state, &channel_id, user_id).await {
        let _ = out_tx
            .send(
                ServerMessage::WatchError {
                    channel_id: channel_id.clone(),
                    code: "forbidden".into(),
                    message: "Not a watch channel or not a member".into(),
                }
                .to_json(),
            )
            .await;
        return;
    }
    let room = state.watch_manager.get_or_create(&channel_id).await;
    let _ = room
        .send(WatchCommand::Join {
            user_id: user_id.to_string(),
            username: username.to_string(),
            sender: out_tx.clone(),
        })
        .await;
    watch_subscriptions.insert(channel_id);
}

pub async fn unsubscribe(
    state: &AppState,
    user_id: &str,
    watch_subscriptions: &mut HashSet<String>,
    channel_id: String,
) {
    if let Some(room) = state.watch_manager.get_room(&channel_id).await {
        let _ = room
            .send(WatchCommand::Leave {
                user_id: user_id.to_string(),
            })
            .await;
    }
    watch_subscriptions.remove(&channel_id);
}

pub async fn transfer_leader(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    watch_subscriptions: &HashSet<String>,
    channel_id: String,
    to_user_id: String,
) {
    if !watch_subscriptions.contains(&channel_id) {
        send_watch_not_subscribed(out_tx, &channel_id).await;
        return;
    }
    // Transfer broadcasts to every viewer, so a leader ping-ponging leadership
    // is a fan-out amplifier. Reuse the queue-op bucket — it's already keyed
    // per user per room and gives 10/min, plenty for legitimate use.
    if !check_watch_queue_rate(state, user_id, &channel_id, out_tx).await {
        return;
    }
    if let Some(room) = state.watch_manager.get_room(&channel_id).await {
        let _ = room
            .send(WatchCommand::TransferLeader {
                from_user: user_id.to_string(),
                to_user: to_user_id,
                reply_to: out_tx.clone(),
            })
            .await;
    }
}

pub async fn playback(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    watch_subscriptions: &HashSet<String>,
    channel_id: String,
    action: String,
    position_ms: i64,
) {
    if !watch_subscriptions.contains(&channel_id) {
        send_watch_not_subscribed(out_tx, &channel_id).await;
        return;
    }
    // Rate cap: 10/sec per user per room. Leader is one user so this is a
    // per-leader cap. Defense against a runaway client looping seek events.
    let rate_key = watch_playback_control_key(user_id, &channel_id);
    if check_rate_limit(
        &state.redis,
        &RateLimitConfig {
            key_prefix: rate_key,
            limit: 10,
            window_secs: 1,
        },
    )
    .await
    .is_err()
    {
        let _ = out_tx
            .send(
                ServerMessage::WatchError {
                    channel_id,
                    code: "rate_limited".into(),
                    message: "Playback control rate limited".into(),
                }
                .to_json(),
            )
            .await;
        return;
    }
    if let Some(room) = state.watch_manager.get_room(&channel_id).await {
        let _ = room
            .send(WatchCommand::PlaybackControl {
                from_user: user_id.to_string(),
                action,
                position_ms,
                reply_to: out_tx.clone(),
            })
            .await;
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn queue_add(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    watch_subscriptions: &HashSet<String>,
    channel_id: String,
    video_id: String,
    title: String,
    duration_ms: i64,
    thumbnail_url: Option<String>,
    nonce: String,
) {
    if !watch_subscriptions.contains(&channel_id) {
        send_watch_not_subscribed(out_tx, &channel_id).await;
        return;
    }
    if title.chars().count() > MAX_WATCH_TITLE_LEN {
        let err = ServerMessage::WatchError {
            channel_id: channel_id.clone(),
            code: "TITLE_TOO_LONG".into(),
            message: format!("title exceeds {MAX_WATCH_TITLE_LEN} characters"),
        }
        .to_json();
        let _ = out_tx.send(err).await;
        return;
    }
    if !check_watch_queue_rate(state, user_id, &channel_id, out_tx).await {
        return;
    }
    if let Some(room) = state.watch_manager.get_room(&channel_id).await {
        let _ = room
            .send(WatchCommand::QueueAdd {
                from_user: user_id.to_string(),
                video_id,
                title,
                duration_ms,
                thumbnail_url,
                nonce,
                reply_to: out_tx.clone(),
            })
            .await;
    }
}

pub async fn queue_remove(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    watch_subscriptions: &HashSet<String>,
    channel_id: String,
    item_id: String,
) {
    if !watch_subscriptions.contains(&channel_id) {
        send_watch_not_subscribed(out_tx, &channel_id).await;
        return;
    }
    if !check_watch_queue_rate(state, user_id, &channel_id, out_tx).await {
        return;
    }
    if let Some(room) = state.watch_manager.get_room(&channel_id).await {
        let _ = room
            .send(WatchCommand::QueueRemove {
                from_user: user_id.to_string(),
                item_id,
                reply_to: out_tx.clone(),
            })
            .await;
    }
}

pub async fn vote(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    watch_subscriptions: &HashSet<String>,
    channel_id: String,
    item_id: String,
    value: i32,
) {
    if !watch_subscriptions.contains(&channel_id) {
        send_watch_not_subscribed(out_tx, &channel_id).await;
        return;
    }
    if !check_watch_queue_rate(state, user_id, &channel_id, out_tx).await {
        return;
    }
    if let Some(room) = state.watch_manager.get_room(&channel_id).await {
        let _ = room
            .send(WatchCommand::Vote {
                from_user: user_id.to_string(),
                item_id,
                value,
                reply_to: out_tx.clone(),
            })
            .await;
    }
}

pub async fn skip(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    watch_subscriptions: &HashSet<String>,
    channel_id: String,
) {
    if !watch_subscriptions.contains(&channel_id) {
        send_watch_not_subscribed(out_tx, &channel_id).await;
        return;
    }
    if !check_watch_queue_rate(state, user_id, &channel_id, out_tx).await {
        return;
    }
    if let Some(room) = state.watch_manager.get_room(&channel_id).await {
        let _ = room
            .send(WatchCommand::Skip {
                from_user: user_id.to_string(),
                reply_to: out_tx.clone(),
            })
            .await;
    }
}

pub async fn reaction(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    username: &str,
    watch_subscriptions: &HashSet<String>,
    channel_id: String,
    emoji: String,
) {
    if !watch_subscriptions.contains(&channel_id) {
        send_watch_not_subscribed(out_tx, &channel_id).await;
        return;
    }
    // Reject empty or oversized payloads — emojis can be multi-codepoint (e.g.
    // flag sequences) so we allow up to 32 bytes, plenty for any single emoji.
    let trimmed = emoji.trim();
    if trimmed.is_empty() || trimmed.len() > 32 {
        return;
    }
    let rate_key = watch_reaction_key(user_id, &channel_id);
    if check_rate_limit(
        &state.redis,
        &RateLimitConfig {
            key_prefix: rate_key,
            limit: 5,
            window_secs: 1,
        },
    )
    .await
    .is_err()
    {
        let _ = out_tx
            .send(
                ServerMessage::WatchError {
                    channel_id,
                    code: "rate_limited".into(),
                    message: "Reaction rate limited".into(),
                }
                .to_json(),
            )
            .await;
        return;
    }
    if let Some(room) = state.watch_manager.get_room(&channel_id).await {
        let _ = room
            .send(WatchCommand::Reaction {
                from_user: user_id.to_string(),
                username: username.to_string(),
                emoji: trimmed.to_string(),
            })
            .await;
    }
}

pub async fn progress(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    watch_subscriptions: &HashSet<String>,
    channel_id: String,
    position_ms: i64,
) {
    if !watch_subscriptions.contains(&channel_id) {
        send_watch_not_subscribed(out_tx, &channel_id).await;
        return;
    }
    // Defense-in-depth cap on the progress stream. The leader's client emits
    // ~once every 5s; a hostile or bugged leader spamming faster would still
    // be bounded by the actor's `current_recorded` one-shot gate, but
    // rate-limiting also protects the per-op `Progress` mailbox slot.
    let rate_key = watch_playback_control_key(user_id, &channel_id);
    if check_rate_limit(
        &state.redis,
        &RateLimitConfig {
            key_prefix: rate_key,
            limit: 10,
            window_secs: 1,
        },
    )
    .await
    .is_err()
    {
        return;
    }
    if let Some(room) = state.watch_manager.get_room(&channel_id).await {
        let _ = room
            .send(WatchCommand::Progress {
                from_user: user_id.to_string(),
                position_ms,
            })
            .await;
    }
}
