use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::Utc;
use uuid::Uuid;

use super::ApiError;
use crate::db;
use crate::state::AppState;
use herald_common::*;

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateCountdownRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate label length
    if req.label.is_empty() {
        return Err(ApiError::BadRequest("label must not be empty".to_string()));
    }
    if req.label.len() > 44 {
        return Err(ApiError::BadRequest(
            "label must be 44 characters or fewer".to_string(),
        ));
    }

    let position = match req.queue_position {
        Some(p) => p,
        None => db::next_queue_position(state.pool()).await?,
    };

    let cd = Countdown {
        id: Uuid::new_v4(),
        label: req.label,
        target: req.target,
        format_template: req.format_template,
        zero_behavior: req.zero_behavior,
        queue_position: position,
        created_at: Utc::now(),
    };

    db::create_countdown(state.pool(), &cd).await?;
    state.notify_board_update().await;
    state.reset_rotation_timer();

    Ok((StatusCode::CREATED, Json(cd)))
}

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<ListResponse<Countdown>>, ApiError> {
    let countdowns = db::list_countdowns(state.pool()).await?;
    let total = countdowns.len();
    Ok(Json(ListResponse {
        items: countdowns,
        total,
    }))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateCountdownRequest>,
) -> Result<Json<Countdown>, ApiError> {
    // Validate label if provided
    if let Some(ref label) = req.label {
        if label.is_empty() {
            return Err(ApiError::BadRequest("label must not be empty".to_string()));
        }
        if label.len() > 44 {
            return Err(ApiError::BadRequest(
                "label must be 44 characters or fewer".to_string(),
            ));
        }
    }

    let updated = db::update_countdown(state.pool(), &id, &req).await?;
    if !updated {
        return Err(ApiError::NotFound(format!("countdown {id} not found")));
    }

    let cd = db::get_countdown(state.pool(), &id)
        .await?
        .ok_or_else(|| ApiError::Internal("countdown disappeared after update".to_string()))?;

    state.notify_board_update().await;

    Ok(Json(cd))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = db::delete_countdown(state.pool(), &id).await?;
    if !deleted {
        return Err(ApiError::NotFound(format!("countdown {id} not found")));
    }
    state.notify_board_update().await;
    state.reset_rotation_timer();
    Ok(StatusCode::NO_CONTENT)
}
