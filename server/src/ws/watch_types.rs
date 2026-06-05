//! Wire types and command enum for the watch room actor.
//!
//! Split out of `watch_room.rs` to keep that file under the project's 800-line
//! cap. The actor and its handlers live in `watch_room.rs`; this module owns
//! the value types they exchange over mpsc and serialize over WebSocket.

use serde::Serialize;
use tokio::sync::mpsc;

use crate::models::watch::QueueItem;

/// Commands the WatchRoomActor accepts over its mpsc inbox. The actor is the
/// single owner of room state — all mutations flow through these messages so
/// no external lock is required around queue / playback / leader.
pub enum WatchCommand {
    Join {
        user_id: String,
        username: String,
        sender: mpsc::Sender<String>,
    },
    Leave {
        user_id: String,
    },
    /// Explicit leader handoff. Validated by the actor: `from_user` must be
    /// the current leader and `to_user` must be currently connected. Replies
    /// to the requester via `reply_to` whether the transfer succeeded so the
    /// caller can surface a `watch_error` to the client.
    TransferLeader {
        from_user: String,
        to_user: String,
        reply_to: mpsc::Sender<String>,
    },
    /// Leader-only playback transition. `action` is one of `play | pause |
    /// seek | rate`. The actor checks the leader rule itself and re-stamps
    /// `server_ts` before broadcasting so all followers share one clock.
    PlaybackControl {
        from_user: String,
        action: String,
        position_ms: i64,
        rate: Option<f64>,
        reply_to: mpsc::Sender<String>,
    },
    /// Append to the queue. Anyone in the room can add. `nonce` echoes back
    /// in the ack so the client can flip its optimistic entry to confirmed.
    QueueAdd {
        from_user: String,
        video_id: String,
        title: String,
        duration_ms: i64,
        thumbnail_url: Option<String>,
        nonce: String,
        reply_to: mpsc::Sender<String>,
    },
    /// Remove a queue item. Permitted for the original adder OR the leader.
    QueueRemove {
        from_user: String,
        item_id: String,
        reply_to: mpsc::Sender<String>,
    },
    /// Cast / change / clear this user's vote on a queue item.
    /// `value` must be -1, 0, or 1; 0 removes any prior vote.
    Vote {
        from_user: String,
        item_id: String,
        value: i32,
        reply_to: mpsc::Sender<String>,
    },
    /// Leader-only. Advance to the next queue item (or stop if empty).
    Skip {
        from_user: String,
        reply_to: mpsc::Sender<String>,
    },
    /// Fire-and-forget floating emoji broadcast. Validated at the connection
    /// boundary; the actor just fans out.
    Reaction {
        from_user: String,
        username: String,
        emoji: String,
    },
    /// Leader-only progress heartbeat. Used by the server to:
    ///   (a) detect when the current video crosses the watch-completion
    ///       threshold (>=90% of duration) and record `watched` edges, and
    ///   (b) auto-advance when the leader's player has reached `duration_ms`.
    Progress {
        from_user: String,
        position_ms: i64,
    },
    /// Generic fan-out for messages composed outside the actor (REST handlers,
    /// admin events). Routed to every subscriber except `exclude_user`.
    Broadcast {
        message: String,
        exclude_user: Option<String>,
    },
}

#[derive(Clone, Serialize)]
pub struct ViewerSummary {
    pub user_id: String,
    pub username: String,
    pub is_leader: bool,
}

#[derive(Clone, Serialize)]
pub struct PlaybackSummary {
    pub video_id: Option<String>,
    pub position_ms: i64,
    pub paused: bool,
    pub server_ts: u64,
    pub rate: f64,
}

#[derive(Clone, Serialize)]
pub struct QueueItemSummary {
    pub id: String,
    pub video_id: String,
    pub title: String,
    pub duration_ms: i64,
    pub thumbnail_url: Option<String>,
    pub added_by: String,
    pub score: i32,
}

impl From<QueueItem> for QueueItemSummary {
    fn from(q: QueueItem) -> Self {
        Self {
            id: q
                .id
                .as_ref()
                .map(|r| r.key().to_string())
                .unwrap_or_default(),
            video_id: q.video_id,
            title: q.title,
            duration_ms: q.duration_ms,
            thumbnail_url: q.thumbnail_url,
            added_by: q.added_by.key().to_string(),
            score: q.score,
        }
    }
}
