pub mod awareness;
pub mod channel_doc_store;
pub mod doc;
pub mod post_store;
pub mod resource;
mod tasks;
pub mod whiteboard_store;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde_json::Value;
use tokio::sync::{mpsc, Mutex};

use crate::repositories::post::PostRepo;

use awareness::Awareness;
use doc::CollabDoc;
use post_store::PostStore;
use resource::{
    awareness_message, closed_message, error_message, state_message, update_message, ResourceKind,
    ResourceRef, ResourceStore, Snapshot,
};
use tasks::{broadcast, persist_loop, sweeper_loop};

/// Hard cap on concurrent subscribers per document. Roadmap §2.4.2.
pub const MAX_COLLABORATORS: usize = 10;

/// How often the sweeper inspects sessions for idle eviction.
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(30);

/// A session with no subscribers and no edits within this window is dropped
/// from memory by the sweeper. Roadmap §2.4.3.
pub const IDLE_TTL: Duration = Duration::from_secs(60);

/// In-memory CRDT session. One per [`ResourceRef`] while the resource is
/// being edited; evicted lazily on last unsubscribe and defensively by the
/// sweeper if it goes idle.
pub(crate) struct Session {
    pub(crate) doc: CollabDoc,
    pub(crate) awareness: Awareness,
    pub(crate) subscribers: Vec<Subscriber>,
    /// True when the in-memory doc has edits not yet flushed to the store.
    pub(crate) dirty: bool,
    /// Timestamp of the most recent `apply_update`. Drives idle eviction.
    pub(crate) last_update_at: Instant,
    /// Single-flight flag: true while a debounced persist task is scheduled
    /// or running. The persist loop clears it before exiting; the next dirty
    /// edit will spawn a fresh task.
    pub(crate) persist_pending: bool,
}

pub(crate) struct Subscriber {
    pub(crate) user_id: String,
    pub(crate) tx: mpsc::Sender<String>,
}

/// Aborts a `JoinHandle` when dropped. Used so the sweeper task stops once
/// all `CollabManager` clones go out of scope (mainly relevant in tests).
struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// In-memory CRDT manager. Generic over resource kind via [`ResourceStore`]
/// — Phase 2 wires a [`PostStore`], Phase 3 also wires a `WhiteboardStore`.
/// Holds active sessions keyed by [`ResourceRef`], dispatches WS messages,
/// debounces persistence, and runs an idle eviction sweeper.
#[derive(Clone)]
pub struct CollabManager {
    sessions: Arc<DashMap<ResourceRef, Arc<Mutex<Session>>>>,
    stores: Arc<HashMap<ResourceKind, Arc<dyn ResourceStore>>>,
    _sweeper: Arc<AbortOnDrop>,
}

impl CollabManager {
    /// Phase 2 convenience constructor — wires only the post store with the
    /// default debounce/sweep intervals.
    pub fn new(posts: Arc<dyn PostRepo>) -> Self {
        let mut stores: HashMap<ResourceKind, Arc<dyn ResourceStore>> = HashMap::new();
        stores.insert(ResourceKind::Post, Arc::new(PostStore::new(posts)));
        Self::with_stores(stores, SWEEP_INTERVAL, IDLE_TTL)
    }

    /// Wire multiple resource stores at once.
    pub fn with_stores(
        stores: HashMap<ResourceKind, Arc<dyn ResourceStore>>,
        sweep_interval: Duration,
        idle_ttl: Duration,
    ) -> Self {
        let sessions: Arc<DashMap<ResourceRef, Arc<Mutex<Session>>>> = Arc::new(DashMap::new());
        let stores = Arc::new(stores);
        let sweeper = {
            let sessions = sessions.clone();
            let stores = stores.clone();
            tokio::spawn(async move {
                sweeper_loop(sessions, stores, sweep_interval, idle_ttl).await;
            })
        };
        Self {
            sessions,
            stores,
            _sweeper: Arc::new(AbortOnDrop(sweeper)),
        }
    }

    /// Test-only constructor: post-only, with caller-supplied intervals.
    pub fn with_intervals(
        posts: Arc<dyn PostRepo>,
        persist_debounce: Duration,
        sweep_interval: Duration,
        idle_ttl: Duration,
    ) -> Self {
        let mut stores: HashMap<ResourceKind, Arc<dyn ResourceStore>> = HashMap::new();
        stores.insert(
            ResourceKind::Post,
            Arc::new(post_store::PostStore::with_debounce(
                posts,
                persist_debounce,
            )),
        );
        Self::with_stores(stores, sweep_interval, idle_ttl)
    }

    fn store_for(&self, kind: ResourceKind) -> Result<Arc<dyn ResourceStore>, String> {
        self.stores
            .get(&kind)
            .cloned()
            .ok_or_else(|| format!("No store registered for {:?}", kind))
    }

    /// Subscribe `user_id` to a resource's session, hydrating from the store
    /// on cache miss. Sends a `*_state` message to `tx` on success.
    pub async fn subscribe(
        &self,
        r: &ResourceRef,
        user_id: &str,
        tx: mpsc::Sender<String>,
    ) -> Result<(), String> {
        let store = self.store_for(r.kind)?;
        store.authorize(r, user_id).await?;

        let session = match self.sessions.get(r).map(|s| s.clone()) {
            Some(s) => s,
            None => {
                let snap = store.load(r).await?;
                // Use the store's own cap so already-persisted large docs
                // (whiteboards up to 4 MB) can be rehydrated. The default
                // 256 KB cap would brick them.
                let doc = CollabDoc::from_snapshot_with_cap(&snap.state_b64, store.max_doc_bytes())
                    .map_err(|e| format!("Bad snapshot: {e}"))?;
                let s = Arc::new(Mutex::new(Session {
                    doc,
                    awareness: Awareness::new(),
                    subscribers: Vec::new(),
                    dirty: false,
                    last_update_at: Instant::now(),
                    persist_pending: false,
                }));
                self.sessions.insert(r.clone(), s.clone());
                s
            }
        };

        let mut s = session.lock().await;

        let already_subscribed = s.subscribers.iter().any(|sub| sub.user_id == user_id);
        if !already_subscribed && s.subscribers.len() >= MAX_COLLABORATORS {
            return Err(format!(
                "Session full (max {MAX_COLLABORATORS} collaborators)"
            ));
        }

        let snap = Snapshot {
            state_b64: s.doc.encode_state(),
            state_vector_b64: s.doc.encode_state_vector(),
        };
        let _ = tx.send(state_message(r, &snap).to_json()).await;

        if !s.awareness.is_empty() {
            let aw = awareness_message(r, s.awareness.snapshot()).to_json();
            let _ = tx.send(aw).await;
        }

        s.subscribers.retain(|sub| sub.user_id != user_id);
        s.subscribers.push(Subscriber {
            user_id: user_id.to_string(),
            tx,
        });
        Ok(())
    }

    /// Drop a user from a session. Public so the WS layer can clean up
    /// dangling subscriptions when a connection closes without sending an
    /// explicit unsubscribe (tab close, network drop).
    pub async fn unsubscribe(&self, r: &ResourceRef, user_id: &str) {
        let Some(session) = self.sessions.get(r).map(|s| s.clone()) else {
            return;
        };

        let empty = {
            let mut s = session.lock().await;
            s.subscribers.retain(|sub| sub.user_id != user_id);
            s.awareness.remove(user_id);

            let payload = awareness_message(r, s.awareness.snapshot()).to_json();
            broadcast(&s, None, &payload).await;
            s.subscribers.is_empty()
        };

        if empty {
            self.flush_now(r).await;
            self.sessions.remove(r);
        }
    }

    /// Apply a remote update from `from_user` and fan it out to peers.
    pub async fn apply_update(
        &self,
        r: &ResourceRef,
        from_user: &str,
        update_b64: &str,
    ) -> Result<(), String> {
        let store = self.store_for(r.kind)?;
        let max_update = store.max_update_bytes();
        let max_doc = store.max_doc_bytes();

        let session = self
            .sessions
            .get(r)
            .ok_or_else(|| "Not subscribed".to_string())?
            .clone();

        let need_spawn = {
            let mut s = session.lock().await;
            s.doc
                .apply_update_with_cap(update_b64, max_update)
                .map_err(|e| format!("Apply failed: {e}"))?;

            // Post-merge ceiling: reject the update if the merged doc would
            // grow past the per-resource cap. The update has already been
            // applied to the local doc — but since no other clients see it
            // until broadcast below, returning the error here is sufficient
            // protection for *future* peers' hydration size. Tombstones we
            // can't undo are acceptable (yrs converges anyway).
            if s.doc.encoded_state_len() > max_doc {
                return Err(format!("Document exceeds {max_doc}-byte limit"));
            }

            let payload =
                update_message(r, update_b64.to_string(), from_user.to_string()).to_json();
            broadcast(&s, Some(from_user), &payload).await;

            s.dirty = true;
            s.last_update_at = Instant::now();
            if s.persist_pending {
                false
            } else {
                s.persist_pending = true;
                true
            }
        };

        if need_spawn {
            let store = self.store_for(r.kind)?;
            let debounce = store.persist_debounce();
            let session_for_task = session.clone();
            let r_owned = r.clone();
            tokio::spawn(async move {
                persist_loop(session_for_task, store, r_owned, debounce).await;
            });
        }
        Ok(())
    }

    /// Update awareness state for a user and broadcast the new snapshot.
    pub async fn update_awareness(&self, r: &ResourceRef, user_id: &str, state: Value) {
        if let Some(session) = self.sessions.get(r) {
            let session = session.clone();
            let mut s = session.lock().await;
            s.awareness.update(user_id.to_string(), state);
            let payload = awareness_message(r, s.awareness.snapshot()).to_json();
            broadcast(&s, None, &payload).await;
        }
    }

    /// Tear down a session — flush pending bytes, run the store's `on_close`
    /// hook, notify subscribers, evict the doc. No-op if no session is
    /// cached (nothing to tear down; resource-level finalization is the
    /// caller's responsibility).
    ///
    /// The broadcast + eviction always happens, even when `on_close` errors,
    /// so a failed finalizer can't strand the session in memory. The
    /// `on_close` error is returned to the caller so they can decide whether
    /// the partial teardown is fatal.
    pub async fn close(&self, r: &ResourceRef, reason: &str) -> Result<(), String> {
        let store = self.store_for(r.kind)?;
        self.flush_now(r).await;

        let Some(session) = self.sessions.get(r).map(|s| s.clone()) else {
            return Ok(());
        };
        let on_close_err = {
            let s = session.lock().await;
            // Run the store hook first, but record the error rather than
            // returning early — we still want to broadcast and evict.
            let err = store.on_close(r, &s.doc).await.err();
            let payload = closed_message(r, reason.to_string()).to_json();
            broadcast(&s, None, &payload).await;
            err
        };
        self.sessions.remove(r);
        match on_close_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Helper for the WS layer: build an error ServerMessage scoped to the
    /// resource and emit it via `tx`. Falls back to silent drop on send fail.
    pub async fn send_error(tx: &mpsc::Sender<String>, r: &ResourceRef, code: &str, message: &str) {
        let _ = tx.try_send(error_message(r, code.to_string(), message.to_string()).to_json());
    }

    /// Force an immediate save and clear the dirty flag. Safe to call when
    /// nothing is dirty (returns without touching the store). Used at session
    /// teardown points to bound data loss.
    async fn flush_now(&self, r: &ResourceRef) {
        let Some(session) = self.sessions.get(r).map(|s| s.clone()) else {
            return;
        };
        let payload = {
            let mut s = session.lock().await;
            if !s.dirty {
                return;
            }
            s.dirty = false;
            Some(Snapshot {
                state_b64: s.doc.encode_state(),
                state_vector_b64: s.doc.encode_state_vector(),
            })
        };
        if let Some(snap) = payload {
            if let Ok(store) = self.store_for(r.kind) {
                if let Err(e) = store.save(r, snap).await {
                    tracing::warn!(resource = ?r, error = %e, "Flush failed");
                    if let Some(session) = self.sessions.get(r).map(|s| s.clone()) {
                        session.lock().await.dirty = true;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
