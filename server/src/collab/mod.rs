pub mod awareness;
pub mod doc;

use std::sync::Arc;

use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};

use crate::repositories::post::PostRepo;
use crate::ws::protocol::{ClientMessage, ServerMessage};

use awareness::Awareness;
use doc::CollabDoc;

/// Per-document collab state. One session per `post_id` while it has active
/// subscribers; evicted lazily on unsubscribe (kept simple — no timer for
/// kickoff scope).
struct Session {
    doc: CollabDoc,
    awareness: Awareness,
    subscribers: Vec<Subscriber>,
}

struct Subscriber {
    user_id: String,
    tx: mpsc::Sender<String>,
}

/// In-memory CRDT manager. Holds active sessions and dispatches WS messages
/// arriving from `ws::connection`. Persistence is best-effort and synchronous
/// for kickoff scope — debouncing is a future optimization.
#[derive(Clone)]
pub struct CollabManager {
    sessions: Arc<DashMap<String, Arc<Mutex<Session>>>>,
    posts: Arc<dyn PostRepo>,
}

impl CollabManager {
    pub fn new(posts: Arc<dyn PostRepo>) -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            posts,
        }
    }

    /// Entry point called from `ws::connection`. Routes the four collab-
    /// related `ClientMessage` variants. Other variants are a no-op.
    pub async fn dispatch(
        &self,
        post_id: String,
        msg: ClientMessage,
        user_id: &str,
        out_tx: mpsc::Sender<String>,
    ) {
        match msg {
            ClientMessage::CollabSubscribe { .. } => {
                if let Err(err) = self.subscribe(&post_id, user_id, out_tx).await {
                    let _ = send_error(&self.error_tx(user_id), &post_id, &err).await;
                }
            }
            ClientMessage::CollabUnsubscribe { .. } => {
                self.unsubscribe(&post_id, user_id).await;
            }
            ClientMessage::CollabUpdate { update_b64, .. } => {
                if let Err(err) = self
                    .apply_update(&post_id, user_id, &update_b64)
                    .await
                {
                    let send_to = self.subscriber_tx(&post_id, user_id).await.unwrap_or(out_tx);
                    let _ = send_error(&send_to, &post_id, &err).await;
                }
            }
            ClientMessage::AwarenessUpdate { state, .. } => {
                self.update_awareness(&post_id, user_id, state).await;
            }
            _ => {}
        }
    }

    async fn subscribe(
        &self,
        post_id: &str,
        user_id: &str,
        tx: mpsc::Sender<String>,
    ) -> Result<(), String> {
        // Load existing snapshot from storage on first subscribe.
        let session = match self.sessions.get(post_id) {
            Some(s) => s.clone(),
            None => {
                let post = self
                    .posts
                    .find_by_id(post_id)
                    .await
                    .map_err(|e| format!("Failed to load post: {e}"))?
                    .ok_or_else(|| "Post not found".to_string())?;
                let doc = CollabDoc::from_snapshot(&post.state_b64)
                    .map_err(|e| format!("Bad snapshot: {e}"))?;
                let s = Arc::new(Mutex::new(Session {
                    doc,
                    awareness: Awareness::new(),
                    subscribers: Vec::new(),
                }));
                self.sessions.insert(post_id.to_string(), s.clone());
                s
            }
        };

        let mut s = session.lock().await;

        // Send the full state to the new subscriber so it can hydrate.
        let state_msg = ServerMessage::CollabState {
            post_id: post_id.to_string(),
            state_b64: s.doc.encode_state(),
            state_vector_b64: s.doc.encode_state_vector(),
        }
        .to_json();
        let _ = tx.send(state_msg).await;

        // Send the current awareness snapshot.
        if !s.awareness.is_empty() {
            let aw_msg = ServerMessage::AwarenessState {
                post_id: post_id.to_string(),
                users: s.awareness.snapshot(),
            }
            .to_json();
            let _ = tx.send(aw_msg).await;
        }

        s.subscribers.push(Subscriber {
            user_id: user_id.to_string(),
            tx,
        });
        Ok(())
    }

    async fn unsubscribe(&self, post_id: &str, user_id: &str) {
        if let Some(session) = self.sessions.get(post_id) {
            let session = session.clone();
            let mut s = session.lock().await;
            s.subscribers.retain(|sub| sub.user_id != user_id);
            s.awareness.remove(user_id);

            // Tell remaining subscribers the awareness set changed.
            broadcast(
                &s,
                None,
                &ServerMessage::AwarenessState {
                    post_id: post_id.to_string(),
                    users: s.awareness.snapshot(),
                }
                .to_json(),
            )
            .await;

            // Evict empty session.
            if s.subscribers.is_empty() {
                drop(s);
                self.sessions.remove(post_id);
            }
        }
    }

    async fn apply_update(
        &self,
        post_id: &str,
        from_user: &str,
        update_b64: &str,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get(post_id)
            .ok_or_else(|| "Not subscribed".to_string())?
            .clone();
        let mut s = session.lock().await;
        s.doc
            .apply_update(update_b64)
            .map_err(|e| format!("Apply failed: {e}"))?;

        // Fan out to other subscribers.
        let payload = ServerMessage::CollabUpdate {
            post_id: post_id.to_string(),
            update_b64: update_b64.to_string(),
            from_user: from_user.to_string(),
        }
        .to_json();
        broadcast(&s, Some(from_user), &payload).await;

        // Persist the latest full state. Kickoff scope: synchronous on every
        // update. Risk #5 (architectural-risks.md) calls out debouncing as the
        // followup — `PERSIST_DEBOUNCE = 2s` is the planned target.
        let state_b64 = s.doc.encode_state();
        let sv_b64 = s.doc.encode_state_vector();
        let posts = self.posts.clone();
        let post_id_owned = post_id.to_string();
        tokio::spawn(async move {
            if let Err(e) = posts.save_snapshot(&post_id_owned, state_b64, sv_b64).await {
                tracing::warn!(post_id = %post_id_owned, error = %e, "Snapshot persist failed");
            }
        });
        Ok(())
    }

    async fn update_awareness(&self, post_id: &str, user_id: &str, state: Value) {
        if let Some(session) = self.sessions.get(post_id) {
            let session = session.clone();
            let mut session_guard = session.lock().await;
            session_guard.awareness.update(user_id.to_string(), state);

            let payload = ServerMessage::AwarenessState {
                post_id: post_id.to_string(),
                users: session_guard.awareness.snapshot(),
            }
            .to_json();
            broadcast(&session_guard, None, &payload).await;
        }
    }

    /// Helper for error routing when we don't already have a subscriber tx.
    fn error_tx(&self, _user_id: &str) -> mpsc::Sender<String> {
        // Construct a no-op sender — errors during subscribe (before we have
        // a registered subscriber) get dropped if we can't reach the user.
        // The connection layer already passes its own `out_tx` for that case.
        let (tx, _rx) = mpsc::channel(1);
        tx
    }

    async fn subscriber_tx(&self, post_id: &str, user_id: &str) -> Option<mpsc::Sender<String>> {
        let session = self.sessions.get(post_id)?.clone();
        let s = session.lock().await;
        s.subscribers
            .iter()
            .find(|sub| sub.user_id == user_id)
            .map(|sub| sub.tx.clone())
    }
}

async fn broadcast(session: &Session, skip_user: Option<&str>, payload: &str) {
    for sub in &session.subscribers {
        if matches!(skip_user, Some(u) if u == sub.user_id) {
            continue;
        }
        let _ = sub.tx.try_send(payload.to_string());
    }
}

async fn send_error(tx: &mpsc::Sender<String>, post_id: &str, err: &str) -> Result<(), ()> {
    let msg = ServerMessage::CollabError {
        post_id: post_id.to_string(),
        code: "collab_failed".into(),
        message: err.to_string(),
    }
    .to_json();
    tx.try_send(msg).map_err(|_| ())
}
