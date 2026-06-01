use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::models::direct::OpenDirectMessageInput;
use crate::repositories::Repos;

pub async fn list_direct_messages(
    State(repos): State<Repos>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, AppError> {
    let dms = repos
        .direct_messages
        .list_for_user(&claims.sub, repos.social.as_ref())
        .await?;

    Ok(Json(json!({ "dms": dms })))
}

pub async fn open_direct_message(
    State(repos): State<Repos>,
    AuthUser(claims): AuthUser,
    Json(input): Json<OpenDirectMessageInput>,
) -> Result<Json<Value>, AppError> {
    let dm = repos
        .direct_messages
        .open(&claims.sub, &input.user_id, repos.social.as_ref())
        .await?;

    Ok(Json(json!({ "dm": dm })))
}
