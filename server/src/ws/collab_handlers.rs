//! Handlers for the CRDT collaboration WS message group: posts (`Collab*`),
//! whiteboards (`Whiteboard*`) and channel documents (`ChannelDoc*`). Extracted
//! from `connection.rs`.
//!
//! All three resource kinds share the same subscribe / unsubscribe / update /
//! awareness lifecycle, differing only by their `ResourceRef` constructor and
//! the rate-limit bucket, so the common logic is factored into the private
//! `*_resource` helpers. The one exception is whiteboard updates, which carry
//! their own 30/sec cap for live drawing.

use std::collections::HashSet;

use tokio::sync::mpsc;

use crate::collab::resource::ResourceRef;
use crate::collab::CollabManager;
use crate::middleware::rate_limit::{
    check_rate_limit, collab_subscribe_key, whiteboard_awareness_key, whiteboard_subscribe_key,
    whiteboard_update_key, RateLimitConfig,
};
use crate::AppState;

/// Cap on the serialized JSON size of an awareness blob. Held in memory per
/// session and broadcast to every peer, so an unbounded blob is a DoS
/// amplification vector independent of the rate limit.
const MAX_AWARENESS_BYTES: usize = 4 * 1024;

fn awareness_too_large(value: &serde_json::Value) -> bool {
    crate::ws::connection_helpers::awareness_too_large(value, MAX_AWARENESS_BYTES)
}

// ─────────────── shared resource lifecycle ───────────────

async fn subscribe_resource(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    collab_subscriptions: &mut HashSet<ResourceRef>,
    r: ResourceRef,
    sub_key: String,
) {
    // Rate-limit subscribe churn so the (uncached) authz path can't be
    // hammered. 10/sec per user is plenty for legitimate page navigation.
    if check_rate_limit(
        &state.redis,
        &RateLimitConfig {
            key_prefix: sub_key,
            limit: 10,
            window_secs: 1,
        },
    )
    .await
    .is_err()
    {
        return;
    }
    collab_subscriptions.insert(r.clone());
    if let Err(e) = state.collab.subscribe(&r, user_id, out_tx.clone()).await {
        CollabManager::send_error(out_tx, &r, "subscribe_failed", &e).await;
    }
}

async fn unsubscribe_resource(
    state: &AppState,
    user_id: &str,
    collab_subscriptions: &mut HashSet<ResourceRef>,
    r: ResourceRef,
) {
    collab_subscriptions.remove(&r);
    state.collab.unsubscribe(&r, user_id).await;
}

async fn apply_update(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    r: ResourceRef,
    update_b64: String,
) {
    if let Err(e) = state.collab.apply_update(&r, user_id, &update_b64).await {
        CollabManager::send_error(out_tx, &r, "update_failed", &e).await;
    }
}

async fn update_awareness(
    state: &AppState,
    user_id: &str,
    r: ResourceRef,
    aw_state: serde_json::Value,
) {
    // Size cap independent of rate limit — an awareness blob is amplified to
    // every peer on broadcast, so an unbounded payload is a DoS amplifier.
    if awareness_too_large(&aw_state) {
        return;
    }
    // Awareness fan-out is amplified to every peer; cap at 2/sec to bound
    // broadcast bandwidth even with the size cap.
    let rate_key = whiteboard_awareness_key(user_id, &r.id);
    if check_rate_limit(
        &state.redis,
        &RateLimitConfig {
            key_prefix: rate_key,
            limit: 2,
            window_secs: 1,
        },
    )
    .await
    .is_err()
    {
        return;
    }
    state.collab.update_awareness(&r, user_id, aw_state).await;
}

// ─────────────── posts ───────────────

pub async fn collab_subscribe(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    collab_subscriptions: &mut HashSet<ResourceRef>,
    post_id: String,
) {
    let key = collab_subscribe_key(user_id);
    subscribe_resource(
        state,
        out_tx,
        user_id,
        collab_subscriptions,
        ResourceRef::post(post_id),
        key,
    )
    .await;
}

pub async fn collab_unsubscribe(
    state: &AppState,
    user_id: &str,
    collab_subscriptions: &mut HashSet<ResourceRef>,
    post_id: String,
) {
    unsubscribe_resource(
        state,
        user_id,
        collab_subscriptions,
        ResourceRef::post(post_id),
    )
    .await;
}

pub async fn collab_update(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    post_id: String,
    update_b64: String,
) {
    apply_update(
        state,
        out_tx,
        user_id,
        ResourceRef::post(post_id),
        update_b64,
    )
    .await;
}

pub async fn awareness_update(
    state: &AppState,
    user_id: &str,
    post_id: String,
    aw_state: serde_json::Value,
) {
    update_awareness(state, user_id, ResourceRef::post(post_id), aw_state).await;
}

// ─────────────── whiteboards ───────────────

pub async fn whiteboard_subscribe(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    collab_subscriptions: &mut HashSet<ResourceRef>,
    whiteboard_id: String,
) {
    let key = whiteboard_subscribe_key(user_id);
    subscribe_resource(
        state,
        out_tx,
        user_id,
        collab_subscriptions,
        ResourceRef::whiteboard(whiteboard_id),
        key,
    )
    .await;
}

pub async fn whiteboard_unsubscribe(
    state: &AppState,
    user_id: &str,
    collab_subscriptions: &mut HashSet<ResourceRef>,
    whiteboard_id: String,
) {
    unsubscribe_resource(
        state,
        user_id,
        collab_subscriptions,
        ResourceRef::whiteboard(whiteboard_id),
    )
    .await;
}

pub async fn whiteboard_update(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    whiteboard_id: String,
    update_b64: String,
) {
    let r = ResourceRef::whiteboard(whiteboard_id);
    // Rate cap: 30 stroke-updates/sec per user per whiteboard. Live drawing
    // must stay smooth; this only catches malicious or runaway clients.
    let rate_key = whiteboard_update_key(user_id, &r.id);
    if check_rate_limit(
        &state.redis,
        &RateLimitConfig {
            key_prefix: rate_key,
            limit: 30,
            window_secs: 1,
        },
    )
    .await
    .is_err()
    {
        CollabManager::send_error(out_tx, &r, "rate_limited", "Too many updates").await;
        return;
    }
    if let Err(e) = state.collab.apply_update(&r, user_id, &update_b64).await {
        CollabManager::send_error(out_tx, &r, "update_failed", &e).await;
    }
}

pub async fn whiteboard_awareness(
    state: &AppState,
    user_id: &str,
    whiteboard_id: String,
    aw_state: serde_json::Value,
) {
    update_awareness(
        state,
        user_id,
        ResourceRef::whiteboard(whiteboard_id),
        aw_state,
    )
    .await;
}

// ─────────────── channel documents ───────────────

pub async fn channel_doc_subscribe(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    collab_subscriptions: &mut HashSet<ResourceRef>,
    channel_id: String,
) {
    let key = collab_subscribe_key(user_id);
    subscribe_resource(
        state,
        out_tx,
        user_id,
        collab_subscriptions,
        ResourceRef::channel_doc(channel_id),
        key,
    )
    .await;
}

pub async fn channel_doc_unsubscribe(
    state: &AppState,
    user_id: &str,
    collab_subscriptions: &mut HashSet<ResourceRef>,
    channel_id: String,
) {
    unsubscribe_resource(
        state,
        user_id,
        collab_subscriptions,
        ResourceRef::channel_doc(channel_id),
    )
    .await;
}

pub async fn channel_doc_update(
    state: &AppState,
    out_tx: &mpsc::Sender<String>,
    user_id: &str,
    channel_id: String,
    update_b64: String,
) {
    apply_update(
        state,
        out_tx,
        user_id,
        ResourceRef::channel_doc(channel_id),
        update_b64,
    )
    .await;
}

pub async fn channel_doc_awareness(
    state: &AppState,
    user_id: &str,
    channel_id: String,
    aw_state: serde_json::Value,
) {
    update_awareness(
        state,
        user_id,
        ResourceRef::channel_doc(channel_id),
        aw_state,
    )
    .await;
}
