use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use super::ApiError;
use crate::db;
use crate::state::AppState;
use herald_common::*;

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateMessageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate grid dimensions
    req.grid
        .validate()
        .map_err(|e| ApiError::BadRequest(e))?;

    let position = match req.queue_position {
        Some(p) => p,
        None => db::next_queue_position(state.pool()).await?,
    };

    let msg = Message {
        id: Uuid::new_v4(),
        grid: req.grid,
        h_align: req.h_align,
        v_align: req.v_align,
        queue_position: position,
        created_at: Utc::now(),
        expires_at: req.expires_at,
    };

    db::create_message(state.pool(), &msg).await?;

    Ok((StatusCode::CREATED, Json(msg)))
}

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<ListResponse<Message>>, ApiError> {
    let messages = db::list_messages(state.pool()).await?;
    let total = messages.len();
    Ok(Json(ListResponse {
        items: messages,
        total,
    }))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Message>, ApiError> {
    let msg = db::get_message(state.pool(), &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("message {id} not found")))?;
    Ok(Json(msg))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateMessageRequest>,
) -> Result<Json<Message>, ApiError> {
    // Validate grid if provided
    if let Some(ref grid) = req.grid {
        grid.validate().map_err(|e| ApiError::BadRequest(e))?;
    }

    let updated = db::update_message(state.pool(), &id, &req).await?;
    if !updated {
        return Err(ApiError::NotFound(format!("message {id} not found")));
    }

    let msg = db::get_message(state.pool(), &id)
        .await?
        .ok_or_else(|| ApiError::Internal("message disappeared after update".to_string()))?;

    Ok(Json(msg))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = db::delete_message(state.pool(), &id).await?;
    if !deleted {
        return Err(ApiError::NotFound(format!("message {id} not found")));
    }
    Ok(StatusCode::NO_CONTENT)
}
