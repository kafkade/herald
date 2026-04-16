pub mod api;
pub mod db;
pub mod state;
pub mod ws;

use axum::Router;
use axum::middleware;
use axum::routing::{delete, get, post, put};

use state::AppState;

/// Spawn the background rotation task that advances the queue on a timer.
/// Returns the JoinHandle for the spawned task.
pub fn start_rotation_task(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("Rotation task started");

        loop {
            let interval_secs = match db::get_rotation_interval(state.pool()).await {
                Ok(secs) => secs,
                Err(e) => {
                    tracing::error!("Failed to read rotation interval: {e}");
                    10 // fallback default
                }
            };

            if interval_secs == 0 {
                tracing::debug!("Rotation disabled (interval = 0), waiting for config change");
                state.rotation_notified().await;
                continue;
            }

            tracing::debug!("Next rotation tick in {interval_secs}s");

            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(interval_secs)) => {
                    match db::get_queue(state.pool()).await {
                        Ok(queue) if queue.is_empty() => {
                            tracing::trace!("Rotation tick: queue is empty, nothing to rotate");
                        }
                        Ok(queue) if queue.len() == 1 => {
                            // Single item — don't advance, but do clean up expired items
                            // (if the single item is expired, this will remove it)
                            match db::delete_expired_messages(state.pool()).await {
                                Ok(0) => {
                                    tracing::trace!("Rotation tick: single item in queue, skipping rotation");
                                }
                                Ok(deleted) => {
                                    tracing::info!("Rotation: removed {deleted} expired message(s), queue may now be empty");
                                    state.notify_board_update().await;
                                }
                                Err(e) => {
                                    tracing::error!("Failed to clean expired messages: {e}");
                                }
                            }
                        }
                        Ok(_) => {
                            // Multiple items — advance with expiry handling
                            match db::advance_to_next_valid_item(state.pool()).await {
                                Ok(deleted) => {
                                    if deleted > 0 {
                                        tracing::info!("Rotation: removed {deleted} expired message(s)");
                                    }
                                    tracing::debug!("Rotation tick: advanced to next item");
                                    state.notify_board_update().await;
                                }
                                Err(e) => {
                                    tracing::error!("Failed to advance rotation: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to read queue for rotation: {e}");
                        }
                    }
                }
                _ = state.rotation_notified() => {
                    tracing::debug!("Rotation timer reset");
                    continue;
                }
            }
        }
    })
}

/// Build the full Axum router with all routes.
pub fn build_router(state: AppState) -> Router {
    // Admin routes — protected by bearer token auth
    let admin_api = Router::new()
        // Messages
        .route("/messages", post(api::messages::create))
        .route("/messages", get(api::messages::list))
        .route("/messages/{id}", get(api::messages::get))
        .route("/messages/{id}", put(api::messages::update))
        .route("/messages/{id}", delete(api::messages::delete))
        // Countdowns
        .route("/countdowns", post(api::countdowns::create))
        .route("/countdowns", get(api::countdowns::list))
        .route("/countdowns/{id}", put(api::countdowns::update))
        .route("/countdowns/{id}", delete(api::countdowns::delete))
        // Queue
        .route("/queue", get(api::queue::list))
        .route("/queue/reorder", put(api::queue::reorder))
        // Config
        .route("/config", get(api::config::get))
        .route("/config", put(api::config::update))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            api::auth::require_auth,
        ));

    // Public routes — no auth required
    let public_api = Router::new().route("/health", get(api::health::health));

    Router::new()
        .nest("/api", admin_api)
        .nest("/api", public_api)
        .route("/ws", get(ws::ws_upgrade))
        .with_state(state)
}
