use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::models::user::User;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Auth {
        ticket: String,
        nonce: String,
    },
    Subscribe {
        channel_id: String,
        level: SubscriptionLevel,
    },
    Unsubscribe {
        channel_id: String,
    },
    ChatMessage {
        channel_id: String,
        content: String,
        nonce: String,
    },
    Typing {
        channel_id: String,
    },
    Resume {
        last_seq: HashMap<String, u64>,
    },
    Heartbeat,
    // ── Phase 2: collaborative editing ──
    CollabSubscribe {
        post_id: String,
    },
    CollabUnsubscribe {
        post_id: String,
    },
    CollabUpdate {
        post_id: String,
        /// Base64-encoded Yjs update bytes.
        update_b64: String,
    },
    AwarenessUpdate {
        post_id: String,
        /// Opaque awareness state (cursor pos, selection, idle ts) — passed
        /// through unchanged to other subscribers.
        state: serde_json::Value,
    },
    // ── Phase 3: shared whiteboard (CRDT canvas) ──
    WhiteboardSubscribe {
        whiteboard_id: String,
    },
    WhiteboardUnsubscribe {
        whiteboard_id: String,
    },
    WhiteboardUpdate {
        whiteboard_id: String,
        update_b64: String,
    },
    WhiteboardAwarenessUpdate {
        whiteboard_id: String,
        state: serde_json::Value,
    },
    // ── Phase 4: watch-together rooms ──
    WatchSubscribe {
        channel_id: String,
    },
    WatchUnsubscribe {
        channel_id: String,
    },
    /// Leader-only. Hand off leadership to another currently-connected member.
    WatchTransferLeader {
        channel_id: String,
        to_user_id: String,
    },
    /// Leader-only. `action` is one of "play" | "pause" | "seek". `client_ts`
    /// is the leader's local timestamp at emit; the server replaces it with
    /// its own `server_ts` before broadcast.
    WatchPlayback {
        channel_id: String,
        action: String,
        position_ms: i64,
        client_ts: u64,
    },
    WatchQueueAdd {
        channel_id: String,
        video_id: String,
        title: String,
        duration_ms: i64,
        thumbnail_url: Option<String>,
        nonce: String,
    },
    WatchQueueRemove {
        channel_id: String,
        item_id: String,
    },
    WatchVote {
        channel_id: String,
        item_id: String,
        /// -1, 0, or 1. 0 clears any prior vote.
        value: i32,
    },
    /// Leader-only. Advances queue to next item.
    WatchSkip {
        channel_id: String,
    },
    WatchReaction {
        channel_id: String,
        emoji: String,
    },
    /// Leader-only. Report the current playback position so the server can
    /// detect when a video ends naturally (>=90% or end-of-stream) and the
    /// `watched` edge should be written. Sent every few seconds.
    WatchProgress {
        channel_id: String,
        position_ms: i64,
    },
    // ── Server/channel lifecycle ──
    ChannelDocSubscribe {
        channel_id: String,
    },
    ChannelDocUnsubscribe {
        channel_id: String,
    },
    ChannelDocUpdate {
        channel_id: String,
        update_b64: String,
    },
    ChannelDocAwarenessUpdate {
        channel_id: String,
        state: serde_json::Value,
    },
    // ── Voice signaling (WebRTC mesh) ──
    VoiceJoin {
        channel_id: String,
    },
    VoiceLeave {
        channel_id: String,
    },
    VoiceSignal {
        channel_id: String,
        to_user_id: String,
        signal: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionLevel {
    Active,
    Badge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    AuthOk {
        user_id: String,
        heartbeat_interval: u64,
    },
    ChatMessage {
        seq: u64,
        channel_id: String,
        message_id: String,
        author: MessageAuthor,
        content: String,
        ts: u64,
    },
    MessageAck {
        nonce: String,
        message_id: String,
        seq: u64,
        ts: u64,
    },
    Typing {
        channel_id: String,
        user_id: String,
        username: String,
    },
    Presence {
        user_id: String,
        status: String,
    },
    Unread {
        channel_id: String,
        count: u64,
        last_message_preview: String,
    },
    DmChannelUpdated {
        dm: serde_json::Value,
        last_message_preview: String,
        from_user: MessageAuthor,
        ts: u64,
    },
    Resync {
        channel_id: String,
    },
    HeartbeatAck,
    Error {
        message: String,
    },
    // ── Phase 2: collaborative editing ──
    CollabState {
        post_id: String,
        /// Base64-encoded full Yjs state (sent on subscribe so the client can
        /// hydrate its local Y.Doc without paying for replay history).
        state_b64: String,
        /// Base64-encoded state vector (so client can request a diff later).
        state_vector_b64: String,
    },
    CollabUpdate {
        post_id: String,
        update_b64: String,
        from_user: String,
    },
    AwarenessState {
        post_id: String,
        /// `user_id -> opaque state`
        users: HashMap<String, serde_json::Value>,
    },
    CollabError {
        post_id: String,
        code: String,
        message: String,
    },
    /// Sent when the server tears down a collab session — currently fires on
    /// publish so editors can flip to a read-only view.
    CollabClosed {
        post_id: String,
        reason: String,
    },
    // ── Phase 3: shared whiteboard ──
    WhiteboardState {
        whiteboard_id: String,
        state_b64: String,
        state_vector_b64: String,
    },
    WhiteboardUpdate {
        whiteboard_id: String,
        update_b64: String,
        from_user: String,
    },
    WhiteboardAwarenessState {
        whiteboard_id: String,
        users: HashMap<String, serde_json::Value>,
    },
    WhiteboardError {
        whiteboard_id: String,
        code: String,
        message: String,
    },
    /// Sent when the server tears down a whiteboard session (e.g., checkpoint
    /// restore). Clients should re-subscribe to fetch fresh state.
    WhiteboardClosed {
        whiteboard_id: String,
        reason: String,
    },
    // ── Phase 4: watch-together rooms ──
    /// Full room snapshot — sent on subscribe and after viewer/leader/queue
    /// transitions so clients can re-hydrate without a separate REST call.
    /// `playback`, `queue`, and `viewers` are passed as opaque JSON values so
    /// the actor can shape them without bloating the protocol enum surface.
    WatchState {
        channel_id: String,
        leader_id: Option<String>,
        playback: serde_json::Value,
        queue: serde_json::Value,
        viewers: serde_json::Value,
    },
    /// Leader-stamped playback transition. Followers apply immediately with
    /// latency compensation `position_ms + (now - server_ts) * rate`.
    WatchPlayback {
        channel_id: String,
        action: String,
        position_ms: i64,
        server_ts: u64,
        by_user: String,
    },
    /// Periodic authoritative playback heartbeat (5s while not paused) so
    /// followers can correct drift without waiting for a transition.
    WatchSyncPulse {
        channel_id: String,
        position_ms: i64,
        server_ts: u64,
        paused: bool,
    },
    /// Diff broadcast after add/remove/score-reorder. Clients reconcile their
    /// local optimistic state against this.
    WatchQueueUpdate {
        channel_id: String,
        queue: serde_json::Value,
    },
    /// Optimistic-add acknowledgment back to the sender — matches the request
    /// nonce so the client can flip its pending entry to confirmed.
    WatchQueueAck {
        channel_id: String,
        nonce: String,
        item_id: String,
    },
    /// Sent when the leader skips or a video ends server-side; carries the
    /// new playback state plus the queue (the front item is removed).
    WatchAdvance {
        channel_id: String,
        playback: serde_json::Value,
        queue: serde_json::Value,
    },
    /// Floating emoji reaction. Echoed back to the sender too so the UI
    /// renders identically across all viewers.
    WatchReaction {
        channel_id: String,
        user_id: String,
        username: String,
        emoji: String,
        ts: u64,
    },
    WatchLeaderChanged {
        channel_id: String,
        leader_id: String,
        /// "transfer" | "disconnect"
        reason: String,
    },
    WatchError {
        channel_id: String,
        code: String,
        message: String,
    },
    /// Sent when the channel is deleted or the server forcibly tears the
    /// session down. Clients should drop their local state.
    WatchClosed {
        channel_id: String,
        reason: String,
    },
    ServerJoined {
        server: serde_json::Value,
    },
    ChannelCreated {
        server_id: String,
        channel: serde_json::Value,
    },
    FriendRequestReceived {
        from_user: NotificationUser,
        ts: u64,
    },
    FriendRequestAccepted {
        user: NotificationUser,
        ts: u64,
    },
    ServerMemberJoined {
        server_id: String,
        user: NotificationUser,
        ts: u64,
    },
    ServerMemberLeft {
        server_id: String,
        user: NotificationUser,
        ts: u64,
    },
    ChannelDocState {
        channel_id: String,
        state_b64: String,
        state_vector_b64: String,
    },
    ChannelDocUpdate {
        channel_id: String,
        update_b64: String,
        from_user: String,
    },
    ChannelDocAwarenessState {
        channel_id: String,
        users: HashMap<String, serde_json::Value>,
    },
    ChannelDocError {
        channel_id: String,
        code: String,
        message: String,
    },
    ChannelDocClosed {
        channel_id: String,
        reason: String,
    },
    VoiceState {
        channel_id: String,
        participants: serde_json::Value,
    },
    VoiceUserJoined {
        channel_id: String,
        user_id: String,
        username: String,
    },
    VoiceUserLeft {
        channel_id: String,
        user_id: String,
    },
    VoiceSignal {
        channel_id: String,
        from_user_id: String,
        signal: serde_json::Value,
    },
    VoiceError {
        channel_id: String,
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAuthor {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationUser {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

impl From<&User> for NotificationUser {
    fn from(user: &User) -> Self {
        Self {
            id: user
                .id
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            username: user.username.clone(),
            display_name: user.display_name.clone(),
            avatar_url: user.avatar_url.clone(),
        }
    }
}

impl ServerMessage {
    pub fn to_json(&self) -> String {
        match serde_json::to_value(self) {
            Ok(mut value) => {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("v".to_string(), serde_json::json!(1));
                }
                serde_json::to_string(&value).unwrap_or_else(|e| {
                    tracing::error!(error = %e, "Failed to serialize ServerMessage");
                    r#"{"type":"error","message":"serialization failure","v":1}"#.to_string()
                })
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to convert ServerMessage to value");
                r#"{"type":"error","message":"serialization failure","v":1}"#.to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::UserStatus;

    #[test]
    fn notification_user_omits_email() {
        let user = User {
            id: Some(surrealdb::RecordId::from(("user", "u1"))),
            email: "private@test.example.com".into(),
            username: "alice".into(),
            display_name: "Alice".into(),
            avatar_url: None,
            status: UserStatus::Online,
            created_at: None,
        };

        let value = serde_json::to_value(NotificationUser::from(&user)).expect("serialize");
        assert!(value.get("email").is_none());
        assert_eq!(
            value.get("username").and_then(serde_json::Value::as_str),
            Some("alice")
        );
    }
}
