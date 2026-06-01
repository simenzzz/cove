//! Docker-backed WebSocket integration tests.
//!
//! These boot the *real* app (via `nexus_server::build_state` + `build_router`)
//! on an ephemeral port and drive the live `/ws` endpoint with a real client.
//! They require a running Redis + SurrealDB and are therefore `#[ignore]`d so a
//! plain `cargo test` stays infra-free.
//!
//! Bring up services + run:
//! ```text
//! docker run -d --name nexus-test-redis  -p 6379:6379 redis:7-alpine \
//!     redis-server --requirepass testpass
//! docker run -d --name nexus-test-surreal -p 8000:8000 surrealdb/surrealdb:v2 \
//!     start --user root --pass root --bind 0.0.0.0:8000 memory
//! cargo test --test ws -- --ignored --test-threads=1
//! ```
//!
//! Overridable via `TEST_SURREAL_URL` / `TEST_REDIS_URL`. Run single-threaded:
//! each test gets a fresh SurrealDB database, but Redis is shared, so the
//! harness `FLUSHDB`s on spawn.

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message as WsMsg;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use nexus_server::config::{AppConfig, Environment};
use nexus_server::{build_router, build_state};

const RECV_TIMEOUT: Duration = Duration::from_secs(5);

// ─────────────────────────── harness ───────────────────────────

struct TestApp {
    base: String,
    ws_url: String,
    http: reqwest::Client,
}

fn unique(prefix: &str) -> String {
    // Keep usernames within the 3-32 char limit: prefix + 8 hex chars.
    let id = uuid::Uuid::new_v4().simple().to_string();
    format!("{prefix}{}", &id[..8])
}

/// Normalize a SurrealDB record id (in any of its JSON encodings) to its bare
/// key — mirrors the frontend `recordKey` util. The WS layer expects bare keys
/// (e.g. `find_by_id` does `select(("channel", id))`).
fn record_key(v: &Value) -> String {
    match v {
        Value::String(s) => s
            .split_once(':')
            .map(|(_, k)| k.to_string())
            .unwrap_or_else(|| s.clone()),
        Value::Object(map) => match map.get("id") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Object(inner)) => match inner.values().next() {
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string().trim_matches('"').to_string(),
                None => String::new(),
            },
            _ => String::new(),
        },
        _ => String::new(),
    }
}

async fn spawn_app() -> TestApp {
    let surreal_url =
        std::env::var("TEST_SURREAL_URL").unwrap_or_else(|_| "127.0.0.1:8000".to_string());
    let redis_url = std::env::var("TEST_REDIS_URL")
        .unwrap_or_else(|_| "redis://:testpass@127.0.0.1:6379".to_string());

    let config = AppConfig {
        env: Environment::Development,
        surreal_url,
        surreal_ns: "nexus_test".to_string(),
        surreal_db: unique("db_"), // fresh database per test for isolation
        surreal_user: "root".to_string(),
        surreal_pass: "root".to_string(),
        jwt_secret: "integration-test-secret-key-at-least-32-bytes".to_string(),
        access_token_expiry_minutes: 15,
        refresh_token_expiry_days: 7,
        redis_url,
        server_host: "127.0.0.1".to_string(),
        server_port: 0,
        secure_cookies: false,
        cors_origin: "http://localhost:3000".to_string(),
        api_rate_limit: 100_000,
        api_rate_window_secs: 10,
    };

    // Non-installing handle: the global metrics recorder may only be installed
    // once per process, and spawn_app runs once per test.
    let handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .build_recorder()
        .handle();

    let state = build_state(config, handle).await.expect("build_state");

    // Load the schema into this test's fresh database.
    let schema = include_str!("../../db/init.surql");
    state.db.query(schema).await.expect("load schema");

    // Clear shared Redis state (rate limits, sequences, presence, tickets).
    let mut conn = state.redis.get().await.expect("redis conn");
    let _: () = redis::cmd("FLUSHDB")
        .query_async(&mut conn)
        .await
        .expect("flushdb");
    drop(conn);

    let app = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    TestApp {
        base: format!("http://{addr}"),
        ws_url: format!("ws://{addr}/ws"),
        http: reqwest::Client::builder()
            .cookie_store(true)
            .build()
            .unwrap(),
    }
}

impl TestApp {
    /// Register a user, returning their access token.
    async fn register(&self, username: &str) -> String {
        let email = format!("{}@test.example.com", username);
        let res = self
            .http
            .post(format!("{}/api/auth/register", self.base))
            .json(&json!({ "email": email, "username": username, "display_name": username, "password": "password123" }))
            .send()
            .await
            .expect("register send");
        assert!(
            res.status().is_success(),
            "register failed: {}",
            res.status()
        );
        let body: Value = res.json().await.expect("register json");
        body["access_token"]
            .as_str()
            .expect("access_token")
            .to_string()
    }

    async fn create_server(&self, token: &str, name: &str) -> String {
        let res = self
            .http
            .post(format!("{}/api/servers", self.base))
            .bearer_auth(token)
            .json(&json!({ "name": name }))
            .send()
            .await
            .expect("create server send");
        assert!(
            res.status().is_success(),
            "create server failed: {}",
            res.status()
        );
        let body: Value = res.json().await.expect("server json");
        record_key(&body["server"]["id"])
    }

    async fn create_channel(
        &self,
        token: &str,
        server_id: &str,
        name: &str,
        channel_type: &str,
    ) -> String {
        let res = self
            .http
            .post(format!("{}/api/servers/{server_id}/channels", self.base))
            .bearer_auth(token)
            .json(&json!({ "name": name, "channel_type": channel_type }))
            .send()
            .await
            .expect("create channel send");
        assert!(
            res.status().is_success(),
            "create channel failed: {}",
            res.status()
        );
        let body: Value = res.json().await.expect("channel json");
        record_key(&body["channel"]["id"])
    }

    async fn create_text_channel(&self, token: &str, server_id: &str, name: &str) -> String {
        self.create_channel(token, server_id, name, "text").await
    }

    async fn create_draft(&self, token: &str, title: &str) -> String {
        let res = self
            .http
            .post(format!("{}/api/posts", self.base))
            .bearer_auth(token)
            .json(&json!({ "title": title }))
            .send()
            .await
            .expect("create draft send");
        assert!(
            res.status().is_success(),
            "create draft failed: {}",
            res.status()
        );
        let body: Value = res.json().await.expect("post json");
        record_key(&body["post"]["id"])
    }

    async fn join_server(&self, token: &str, server_id: &str) {
        let res = self
            .http
            .post(format!("{}/api/servers/{server_id}/join", self.base))
            .bearer_auth(token)
            .send()
            .await
            .expect("join send");
        assert!(res.status().is_success(), "join failed: {}", res.status());
    }

    async fn ws_ticket(&self, token: &str) -> (String, String) {
        let res = self
            .http
            .post(format!("{}/api/auth/ws-ticket", self.base))
            .bearer_auth(token)
            .send()
            .await
            .expect("ws-ticket send");
        assert!(
            res.status().is_success(),
            "ws-ticket failed: {}",
            res.status()
        );
        let body: Value = res.json().await.expect("ticket json");
        (
            body["ticket"].as_str().expect("ticket").to_string(),
            body["nonce"].as_str().expect("nonce").to_string(),
        )
    }

    /// Open a WS connection and complete the auth handshake.
    async fn connect_authed(&self, token: &str) -> WsConn {
        let (ticket, nonce) = self.ws_ticket(token).await;
        let (ws, _resp) = connect_async(self.ws_url.as_str())
            .await
            .expect("ws connect");
        let mut conn = WsConn { ws };
        conn.send(json!({ "v": 1, "type": "auth", "ticket": ticket, "nonce": nonce }))
            .await;
        let ok = conn.recv_type("auth_ok").await;
        assert!(
            ok["heartbeat_interval"].is_number(),
            "auth_ok missing heartbeat_interval: {ok}"
        );
        conn
    }
}

// ─────────────────────────── ws client ───────────────────────────

struct WsConn {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl WsConn {
    async fn send(&mut self, v: Value) {
        self.ws
            .send(WsMsg::Text(v.to_string().into()))
            .await
            .expect("ws send");
    }

    /// Receive the next decoded JSON text frame (ignoring ping/pong/binary).
    async fn recv_json(&mut self) -> Value {
        loop {
            let frame = tokio::time::timeout(RECV_TIMEOUT, self.ws.next())
                .await
                .expect("ws recv timed out")
                .expect("ws stream closed")
                .expect("ws frame error");
            if let WsMsg::Text(t) = frame {
                return serde_json::from_str(t.as_str()).expect("decode ws json");
            }
        }
    }

    /// Receive until a frame of the given `type`, skipping others (presence, etc).
    async fn recv_type(&mut self, ty: &str) -> Value {
        loop {
            let m = self.recv_json().await;
            if m["type"] == ty {
                return m;
            }
        }
    }

    /// Round-trip barrier: heartbeat -> heartbeat_ack guarantees the server has
    /// processed everything we sent before it (e.g. a prior subscribe).
    async fn sync(&mut self) {
        self.send(json!({ "v": 1, "type": "heartbeat" })).await;
        self.recv_type("heartbeat_ack").await;
    }

    async fn subscribe(&mut self, channel_id: &str) {
        self.send(json!({
            "v": 1, "type": "subscribe", "channel_id": channel_id, "level": "active"
        }))
        .await;
    }

    async fn watch_subscribe(&mut self, channel_id: &str) {
        self.send(json!({ "v": 1, "type": "watch_subscribe", "channel_id": channel_id }))
            .await;
    }
}

// ─────────────────────────── tests ───────────────────────────

#[tokio::test]
#[ignore = "requires docker redis + surrealdb"]
async fn ws_auth_handshake_succeeds() {
    let app = spawn_app().await;
    let token = app.register(&unique("alice_")).await;
    let _conn = app.connect_authed(&token).await; // asserts auth_ok internally
}

#[tokio::test]
#[ignore = "requires docker redis + surrealdb"]
async fn ws_rejects_bad_ticket() {
    let app = spawn_app().await;
    let (ws, _resp) = connect_async(app.ws_url.as_str())
        .await
        .expect("ws connect");
    let mut conn = WsConn { ws };
    conn.send(json!({ "v": 1, "type": "auth", "ticket": "bogus", "nonce": "bogus" }))
        .await;
    // Server must not authenticate a forged ticket: expect an error or a close,
    // never an auth_ok.
    let outcome = tokio::time::timeout(RECV_TIMEOUT, conn.ws.next()).await;
    if let Ok(Some(Ok(WsMsg::Text(t)))) = outcome {
        let v: Value = serde_json::from_str(t.as_str()).unwrap();
        assert_ne!(v["type"], "auth_ok", "forged ticket must not authenticate");
    }
}

#[tokio::test]
#[ignore = "requires docker redis + surrealdb"]
async fn ws_heartbeat_is_acked() {
    let app = spawn_app().await;
    let token = app.register(&unique("bob_")).await;
    let mut conn = app.connect_authed(&token).await;
    conn.send(json!({ "v": 1, "type": "heartbeat" })).await;
    let ack = conn.recv_type("heartbeat_ack").await;
    assert_eq!(ack["type"], "heartbeat_ack");
}

#[tokio::test]
#[ignore = "requires docker redis + surrealdb"]
async fn ws_chat_message_acks_sender() {
    let app = spawn_app().await;
    let token = app.register(&unique("carol_")).await;
    let server_id = app.create_server(&token, "Cove").await;
    let channel_id = app.create_text_channel(&token, &server_id, "general").await;

    let mut conn = app.connect_authed(&token).await;
    conn.subscribe(&channel_id).await;
    conn.send(json!({
        "v": 1, "type": "chat_message",
        "channel_id": channel_id, "content": "hello world", "nonce": "n-123"
    }))
    .await;

    let ack = conn.recv_type("message_ack").await;
    assert_eq!(ack["nonce"], "n-123");
    assert!(ack["message_id"].is_string());
    assert!(ack["seq"].as_u64().unwrap() >= 1);
}

#[tokio::test]
#[ignore = "requires docker redis + surrealdb"]
async fn ws_chat_message_rejected_when_not_subscribed() {
    let app = spawn_app().await;
    let token = app.register(&unique("dave_")).await;
    let server_id = app.create_server(&token, "Cove").await;
    let channel_id = app.create_text_channel(&token, &server_id, "general").await;

    let mut conn = app.connect_authed(&token).await;
    // No subscribe — the ChatMessage arm must reject.
    conn.send(json!({
        "v": 1, "type": "chat_message",
        "channel_id": channel_id, "content": "hi", "nonce": "n-1"
    }))
    .await;
    let err = conn.recv_type("error").await;
    assert!(
        err["message"].as_str().unwrap().contains("Not subscribed"),
        "expected not-subscribed error, got {err}"
    );
}

#[tokio::test]
#[ignore = "requires docker redis + surrealdb"]
async fn ws_watch_subscribe_and_queue_add() {
    let app = spawn_app().await;
    let token = app.register(&unique("erin_")).await;
    let server_id = app.create_server(&token, "Cove").await;
    let channel_id = app
        .create_channel(&token, &server_id, "movies", "watch")
        .await;

    let mut conn = app.connect_authed(&token).await;
    conn.watch_subscribe(&channel_id).await;
    // Joining a watch room replies with the current room state.
    let state = conn.recv_type("watch_state").await;
    assert_eq!(state["channel_id"], channel_id);

    // Add a queue item; the room acks the optimistic nonce.
    conn.send(json!({
        "v": 1, "type": "watch_queue_add",
        "channel_id": channel_id,
        "video_id": "dQw4w9WgXcQ",
        "title": "test clip",
        "duration_ms": 1000,
        "thumbnail_url": null,
        "nonce": "q-nonce-1"
    }))
    .await;
    let ack = conn.recv_type("watch_queue_ack").await;
    assert_eq!(ack["nonce"], "q-nonce-1");
}

#[tokio::test]
#[ignore = "requires docker redis + surrealdb"]
async fn ws_collab_subscribe_to_post_replays_state() {
    let app = spawn_app().await;
    let token = app.register(&unique("frank_")).await;
    let post_id = app.create_draft(&token, "My Draft").await;

    let mut conn = app.connect_authed(&token).await;
    conn.send(json!({ "v": 1, "type": "collab_subscribe", "post_id": post_id }))
        .await;
    // The author is authorized; subscribing replays the current CRDT state.
    let state = conn.recv_type("collab_state").await;
    assert_eq!(state["post_id"], post_id);
    assert!(state["state_b64"].is_string());
}

#[tokio::test]
#[ignore = "requires docker redis + surrealdb"]
async fn ws_chat_message_broadcasts_to_other_subscriber() {
    let app = spawn_app().await;
    let owner = app.register(&unique("owner_")).await;
    let server_id = app.create_server(&owner, "Cove").await;
    let channel_id = app.create_text_channel(&owner, &server_id, "general").await;

    let member = app.register(&unique("member_")).await;
    app.join_server(&member, &server_id).await;

    let mut a = app.connect_authed(&owner).await;
    let mut b = app.connect_authed(&member).await;
    a.subscribe(&channel_id).await;
    b.subscribe(&channel_id).await;
    // Barrier: ensure both joins are processed by the room before A sends.
    a.sync().await;
    b.sync().await;

    a.send(json!({
        "v": 1, "type": "chat_message",
        "channel_id": channel_id, "content": "broadcast me", "nonce": "bn-1"
    }))
    .await;

    // A gets the ack; B gets the broadcast (sender is excluded from broadcast).
    let ack = a.recv_type("message_ack").await;
    assert_eq!(ack["nonce"], "bn-1");

    let bcast = b.recv_type("chat_message").await;
    assert_eq!(bcast["content"], "broadcast me");
    assert_eq!(bcast["channel_id"], channel_id);
    assert!(bcast["seq"].as_u64().unwrap() >= 1);
}
