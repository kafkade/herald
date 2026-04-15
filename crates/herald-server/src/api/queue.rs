use axum::Json;
use axum::extract::State;

use super::ApiError;
use crate::db;
use crate::state::AppState;
use herald_common::*;

pub async fn list(State(state): State<AppState>) -> Result<Json<QueueListResponse>, ApiError> {
    let items = db::get_queue(state.pool()).await?;
    let current_index = db::get_current_index(state.pool()).await?;
    let total = items.len();

    Ok(Json(QueueListResponse {
        items,
        total,
        current_index,
    }))
}

pub async fn reorder(
    State(state): State<AppState>,
    Json(req): Json<ReorderQueueRequest>,
) -> Result<Json<QueueListResponse>, ApiError> {
    // Validate: the order must contain all current queue item IDs
    let current_items = db::get_queue(state.pool()).await?;
    let current_ids: std::collections::HashSet<uuid::Uuid> =
        current_items.iter().map(|i| i.id).collect();
    let request_ids: std::collections::HashSet<uuid::Uuid> = req.order.iter().copied().collect();

    if current_ids != request_ids {
        return Err(ApiError::BadRequest(
            "order must contain exactly all current queue item IDs".to_string(),
        ));
    }

    db::reorder_queue(state.pool(), &req.order).await?;

    // Return the updated queue
    let items = db::get_queue(state.pool()).await?;
    let current_index = db::get_current_index(state.pool()).await?;
    let total = items.len();

    Ok(Json(QueueListResponse {
        items,
        total,
        current_index,
    }))
}
