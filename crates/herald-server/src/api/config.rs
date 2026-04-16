use axum::Json;
use axum::extract::State;

use super::ApiError;
use crate::db;
use crate::state::AppState;
use herald_common::UpdateConfigRequest;

pub async fn get(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Map<String, serde_json::Value>>, ApiError> {
    let config = db::get_config(state.pool()).await?;
    Ok(Json(config))
}

pub async fn update(
    State(state): State<AppState>,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<serde_json::Map<String, serde_json::Value>>, ApiError> {
    if req.values.is_empty() {
        return Err(ApiError::BadRequest(
            "request body must contain at least one key-value pair".to_string(),
        ));
    }

    db::set_config(state.pool(), &req.values).await?;
    state.reset_rotation_timer();
    let config = db::get_config(state.pool()).await?;
    Ok(Json(config))
}
