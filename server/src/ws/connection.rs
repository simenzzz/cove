use std::collections::{HashMap, HashSet};

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::header::ORIGIN;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::auth::ws_ticket;
use crate::collab::resource::ResourceRef;
use crate::ws::connection_helpers::{
    awareness_too_large as awareness_too_large_inner, check_channel_type_access,
    compute_presence_audience, refresh_audience_if_stale,
};

/// Server-side min-interval between successive heartbeats from one
/// connection. The client claims it heartbeats every 30s; anything tighter
/// than 2s is either a bug or a probe.
const MIN_HEARTBEAT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);
const MAX_VOICE_SIGNAL_BYTES: usize = 16 * 1024;
use crate::ws::chat_handlers;
use crate::ws::collab_handlers;
use crate::ws::presence;
use crate::ws::protocol::{ClientMessage, ServerMessage, SubscriptionLevel};
use crate::ws::room::RoomCommand;
use crate::ws::voice_types::VoiceCommand;
use crate::ws::watch_handlers;
use crate::ws::watch_types::WatchCommand;
use crate::AppState;

pub async fn handle_ws_upgrade(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Response, Response> {
    if state.config.env.is_production() {
        let origin = headers.get(ORIGIN).and_then(|v| v.to_str().ok());
        if origin != Some(state.config.cors_origin.as_str()) {
            return Err(Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body("Invalid origin".into())
                .unwrap_or_default());
        }
    }

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state)))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let auth = tokio::time::timeout(std::time::Duration::from_secs(5), socket.next()).await;
    let (ticket, nonce) = match auth {
        Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str::<ClientMessage>(&text) {
            Ok(ClientMessage::Auth { ticket, nonce }) => (ticket, nonce),
            _ => {
                let _ = socket
                    .send(Message::Text(
                        ServerMessage::Error {
                            message: "Expected auth message".into(),
                        }
                        .to_json()
                        .into(),
                    ))
                    .await;
                return;
            }
        },
        _ => {
            let _ = socket
                .send(Message::Text(
                    ServerMessage::Error {
                        message: "Authentication timeout".into(),
                    }
                    .to_json()
                    .into(),
                ))
                .await;
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Consume ticket atomically (GETDEL) and validate the bound nonce in
    // constant time. Credentials arrive in the first WS frame, not the URL,
    // so reverse-proxy access logs never see them.
    let user_id = match ws_ticket::consume_ticket(&state.redis, &ticket, &nonce).await {
        Ok(Some(id)) => id,
        _ => {
            let _ = ws_sender
                .send(Message::Text(
                    ServerMessage::Error {
                        message: "Invalid or expired ticket".into(),
                    }
                    .to_json()
                    .into(),
                ))
                .await;
            return;
        }
    };

    // Fetch user profile for real display data.
    let (username, display_name, avatar_url) = match state.repos.users.find_by_id(&user_id).await {
        Ok(Some(user)) => (user.username, user.display_name, user.avatar_url),
        _ => (user_id.clone(), user_id.clone(), None),
    };

    // Send auth_ok
    let auth_ok = ServerMessage::AuthOk {
        user_id: user_id.clone(),
        heartbeat_interval: 30000,
    };
    if ws_sender
        .send(Message::Text(auth_ok.to_json().into()))
        .await
        .is_err()
    {
        return;
    }

    // Set user online
    let _ = presence::set_online_with_ttl(&state.redis, &user_id, 300).await;
    crate::metrics::record_ws_connect();

    // Channel for outgoing messages (writer task reads from this)
    let (out_tx, mut out_rx) = mpsc::channel::<String>(256);

    // Snapshot whether the user already had another connection BEFORE this
    // one registers — multi-tab users shouldn't re-broadcast "online" every
    // time they open a new tab. The check must come before `register`.
    let was_offline_before_register = !state.user_connections.is_online(&user_id);

    // Generate a unique connection ID and register
    let conn_id = uuid::Uuid::new_v4().to_string();
    state
        .user_connections
        .register(&user_id, conn_id.clone(), out_tx.clone());

    // Presence audience: graph-scoped union of friends + server co-members.
    // Cached locally with a TTL so a block / unfriend / server-leave mid
    // session takes effect within `AUDIENCE_TTL` instead of waiting for the
    // user to reconnect — Phase 1.3 spec wants presence scoped to live graph
    // relationships, not the snapshot taken at connect time.
    let mut audience = compute_presence_audience(&state, &user_id).await;
    let mut audience_fetched_at = std::time::Instant::now();

    let online_msg = ServerMessage::Presence {
        user_id: user_id.clone(),
        status: "online".to_string(),
    }
    .to_json();

    if was_offline_before_register
        && presence::try_claim_flap_slot(&state.redis, &user_id, "online").await
    {
        for audience_id in &audience {
            if state.user_connections.is_online(audience_id) {
                state
                    .user_connections
                    .send_to_user(audience_id, online_msg.clone())
                    .await;
            }
        }
    } else if was_offline_before_register {
        crate::metrics::record_presence_flap_suppressed();
        tracing::debug!(%user_id, "suppressed online broadcast — flap slot held");
    }

    // Spawn writer task
    let writer_handle = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if ws_sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Track subscriptions
    let mut subscriptions: HashMap<String, SubscriptionLevel> = HashMap::new();
    let mut collab_subscriptions: HashSet<ResourceRef> = HashSet::new();
    let mut watch_subscriptions: HashSet<String> = HashSet::new();
    let mut voice_subscriptions: HashSet<String> = HashSet::new();
    let mut last_typing: HashMap<String, std::time::Instant> = HashMap::new();

    let heartbeat_timeout = std::time::Duration::from_secs(60);
    let idle_timeout = std::time::Duration::from_secs(300);

    let mut heartbeat_deadline = tokio::time::Instant::now() + heartbeat_timeout;
    let mut idle_deadline = tokio::time::Instant::now() + idle_timeout;
    let mut is_idle = false;
    // For min-interval enforcement on heartbeats.
    let mut last_heartbeat_at: Option<std::time::Instant> = None;

    // Main read loop — race heartbeat + idle timeouts against incoming messages
    loop {
        let msg = tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(m)) => m,
                    _ => break, // Connection closed or error
                }
            }
            _ = tokio::time::sleep_until(heartbeat_deadline) => {
                tracing::info!(%user_id, "Heartbeat timeout, disconnecting");
                break;
            }
            _ = tokio::time::sleep_until(idle_deadline) => {
                if !is_idle {
                    is_idle = true;
                    let _ = presence::set_status(&state.redis, &user_id, "idle").await;
                    if presence::try_claim_flap_slot(&state.redis, &user_id, "idle").await {
                        refresh_audience_if_stale(
                            &state, &user_id,
                            &mut audience, &mut audience_fetched_at,
                        ).await;
                        let idle_msg = ServerMessage::Presence {
                            user_id: user_id.clone(),
                            status: "idle".to_string(),
                        }.to_json();
                        for audience_id in &audience {
                            state.user_connections.send_to_user(audience_id, idle_msg.clone()).await;
                        }
                    } else {
                        crate::metrics::record_presence_flap_suppressed();
                    }
                }
                // Reset idle deadline to keep checking, but don't disconnect
                idle_deadline = tokio::time::Instant::now() + idle_timeout;
                continue;
            }
        };

        match msg {
            Message::Text(text) => {
                let client_msg: ClientMessage = match serde_json::from_str(&text) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                match client_msg {
                    ClientMessage::Auth { .. } => {
                        // Already authenticated via the first frame; ignore repeats.
                    }
                    ClientMessage::Heartbeat => {
                        let now = std::time::Instant::now();
                        if let Some(prev) = last_heartbeat_at {
                            if now.duration_since(prev) < MIN_HEARTBEAT_INTERVAL {
                                tracing::warn!(
                                    %user_id,
                                    "heartbeat min-interval violated; dropping"
                                );
                                continue;
                            }
                        }
                        last_heartbeat_at = Some(now);
                        heartbeat_deadline = tokio::time::Instant::now() + heartbeat_timeout;
                        idle_deadline = tokio::time::Instant::now() + idle_timeout;

                        // Restore from idle if needed
                        if is_idle {
                            is_idle = false;
                            let _ =
                                presence::set_online_with_ttl(&state.redis, &user_id, 300).await;
                            if presence::try_claim_flap_slot(&state.redis, &user_id, "online").await
                            {
                                refresh_audience_if_stale(
                                    &state,
                                    &user_id,
                                    &mut audience,
                                    &mut audience_fetched_at,
                                )
                                .await;
                                let online_msg = ServerMessage::Presence {
                                    user_id: user_id.clone(),
                                    status: "online".to_string(),
                                }
                                .to_json();
                                for audience_id in &audience {
                                    state
                                        .user_connections
                                        .send_to_user(audience_id, online_msg.clone())
                                        .await;
                                }
                            } else {
                                crate::metrics::record_presence_flap_suppressed();
                            }
                        } else {
                            let _ =
                                presence::set_online_with_ttl(&state.redis, &user_id, 300).await;
                        }

                        let _ = out_tx.send(ServerMessage::HeartbeatAck.to_json()).await;
                    }
                    ClientMessage::Subscribe { channel_id, level } => {
                        chat_handlers::subscribe(
                            &state,
                            &out_tx,
                            &user_id,
                            &username,
                            &mut subscriptions,
                            channel_id,
                            level,
                        )
                        .await;
                    }
                    ClientMessage::Unsubscribe { channel_id } => {
                        chat_handlers::unsubscribe(
                            &state,
                            &user_id,
                            &mut subscriptions,
                            channel_id,
                        )
                        .await;
                    }
                    ClientMessage::ChatMessage {
                        channel_id,
                        content,
                        nonce,
                    } => {
                        chat_handlers::chat_message(
                            &state,
                            &out_tx,
                            &user_id,
                            &username,
                            &display_name,
                            &avatar_url,
                            &subscriptions,
                            channel_id,
                            content,
                            nonce,
                        )
                        .await;
                    }
                    ClientMessage::Typing { channel_id } => {
                        chat_handlers::typing(
                            &state,
                            &user_id,
                            &username,
                            &subscriptions,
                            &mut last_typing,
                            channel_id,
                        )
                        .await;
                    }
                    ClientMessage::Resume { last_seq } => {
                        chat_handlers::resume(
                            &state,
                            &out_tx,
                            &user_id,
                            &username,
                            &mut subscriptions,
                            last_seq,
                        )
                        .await;
                    }
                    // Phase 2 collab + Phase 3 whiteboard messages: route to
                    // CollabManager via a typed ResourceRef.
                    ClientMessage::CollabSubscribe { post_id } => {
                        collab_handlers::collab_subscribe(
                            &state,
                            &out_tx,
                            &user_id,
                            &mut collab_subscriptions,
                            post_id,
                        )
                        .await;
                    }
                    ClientMessage::CollabUnsubscribe { post_id } => {
                        collab_handlers::collab_unsubscribe(
                            &state,
                            &user_id,
                            &mut collab_subscriptions,
                            post_id,
                        )
                        .await;
                    }
                    ClientMessage::CollabUpdate {
                        post_id,
                        update_b64,
                    } => {
                        collab_handlers::collab_update(
                            &state, &out_tx, &user_id, post_id, update_b64,
                        )
                        .await;
                    }
                    ClientMessage::AwarenessUpdate {
                        post_id,
                        state: aw_state,
                    } => {
                        collab_handlers::awareness_update(&state, &user_id, post_id, aw_state)
                            .await;
                    }
                    ClientMessage::WhiteboardSubscribe { whiteboard_id } => {
                        collab_handlers::whiteboard_subscribe(
                            &state,
                            &out_tx,
                            &user_id,
                            &mut collab_subscriptions,
                            whiteboard_id,
                        )
                        .await;
                    }
                    ClientMessage::WhiteboardUnsubscribe { whiteboard_id } => {
                        collab_handlers::whiteboard_unsubscribe(
                            &state,
                            &user_id,
                            &mut collab_subscriptions,
                            whiteboard_id,
                        )
                        .await;
                    }
                    ClientMessage::WhiteboardUpdate {
                        whiteboard_id,
                        update_b64,
                    } => {
                        collab_handlers::whiteboard_update(
                            &state,
                            &out_tx,
                            &user_id,
                            whiteboard_id,
                            update_b64,
                        )
                        .await;
                    }
                    ClientMessage::WhiteboardAwarenessUpdate {
                        whiteboard_id,
                        state: aw_state,
                    } => {
                        collab_handlers::whiteboard_awareness(
                            &state,
                            &user_id,
                            whiteboard_id,
                            aw_state,
                        )
                        .await;
                    }
                    ClientMessage::ChannelDocSubscribe { channel_id } => {
                        collab_handlers::channel_doc_subscribe(
                            &state,
                            &out_tx,
                            &user_id,
                            &mut collab_subscriptions,
                            channel_id,
                        )
                        .await;
                    }
                    ClientMessage::ChannelDocUnsubscribe { channel_id } => {
                        collab_handlers::channel_doc_unsubscribe(
                            &state,
                            &user_id,
                            &mut collab_subscriptions,
                            channel_id,
                        )
                        .await;
                    }
                    ClientMessage::ChannelDocUpdate {
                        channel_id,
                        update_b64,
                    } => {
                        collab_handlers::channel_doc_update(
                            &state, &out_tx, &user_id, channel_id, update_b64,
                        )
                        .await;
                    }
                    ClientMessage::ChannelDocAwarenessUpdate {
                        channel_id,
                        state: aw_state,
                    } => {
                        collab_handlers::channel_doc_awareness(
                            &state, &user_id, channel_id, aw_state,
                        )
                        .await;
                    }
                    ClientMessage::VoiceJoin { channel_id } => {
                        if !check_channel_type_access(
                            &state,
                            &channel_id,
                            &user_id,
                            crate::models::channel::ChannelType::Voice,
                        )
                        .await
                        {
                            let _ = out_tx
                                .send(
                                    ServerMessage::VoiceError {
                                        channel_id: channel_id.clone(),
                                        code: "forbidden".into(),
                                        message: "Not a voice channel or not a member".into(),
                                    }
                                    .to_json(),
                                )
                                .await;
                            continue;
                        }
                        let room = state.voice_manager.get_or_create(&channel_id).await;
                        let _ = room
                            .send(VoiceCommand::Join {
                                user_id: user_id.clone(),
                                username: username.clone(),
                                sender: out_tx.clone(),
                            })
                            .await;
                        voice_subscriptions.insert(channel_id);
                    }
                    ClientMessage::VoiceLeave { channel_id } => {
                        if let Some(room) = state.voice_manager.get_room(&channel_id).await {
                            let _ = room
                                .send(VoiceCommand::Leave {
                                    user_id: user_id.clone(),
                                })
                                .await;
                        }
                        voice_subscriptions.remove(&channel_id);
                    }
                    ClientMessage::VoiceSignal {
                        channel_id,
                        to_user_id,
                        signal,
                    } => {
                        if !voice_subscriptions.contains(&channel_id) {
                            let _ = out_tx
                                .send(
                                    ServerMessage::VoiceError {
                                        channel_id,
                                        code: "not_joined".into(),
                                        message: "Not in voice room".into(),
                                    }
                                    .to_json(),
                                )
                                .await;
                            continue;
                        }
                        if awareness_too_large_inner(&signal, MAX_VOICE_SIGNAL_BYTES) {
                            continue;
                        }
                        if let Some(room) = state.voice_manager.get_room(&channel_id).await {
                            let _ = room
                                .send(VoiceCommand::Signal {
                                    from_user: user_id.clone(),
                                    to_user: to_user_id,
                                    signal,
                                    reply_to: out_tx.clone(),
                                })
                                .await;
                        }
                    }
                    // ── Phase 4: watch-together rooms ──
                    ClientMessage::WatchSubscribe { channel_id } => {
                        watch_handlers::subscribe(
                            &state,
                            &out_tx,
                            &user_id,
                            &username,
                            &mut watch_subscriptions,
                            channel_id,
                        )
                        .await;
                    }
                    ClientMessage::WatchUnsubscribe { channel_id } => {
                        watch_handlers::unsubscribe(
                            &state,
                            &user_id,
                            &mut watch_subscriptions,
                            channel_id,
                        )
                        .await;
                    }
                    ClientMessage::WatchTransferLeader {
                        channel_id,
                        to_user_id,
                    } => {
                        watch_handlers::transfer_leader(
                            &state,
                            &out_tx,
                            &user_id,
                            &watch_subscriptions,
                            channel_id,
                            to_user_id,
                        )
                        .await;
                    }
                    ClientMessage::WatchPlayback {
                        channel_id,
                        action,
                        position_ms,
                        client_ts: _,
                    } => {
                        watch_handlers::playback(
                            &state,
                            &out_tx,
                            &user_id,
                            &watch_subscriptions,
                            channel_id,
                            action,
                            position_ms,
                        )
                        .await;
                    }
                    ClientMessage::WatchQueueAdd {
                        channel_id,
                        video_id,
                        title,
                        duration_ms,
                        thumbnail_url,
                        nonce,
                    } => {
                        watch_handlers::queue_add(
                            &state,
                            &out_tx,
                            &user_id,
                            &watch_subscriptions,
                            channel_id,
                            video_id,
                            title,
                            duration_ms,
                            thumbnail_url,
                            nonce,
                        )
                        .await;
                    }
                    ClientMessage::WatchQueueRemove {
                        channel_id,
                        item_id,
                    } => {
                        watch_handlers::queue_remove(
                            &state,
                            &out_tx,
                            &user_id,
                            &watch_subscriptions,
                            channel_id,
                            item_id,
                        )
                        .await;
                    }
                    ClientMessage::WatchVote {
                        channel_id,
                        item_id,
                        value,
                    } => {
                        watch_handlers::vote(
                            &state,
                            &out_tx,
                            &user_id,
                            &watch_subscriptions,
                            channel_id,
                            item_id,
                            value,
                        )
                        .await;
                    }
                    ClientMessage::WatchSkip { channel_id } => {
                        watch_handlers::skip(
                            &state,
                            &out_tx,
                            &user_id,
                            &watch_subscriptions,
                            channel_id,
                        )
                        .await;
                    }
                    ClientMessage::WatchReaction { channel_id, emoji } => {
                        watch_handlers::reaction(
                            &state,
                            &out_tx,
                            &user_id,
                            &username,
                            &watch_subscriptions,
                            channel_id,
                            emoji,
                        )
                        .await;
                    }
                    ClientMessage::WatchProgress {
                        channel_id,
                        position_ms,
                    } => {
                        watch_handlers::progress(
                            &state,
                            &out_tx,
                            &user_id,
                            &watch_subscriptions,
                            channel_id,
                            position_ms,
                        )
                        .await;
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup — unregister first so `is_online` reflects whether any *other*
    // connection survives this disconnect (e.g. another tab).
    crate::metrics::record_ws_disconnect();
    state.user_connections.unregister(&user_id, &conn_id);

    // 30-second offline grace period. Phase 1.3 spec: don't flap to "offline"
    // on a brief network blip — the user reopening their laptop within 30s
    // should never trigger an offline → online round-trip for the whole
    // audience. We spawn a detached task that re-checks `is_online` after
    // the grace window; if any other connection arrived (or stayed) we skip
    // the offline broadcast entirely.
    if !state.user_connections.is_online(&user_id) {
        let state_for_grace = state.clone();
        let user_for_grace = user_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            if state_for_grace.user_connections.is_online(&user_for_grace) {
                return;
            }
            let _ = presence::set_offline(&state_for_grace.redis, &user_for_grace).await;
            // Same flap slot as online/idle transitions — if the user just
            // emitted any presence event within the lock TTL, suppress the
            // offline echo to keep audience traffic bounded under a
            // reconnect-loop attacker.
            if !presence::try_claim_flap_slot(&state_for_grace.redis, &user_for_grace, "offline")
                .await
            {
                crate::metrics::record_presence_flap_suppressed();
                return;
            }
            // Re-resolve audience fresh: 30s have passed, the user may have
            // joined/left servers or blocked someone before disconnecting.
            // We want the offline event to honor the current graph, not
            // whatever was cached at connect time.
            let fresh_audience = compute_presence_audience(&state_for_grace, &user_for_grace).await;
            let offline_msg = ServerMessage::Presence {
                user_id: user_for_grace.clone(),
                status: "offline".to_string(),
            }
            .to_json();
            for audience_id in &fresh_audience {
                state_for_grace
                    .user_connections
                    .send_to_user(audience_id, offline_msg.clone())
                    .await;
            }
        });
    }

    for channel_id in subscriptions.keys() {
        if let Some(room) = state.room_manager.get_room(channel_id).await {
            let _ = room
                .send(RoomCommand::Leave {
                    user_id: user_id.clone(),
                })
                .await;
        }
    }

    // Tear down any dangling collab/whiteboard subscriptions — without this
    // the CollabManager session holds a dead `mpsc::Sender` and never evicts.
    for r in &collab_subscriptions {
        state.collab.unsubscribe(r, &user_id).await;
    }

    // Same for watch rooms: drop the actor's reference so it can leader-handoff
    // and eventually grace-period-evict if the room is now empty.
    for channel_id in &watch_subscriptions {
        if let Some(room) = state.watch_manager.get_room(channel_id).await {
            let _ = room
                .send(WatchCommand::Leave {
                    user_id: user_id.clone(),
                })
                .await;
        }
    }

    for channel_id in &voice_subscriptions {
        if let Some(room) = state.voice_manager.get_room(channel_id).await {
            let _ = room
                .send(VoiceCommand::Leave {
                    user_id: user_id.clone(),
                })
                .await;
        }
    }

    drop(out_tx);
    let _ = writer_handle.await;
    tracing::info!(%user_id, "WebSocket client disconnected");
}
