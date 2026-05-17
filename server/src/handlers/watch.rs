use axum::extract::{Path, Query, State};
use axum::Json;
use deadpool_redis::redis::AsyncCommands;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::models::channel::ChannelType;
use crate::repositories::Repos;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct RecommendationsQuery {
    /// Capped to a small range to keep the graph traversal cheap.
    pub limit: Option<u32>,
}

const REC_CACHE_TTL_SECS: u64 = 60;
const REC_DEFAULT_LIMIT: u32 = 10;
const REC_MAX_LIMIT: u32 = 50;

/// `GET /api/channels/:channel_id/watch/recommendations?limit=10` — surfaces
/// videos that other members of the user's servers have watched but the user
/// hasn't. Cached in Redis for 60s per user to amortize the 2-hop traversal.
pub async fn get_recommendations(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(channel_id): Path<String>,
    Query(query): Query<RecommendationsQuery>,
) -> Result<Json<Value>, AppError> {
    authorize_watch_member(&state.repos, &channel_id, &claims.sub).await?;

    let limit = query
        .limit
        .unwrap_or(REC_DEFAULT_LIMIT)
        .clamp(1, REC_MAX_LIMIT);

    let cache_key = format!("rec:user:{}:{limit}", claims.sub);

    // Cache read — failures fall through to a fresh query rather than 500ing
    // the user. The recommendation feature degrades gracefully if Redis hiccups.
    if let Ok(mut conn) = state.redis.get().await {
        match conn.get::<_, Option<String>>(&cache_key).await {
            Ok(Some(json)) => {
                if let Ok(value) = serde_json::from_str::<Value>(&json) {
                    return Ok(Json(value));
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::debug!(error = %e, "rec cache read failed; recomputing");
            }
        }
    }

    let recs = state
        .repos
        .recommendations
        .for_user(&claims.sub, limit)
        .await?;

    let payload = json!({
        "channel_id": channel_id,
        "recommendations": recs,
    });

    // Cache write — best-effort; ignore errors so a Redis blip doesn't block
    // the response. The TTL prevents unbounded staleness.
    if let Ok(mut conn) = state.redis.get().await {
        if let Ok(json_str) = serde_json::to_string(&payload) {
            let _: Result<(), _> = conn.set_ex(&cache_key, json_str, REC_CACHE_TTL_SECS).await;
        }
    }

    Ok(Json(payload))
}

/// Same fail-closed shape as `authorize_member` in whiteboards.rs — single
/// generic Forbidden whether the channel is missing, the wrong type, or the
/// user isn't a member. Details only in trace logs.
async fn authorize_watch_member(
    repos: &Repos,
    channel_id: &str,
    user_id: &str,
) -> Result<(), AppError> {
    let channel = match repos.channels.find_by_id(channel_id).await? {
        Some(c) => c,
        None => {
            tracing::debug!(channel_id = %channel_id, user_id = %user_id, "watch auth: channel missing");
            return Err(AppError::Forbidden("Not authorized for this watch room".into()));
        }
    };
    if !matches!(channel.channel_type, ChannelType::Watch) {
        tracing::debug!(channel_id = %channel_id, user_id = %user_id, "watch auth: wrong channel type");
        return Err(AppError::Forbidden("Not authorized for this watch room".into()));
    }
    let server_key = channel.server.key().to_string();
    let is_member = repos.servers.is_member(&server_key, user_id).await?;
    if !is_member {
        tracing::debug!(channel_id = %channel_id, user_id = %user_id, "watch auth: non-member");
        return Err(AppError::Forbidden("Not authorized for this watch room".into()));
    }
    Ok(())
}
