use axum::extract::State;
use axum::Json;

use crate::db;
use crate::state::AppState;
use herald_common::HealthResponse;

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let queue_size = db::queue_size(state.pool()).await.unwrap_or(0);

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: state.uptime_seconds(),
        queue_size,
    })
}
