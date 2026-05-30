use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::models::channel::CreateChannel;
use crate::AppState;

const MAX_CHANNEL_NAME_LEN: usize = 64;

pub async fn create_channel(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(server_id): Path<String>,
    Json(mut input): Json<CreateChannel>,
) -> Result<Json<Value>, AppError> {
    // Verify user is the server owner
    let server = state
        .repos
        .servers
        .find_by_id(&server_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Server not found".into()))?;

    let owner_key = server.owner.key().to_string();
    if owner_key != claims.sub {
        return Err(AppError::Forbidden(
            "Only the server owner can create channels".into(),
        ));
    }

    // Validate channel name
    let name = input.name.trim();
    validate_channel_name(name)?;
    input.name = name.to_string();

    let channel = state.repos.channels.create(input, &server_id).await?;

    broadcast_channel_created(&state, &server_id, &channel).await;

    Ok(Json(json!({ "channel": channel })))
}

fn validate_channel_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() || name.len() > MAX_CHANNEL_NAME_LEN {
        return Err(AppError::BadRequest(format!(
            "Channel name must be 1-{MAX_CHANNEL_NAME_LEN} characters"
        )));
    }
    Ok(())
}

async fn broadcast_channel_created(
    state: &AppState,
    server_id: &str,
    channel: &crate::models::channel::Channel,
) {
    let payload = crate::ws::protocol::ServerMessage::ChannelCreated {
        server_id: server_id.to_string(),
        channel: serde_json::to_value(channel).unwrap_or(serde_json::Value::Null),
    }
    .to_json();

    let member_ids = match state.repos.servers.list_member_ids(server_id).await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::warn!(
                server_id = %server_id,
                error = %e,
                "failed to list server members for channel_created fanout"
            );
            return;
        }
    };

    for member_id in member_ids {
        state
            .user_connections
            .send_to_user(&member_id, payload.clone())
            .await;
    }
}

pub async fn get_channels(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    // Verify user is a member of the server
    if !state
        .repos
        .servers
        .is_member(&server_id, &claims.sub)
        .await?
    {
        return Err(AppError::Forbidden("Not a member of this server".into()));
    }

    let channels = state.repos.channels.list_for_server(&server_id).await?;

    Ok(Json(json!({ "channels": channels })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_channel_name_accepts_db_limit() {
        assert!(validate_channel_name(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn validate_channel_name_rejects_empty_after_trim() {
        let err = validate_channel_name("").expect_err("empty should fail");
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn validate_channel_name_rejects_over_db_limit() {
        let err = validate_channel_name(&"a".repeat(65)).expect_err("oversize should fail");
        assert!(matches!(err, AppError::BadRequest(_)));
    }
}
