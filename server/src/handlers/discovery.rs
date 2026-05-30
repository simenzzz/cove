use std::collections::HashMap;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::models::server::Server;
use crate::AppState;

/// One row of the friend-overlap discovery query. `server` is the related
/// server's record id (not the full document); we hydrate the full docs in a
/// single follow-up query below so the client never has to N+1 fetch names.
#[derive(Debug, Serialize, Deserialize)]
struct ServerFriendCount {
    server: surrealdb::RecordId,
    friend_count: u64,
}

/// Merge a server document with its friend-overlap count into a single JSON
/// object the client can render directly (`{ ...server, friend_count }`).
fn server_with_count(server: Server, friend_count: u64) -> Result<Value, AppError> {
    let mut value = serde_json::to_value(server)
        .map_err(|e| AppError::Internal(format!("failed to serialize server: {e}")))?;
    if let Value::Object(map) = &mut value {
        map.insert("friend_count".into(), json!(friend_count));
    }
    Ok(value)
}

/// Discover servers ranked by friend-member overlap.
/// Traverses: user -> friends_with -> user -> member_of -> server
pub async fn discover_servers(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, AppError> {
    let limit = 20u32;

    // Get friend IDs
    let friend_ids = state.repos.social.get_friend_ids(&claims.sub).await?;

    if friend_ids.is_empty() {
        // No friends — return recent servers the user hasn't joined
        let mut result = state
            .db
            .query(
                "SELECT * FROM server WHERE id NOT IN \
                 (SELECT VALUE out FROM member_of WHERE in = $user) \
                 ORDER BY created_at DESC LIMIT $limit",
            )
            .bind(("user", surrealdb::RecordId::from(("user", &claims.sub))))
            .bind(("limit", limit))
            .await?;
        // Deserialize into the typed model: the SurrealDB SDK cannot decode
        // rows containing RecordId/Datetime fields into serde_json::Value.
        let servers: Vec<Server> = result.take(0)?;
        // Uniform shape with the friends branch: every entry carries friend_count.
        let out = servers
            .into_iter()
            .map(|s| server_with_count(s, 0))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Json(json!({ "servers": out })));
    }

    let friend_record_ids: Vec<surrealdb::RecordId> = friend_ids
        .iter()
        .map(|id| surrealdb::RecordId::from(("user", id.as_str())))
        .collect();

    // Find servers where friends are members, ranked by friend overlap
    // In RELATE user -> member_of -> server: in=user, out=server
    let mut result = state
        .db
        .query(
            "SELECT out AS server, count() AS friend_count \
             FROM member_of WHERE in IN $friends \
             AND out NOT IN (SELECT VALUE out FROM member_of WHERE in = $user) \
             GROUP BY server \
             ORDER BY friend_count DESC \
             LIMIT $limit",
        )
        .bind(("friends", friend_record_ids))
        .bind(("user", surrealdb::RecordId::from(("user", &claims.sub))))
        .bind(("limit", limit))
        .await?;

    let counts: Vec<ServerFriendCount> = result.take(0)?;
    if counts.is_empty() {
        return Ok(Json(json!({ "servers": [] })));
    }

    // Hydrate the full server documents in ONE query (was an N+1 round-trip
    // per server from the client), then merge each doc with its friend_count
    // while preserving the friend-overlap ordering from the query above.
    let server_ids: Vec<surrealdb::RecordId> = counts.iter().map(|c| c.server.clone()).collect();
    let mut server_result = state
        .db
        .query("SELECT * FROM server WHERE id IN $ids")
        .bind(("ids", server_ids))
        .await?;
    let servers: Vec<Server> = server_result.take(0)?;

    let by_id: HashMap<String, Server> = servers
        .into_iter()
        .filter_map(|s| s.id.as_ref().map(|id| (id.key().to_string(), s.clone())))
        .collect();

    let out = counts
        .into_iter()
        .filter_map(|c| {
            by_id
                .get(&c.server.key().to_string())
                .cloned()
                .map(|s| server_with_count(s, c.friend_count))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(json!({ "servers": out })))
}
