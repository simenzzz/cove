use super::*;
use crate::models::post::Post;
use crate::repositories::post::MockPostRepo;
use serde_json::json;
use yrs::{ReadTxn, StateVector, Text, Transact};

fn post_ref() -> ResourceRef {
    ResourceRef::post("p1")
}

fn fake_post(author: &str, published: bool) -> Post {
    fake_post_with_state(author, published, String::new())
}

fn fake_post_with_state(author: &str, published: bool, state_b64: String) -> Post {
    Post {
        id: Some(surrealdb::RecordId::from(("post", "p1"))),
        author: surrealdb::RecordId::from(("user", author)),
        title: "draft".into(),
        state_b64,
        state_vector_b64: String::new(),
        published,
        published_content: None,
        created_at: None,
        updated_at: None,
        author_username: None,
        author_display_name: None,
    }
}

fn encoded_state_with_text(text: &str) -> String {
    let doc = yrs::Doc::new();
    let t = doc.get_or_insert_text(doc::TEXT_ROOT);
    let mut txn = doc.transact_mut();
    t.insert(&mut txn, 0, text);
    let bytes = txn.encode_state_as_update_v1(&StateVector::default());
    drop(txn);
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn encoded_update_inserting(text: &str) -> String {
    encoded_state_with_text(text)
}

fn fast_intervals() -> CollabManager {
    let posts = MockPostRepo::new();
    CollabManager::with_intervals(
        Arc::new(posts),
        Duration::from_millis(20),
        Duration::from_millis(25),
        Duration::from_millis(50),
    )
}

fn fast_intervals_with(posts: Arc<dyn PostRepo>) -> CollabManager {
    CollabManager::with_intervals(
        posts,
        Duration::from_millis(20),
        Duration::from_millis(25),
        Duration::from_millis(50),
    )
}

#[tokio::test]
async fn subscribe_rejects_non_collaborator() {
    let mut posts = MockPostRepo::new();
    posts
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_post("author", false))));
    posts.expect_is_invited().returning(|_, _| Ok(false));

    let manager = CollabManager::new(Arc::new(posts));
    let (tx, _rx) = mpsc::channel(8);

    let result = manager.subscribe(&post_ref(), "stranger", tx).await;
    assert!(result.is_err());
    assert!(manager.sessions.get(&post_ref()).is_none());
}

#[tokio::test]
async fn subscribe_rejects_published_post() {
    let mut posts = MockPostRepo::new();
    posts
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_post("author", true))));

    let manager = CollabManager::new(Arc::new(posts));
    let (tx, _rx) = mpsc::channel(8);

    let result = manager.subscribe(&post_ref(), "author", tx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn unsubscribe_evicts_session_when_last_user_leaves() {
    let mut posts = MockPostRepo::new();
    posts
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_post("u1", false))));
    posts.expect_is_invited().never();

    let manager = CollabManager::new(Arc::new(posts));
    let (tx, _rx) = mpsc::channel(8);
    manager
        .subscribe(&post_ref(), "u1", tx)
        .await
        .expect("subscribe");
    assert!(manager.sessions.get(&post_ref()).is_some());

    manager.unsubscribe(&post_ref(), "u1").await;
    assert!(
        manager.sessions.get(&post_ref()).is_none(),
        "session should be evicted when the last subscriber leaves"
    );
}

#[tokio::test]
async fn close_removes_session_and_notifies() {
    let mut posts = MockPostRepo::new();
    posts
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_post("u1", false))));
    posts.expect_is_invited().never();

    let manager = CollabManager::new(Arc::new(posts));
    let (tx, mut rx) = mpsc::channel(8);
    manager
        .subscribe(&post_ref(), "u1", tx)
        .await
        .expect("subscribe");

    let _ = rx.recv().await;

    manager
        .close(&post_ref(), "published")
        .await
        .expect("close should succeed");
    assert!(manager.sessions.get(&post_ref()).is_none());

    let notice = rx.recv().await.expect("subscriber should be notified");
    assert!(notice.contains("collab_closed"));
    assert!(notice.contains("published"));
}

#[tokio::test]
async fn update_broadcast_reaches_other_subscriber() {
    let mut posts = MockPostRepo::new();
    posts
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_post("u1", false))));
    posts.expect_is_invited().returning(|_, _| Ok(true));
    posts.expect_save_snapshot().returning(|_, _, _| Ok(()));

    let manager = CollabManager::new(Arc::new(posts));

    let (tx_a, mut rx_a) = mpsc::channel(8);
    let (tx_b, mut rx_b) = mpsc::channel(8);
    manager
        .subscribe(&post_ref(), "u1", tx_a)
        .await
        .expect("u1 subscribe");
    manager
        .subscribe(&post_ref(), "u2", tx_b)
        .await
        .expect("u2 subscribe");

    let _ = rx_a.recv().await;
    let _ = rx_b.recv().await;

    let update = encoded_update_inserting("hello");
    manager
        .apply_update(&post_ref(), "u1", &update)
        .await
        .expect("apply_update");

    let received = rx_b.recv().await.expect("u2 receives update");
    assert!(received.contains("collab_update"));
    assert!(received.contains("\"from_user\":\"u1\""));
    assert!(received.contains(&update));

    let echoed = rx_a.try_recv();
    assert!(
        echoed.is_err(),
        "sender should not receive their own update echo, got: {echoed:?}"
    );
}

#[tokio::test]
async fn awareness_broadcast_reaches_other_subscriber() {
    let mut posts = MockPostRepo::new();
    posts
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_post("u1", false))));
    posts.expect_is_invited().returning(|_, _| Ok(true));

    let manager = CollabManager::new(Arc::new(posts));

    let (tx_a, mut rx_a) = mpsc::channel(8);
    let (tx_b, mut rx_b) = mpsc::channel(8);
    manager
        .subscribe(&post_ref(), "u1", tx_a)
        .await
        .expect("u1 subscribe");
    manager
        .subscribe(&post_ref(), "u2", tx_b)
        .await
        .expect("u2 subscribe");

    let _ = rx_a.recv().await;
    let _ = rx_b.recv().await;
    while rx_a.try_recv().is_ok() {}
    while rx_b.try_recv().is_ok() {}

    manager
        .update_awareness(&post_ref(), "u1", json!({ "cursor": 7 }))
        .await;

    let msg = rx_b.recv().await.expect("u2 receives awareness");
    assert!(msg.contains("awareness_state"));
    assert!(msg.contains("\"u1\""));
    assert!(msg.contains("\"cursor\":7"));
}

#[tokio::test]
async fn state_replays_to_resubscribed_user() {
    let seeded = encoded_state_with_text("durable");

    let mut posts = MockPostRepo::new();
    let seeded_clone = seeded.clone();
    posts.expect_find_by_id().returning(move |_| {
        Ok(Some(fake_post_with_state(
            "u1",
            false,
            seeded_clone.clone(),
        )))
    });
    posts.expect_is_invited().returning(|_, _| Ok(true));
    posts.expect_save_snapshot().returning(|_, _, _| Ok(()));

    let manager = CollabManager::new(Arc::new(posts));

    let (tx1, mut rx1) = mpsc::channel(8);
    manager
        .subscribe(&post_ref(), "u2", tx1)
        .await
        .expect("subscribe");
    let initial = rx1.recv().await.expect("collab_state");
    assert!(initial.contains("collab_state"));
    assert!(initial.contains(&seeded));

    manager.unsubscribe(&post_ref(), "u2").await;
    let (tx2, mut rx2) = mpsc::channel(8);
    manager
        .subscribe(&post_ref(), "u2", tx2)
        .await
        .expect("resubscribe");
    let replay = rx2.recv().await.expect("collab_state on resubscribe");
    assert!(replay.contains("collab_state"));
    assert!(replay.contains(&seeded));
}

#[tokio::test]
async fn subscribe_rejects_when_session_full() {
    let mut posts = MockPostRepo::new();
    posts
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_post("author", false))));
    posts.expect_is_invited().returning(|_, _| Ok(true));

    let manager = CollabManager::new(Arc::new(posts));

    for i in 0..MAX_COLLABORATORS {
        let (tx, _rx) = mpsc::channel(8);
        std::mem::forget(_rx);
        manager
            .subscribe(&post_ref(), &format!("u{i}"), tx)
            .await
            .expect("under cap");
    }

    let (tx, _rx) = mpsc::channel(8);
    let err = manager
        .subscribe(&post_ref(), "overflow", tx)
        .await
        .expect_err("should reject when full");
    assert!(err.contains("Session full"), "unexpected error: {err}");
}

#[tokio::test]
async fn evicted_session_rehydrates_from_snapshot() {
    let saved: Arc<std::sync::Mutex<String>> = Arc::new(std::sync::Mutex::new(String::new()));

    let mut posts = MockPostRepo::new();
    let saved_for_find = saved.clone();
    posts.expect_find_by_id().returning(move |_| {
        let state = saved_for_find.lock().unwrap().clone();
        Ok(Some(fake_post_with_state("u1", false, state)))
    });
    posts.expect_is_invited().returning(|_, _| Ok(true));
    let saved_for_save = saved.clone();
    posts.expect_save_snapshot().returning(move |_, state, _| {
        *saved_for_save.lock().unwrap() = state;
        Ok(())
    });

    let manager = CollabManager::with_intervals(
        Arc::new(posts),
        Duration::from_millis(20),
        Duration::from_millis(500),
        Duration::from_secs(60),
    );

    let (tx, mut rx) = mpsc::channel(8);
    manager
        .subscribe(&post_ref(), "u1", tx)
        .await
        .expect("subscribe");
    let _ = rx.recv().await;

    let update = encoded_update_inserting("evict-me");
    manager
        .apply_update(&post_ref(), "u1", &update)
        .await
        .expect("apply");

    tokio::time::sleep(Duration::from_millis(80)).await;

    manager.unsubscribe(&post_ref(), "u1").await;
    assert!(
        manager.sessions.get(&post_ref()).is_none(),
        "session evicted"
    );
    assert!(
        !saved.lock().unwrap().is_empty(),
        "snapshot should have been persisted"
    );

    let (tx2, mut rx2) = mpsc::channel(8);
    manager
        .subscribe(&post_ref(), "u1", tx2)
        .await
        .expect("resubscribe");
    let hello = rx2.recv().await.expect("collab_state");
    assert!(hello.contains("collab_state"));
    let saved_b64 = saved.lock().unwrap().clone();
    assert!(
        hello.contains(&saved_b64),
        "expected state to embed saved snapshot {saved_b64}, got {hello}"
    );

    let restored = doc::CollabDoc::from_snapshot(&saved_b64).expect("restore");
    assert_eq!(restored.text(), "evict-me");
}

#[tokio::test]
async fn idle_session_evicted_after_ttl() {
    let mut posts = MockPostRepo::new();
    posts
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_post("u1", false))));
    posts.expect_is_invited().never();

    let manager = fast_intervals_with(Arc::new(posts));

    let (tx, _rx) = mpsc::channel(8);
    manager
        .subscribe(&post_ref(), "u1", tx)
        .await
        .expect("subscribe");

    manager.unsubscribe(&post_ref(), "u1").await;
    assert!(
        manager.sessions.get(&post_ref()).is_none(),
        "unsubscribe evicts"
    );

    let zombie = ResourceRef::post("zombie");
    let stale = Arc::new(Mutex::new(Session {
        doc: doc::CollabDoc::new(),
        awareness: Awareness::new(),
        subscribers: Vec::new(),
        dirty: false,
        last_update_at: Instant::now() - Duration::from_secs(3600),
        persist_pending: false,
    }));
    manager.sessions.insert(zombie.clone(), stale);

    tokio::time::sleep(Duration::from_millis(120)).await;
    assert!(
        manager.sessions.get(&zombie).is_none(),
        "sweeper should have evicted the idle session"
    );
}

#[tokio::test]
async fn fast_intervals_constructor_smoke() {
    let _m = fast_intervals();
}

// ---------- §3.1 verification: post + whiteboard coexistence ----------

use crate::collab::whiteboard_store::WhiteboardStore;
use crate::models::channel::{Channel, ChannelType};
use crate::repositories::channel::MockChannelRepo;
use crate::repositories::server::MockServerRepo;
use crate::repositories::whiteboard::MockWhiteboardRepo;

fn wb_ref() -> ResourceRef {
    ResourceRef::whiteboard("c1")
}

fn fake_channel(id: &str, server_id: &str, kind: ChannelType) -> Channel {
    Channel {
        id: Some(surrealdb::RecordId::from(("channel", id))),
        name: "wb".into(),
        channel_type: kind,
        server: Some(surrealdb::RecordId::from(("server", server_id))),
        created_at: None,
    }
}

fn manager_with_both(
    posts: Arc<dyn PostRepo>,
    whiteboards: Arc<dyn crate::repositories::whiteboard::WhiteboardRepo>,
    channels: Arc<dyn crate::repositories::channel::ChannelRepo>,
    servers: Arc<dyn crate::repositories::server::ServerRepo>,
) -> CollabManager {
    let mut stores: HashMap<ResourceKind, Arc<dyn ResourceStore>> = HashMap::new();
    stores.insert(
        ResourceKind::Post,
        Arc::new(post_store::PostStore::with_debounce(
            posts,
            Duration::from_millis(20),
        )),
    );
    stores.insert(
        ResourceKind::Whiteboard,
        Arc::new(WhiteboardStore::with_intervals(
            whiteboards,
            channels,
            servers,
            Duration::from_millis(20),
            0,
        )),
    );
    CollabManager::with_stores(stores, Duration::from_millis(500), Duration::from_secs(60))
}

#[tokio::test]
async fn posts_and_whiteboards_coexist_in_one_manager() {
    let mut posts = MockPostRepo::new();
    posts
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_post("u1", false))));
    posts.expect_is_invited().never();
    posts.expect_save_snapshot().returning(|_, _, _| Ok(()));

    let mut wbs = MockWhiteboardRepo::new();
    wbs.expect_find_by_channel().returning(|_| Ok(None));
    wbs.expect_upsert_snapshot().returning(|_, _, _| Ok(1));

    let mut chans = MockChannelRepo::new();
    chans
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_channel("c1", "s1", ChannelType::Whiteboard))));

    let mut servers = MockServerRepo::new();
    servers.expect_is_member().returning(|_, _| Ok(true));

    let manager = manager_with_both(
        Arc::new(posts),
        Arc::new(wbs),
        Arc::new(chans),
        Arc::new(servers),
    );

    // Subscribe to a post — must succeed and seed a session.
    let (tx_post, mut rx_post) = mpsc::channel(8);
    manager
        .subscribe(&post_ref(), "u1", tx_post)
        .await
        .expect("post subscribe");
    let hello_post = rx_post.recv().await.expect("post collab_state");
    assert!(hello_post.contains("collab_state"));

    // Subscribe to a whiteboard with the same user — must succeed and
    // seed a *separate* session keyed by ResourceRef::Whiteboard.
    let (tx_wb, mut rx_wb) = mpsc::channel(8);
    manager
        .subscribe(&wb_ref(), "u1", tx_wb)
        .await
        .expect("whiteboard subscribe");
    let hello_wb = rx_wb.recv().await.expect("whiteboard state");
    assert!(hello_wb.contains("whiteboard_state"));

    // Both sessions live in the same DashMap, keyed independently.
    assert!(manager.sessions.get(&post_ref()).is_some());
    assert!(manager.sessions.get(&wb_ref()).is_some());
}

#[tokio::test]
async fn whiteboard_authorize_rejects_non_member() {
    let posts = MockPostRepo::new();
    let wbs = MockWhiteboardRepo::new();
    let mut chans = MockChannelRepo::new();
    chans
        .expect_find_by_id()
        .returning(|_| Ok(Some(fake_channel("c1", "s1", ChannelType::Whiteboard))));
    let mut servers = MockServerRepo::new();
    servers.expect_is_member().returning(|_, _| Ok(false));

    let manager = manager_with_both(
        Arc::new(posts),
        Arc::new(wbs),
        Arc::new(chans),
        Arc::new(servers),
    );

    let (tx, _rx) = mpsc::channel(8);
    let err = manager
        .subscribe(&wb_ref(), "stranger", tx)
        .await
        .expect_err("non-member must be rejected");
    assert!(err.contains("Not a member"));
    // No session was created.
    assert!(manager.sessions.get(&wb_ref()).is_none());
}
