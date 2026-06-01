use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::models::channel::{ChannelType, CreateChannel};
use crate::models::server::CreateServer;
use crate::ws::protocol::{NotificationUser, ServerMessage};
use crate::AppState;

/// Extract the key portion from a SurrealDB RecordId string ("table:key" → "key")
fn extract_record_key(record_id: &surrealdb::RecordId) -> String {
    let s = record_id.to_string();
    s.split_once(':').map(|(_, k)| k.to_string()).unwrap_or(s)
}

fn now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis().max(0) as u64
}

pub async fn create_server(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(input): Json<CreateServer>,
) -> Result<Json<Value>, AppError> {
    // Validate server name
    let name = input.name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::BadRequest(
            "Server name must be 1-100 characters".into(),
        ));
    }

    let server = state.repos.servers.create(input, &claims.sub).await?;

    let server_id = server
        .id
        .as_ref()
        .ok_or_else(|| AppError::Internal("Server created without ID".into()))
        .map(extract_record_key)?;

    // Auto-add owner as member
    state
        .repos
        .servers
        .add_member(&server_id, &claims.sub)
        .await?;

    // Auto-create #general channel
    let general = CreateChannel {
        name: "general".to_string(),
        channel_type: ChannelType::Text,
    };
    let channel = state.repos.channels.create(general, &server_id).await?;

    Ok(Json(json!({
        "server": server,
        "channel": channel,
    })))
}

pub async fn get_server(
    State(state): State<AppState>,
    AuthUser(_claims): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let server = state
        .repos
        .servers
        .find_by_id(&id)
        .await?
        .ok_or_else(|| AppError::NotFound("Server not found".into()))?;

    Ok(Json(json!({ "server": server })))
}

pub async fn list_servers(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, AppError> {
    let servers = state.repos.servers.list_for_user(&claims.sub).await?;

    Ok(Json(json!({ "servers": servers })))
}

pub async fn join_server(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    // Verify server exists
    let server = state
        .repos
        .servers
        .find_by_id(&id)
        .await?
        .ok_or_else(|| AppError::NotFound("Server not found".into()))?;

    let was_member = state.repos.servers.is_member(&id, &claims.sub).await?;
    state.repos.servers.add_member(&id, &claims.sub).await?;

    if !was_member {
        state
            .user_connections
            .send_to_user(
                &claims.sub,
                crate::ws::protocol::ServerMessage::ServerJoined {
                    server: serde_json::to_value(&server).unwrap_or(serde_json::Value::Null),
                }
                .to_json(),
            )
            .await;

        let owner_id = server.owner.key().to_string();
        if owner_id != claims.sub {
            if let Some(user) = state.repos.users.find_by_id(&claims.sub).await? {
                state
                    .user_connections
                    .send_to_user(
                        &owner_id,
                        ServerMessage::ServerMemberJoined {
                            server_id: id.clone(),
                            user: NotificationUser::from(&user),
                            ts: now_ms(),
                        }
                        .to_json(),
                    )
                    .await;
            }
        }
    }

    Ok(Json(json!({ "status": "joined" })))
}

pub async fn leave_server(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let server = state
        .repos
        .servers
        .find_by_id(&id)
        .await?
        .ok_or_else(|| AppError::NotFound("Server not found".into()))?;

    let owner_id = server.owner.key().to_string();
    if owner_id == claims.sub {
        return Err(AppError::BadRequest(
            "Server owners cannot leave their own server".into(),
        ));
    }

    if !state.repos.servers.is_member(&id, &claims.sub).await? {
        return Err(AppError::Forbidden("Not a member of this server".into()));
    }

    state.repos.servers.remove_member(&id, &claims.sub).await?;

    if let Some(user) = state.repos.users.find_by_id(&claims.sub).await? {
        state
            .user_connections
            .send_to_user(
                &owner_id,
                ServerMessage::ServerMemberLeft {
                    server_id: id,
                    user: NotificationUser::from(&user),
                    ts: now_ms(),
                }
                .to_json(),
            )
            .await;
    }

    Ok(Json(json!({ "status": "left" })))
}
