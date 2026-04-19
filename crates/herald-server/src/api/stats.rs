use axum::Json;
use axum::extract::State;

use crate::db;
use crate::state::AppState;
use herald_common::StatsResponse;

pub async fn get(State(state): State<AppState>) -> Json<StatsResponse> {
    let total_messages = db::count_messages(state.pool()).await.unwrap_or(0);
    let total_countdowns = db::count_countdowns(state.pool()).await.unwrap_or(0);

    Json(StatsResponse {
        connected_viewers: state.viewer_count(),
        uptime_secs: state.uptime_seconds(),
        total_messages,
        total_countdowns,
    })
}
