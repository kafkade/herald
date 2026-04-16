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
    Json(req): Json<CreateMessageRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Resolve grid from text or raw grid (exactly one must be provided)
    let (grid, source_text) = match (req.text, req.grid) {
        (Some(text), None) => {
            let grid =
                Grid::from_text(&text, req.h_align, req.v_align).map_err(ApiError::BadRequest)?;
            (grid, Some(text))
        }
        (None, Some(grid)) => {
            grid.validate().map_err(ApiError::BadRequest)?;
            (grid, None)
        }
        (Some(_), Some(_)) => {
            return Err(ApiError::BadRequest(
                "provide either 'text' or 'grid', not both".to_string(),
            ));
        }
        (None, None) => {
            return Err(ApiError::BadRequest(
                "provide either 'text' or 'grid'".to_string(),
            ));
        }
    };

    let position = match req.queue_position {
        Some(p) => p,
        None => db::next_queue_position(state.pool()).await?,
    };

    let msg = Message {
        id: Uuid::new_v4(),
        grid,
        source_text,
        h_align: req.h_align,
        v_align: req.v_align,
        queue_position: position,
        created_at: Utc::now(),
        expires_at: req.expires_at,
    };

    db::create_message(state.pool(), &msg).await?;
    state.notify_board_update().await;
    state.reset_rotation_timer();

    Ok((StatusCode::CREATED, Json(msg)))
}

pub async fn list(State(state): State<AppState>) -> Result<Json<ListResponse<Message>>, ApiError> {
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
    // Cannot provide both text and grid
    if req.text.is_some() && req.grid.is_some() {
        return Err(ApiError::BadRequest(
            "provide either 'text' or 'grid', not both".to_string(),
        ));
    }

    // Validate grid if provided directly
    if let Some(ref grid) = req.grid {
        grid.validate().map_err(ApiError::BadRequest)?;
    }

    // If text is provided, or alignment changed with existing source_text, recompute grid
    let existing = db::get_message(state.pool(), &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("message {id} not found")))?;

    let resolved = resolve_update_content(&existing, &req)?;

    let updated = db::update_message(state.pool(), &id, &req, resolved.as_ref()).await?;
    if !updated {
        return Err(ApiError::NotFound(format!("message {id} not found")));
    }

    let msg = db::get_message(state.pool(), &id)
        .await?
        .ok_or_else(|| ApiError::Internal("message disappeared after update".to_string()))?;

    state.notify_board_update().await;

    Ok(Json(msg))
}

/// Resolved content for an update (grid + source_text).
pub struct ResolvedContent {
    pub grid: Grid,
    pub source_text: Option<String>,
}

/// If the update changes text or alignment for a text-based message, recompute the grid.
fn resolve_update_content(
    existing: &Message,
    req: &UpdateMessageRequest,
) -> Result<Option<ResolvedContent>, ApiError> {
    if let Some(ref text) = req.text {
        // Explicit text update — recompute grid
        let h_align = req.h_align.unwrap_or(existing.h_align);
        let v_align = req.v_align.unwrap_or(existing.v_align);
        let grid = Grid::from_text(text, h_align, v_align).map_err(ApiError::BadRequest)?;
        return Ok(Some(ResolvedContent {
            grid,
            source_text: Some(text.clone()),
        }));
    }

    if let Some(ref grid) = req.grid {
        // Explicit grid update — clear source_text
        return Ok(Some(ResolvedContent {
            grid: grid.clone(),
            source_text: None,
        }));
    }

    // Alignment-only update: reflow if source_text exists
    if (req.h_align.is_some() || req.v_align.is_some()) && existing.source_text.is_some() {
        let text = existing.source_text.as_ref().unwrap();
        let h_align = req.h_align.unwrap_or(existing.h_align);
        let v_align = req.v_align.unwrap_or(existing.v_align);
        let grid = Grid::from_text(text, h_align, v_align).map_err(ApiError::BadRequest)?;
        return Ok(Some(ResolvedContent {
            grid,
            source_text: Some(text.clone()),
        }));
    }

    Ok(None)
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = db::delete_message(state.pool(), &id).await?;
    if !deleted {
        return Err(ApiError::NotFound(format!("message {id} not found")));
    }
    state.notify_board_update().await;
    state.reset_rotation_timer();
    Ok(StatusCode::NO_CONTENT)
}
