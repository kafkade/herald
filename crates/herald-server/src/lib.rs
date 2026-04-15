pub mod api;
pub mod db;
pub mod state;

use axum::middleware;
use axum::routing::{delete, get, post, put};
use axum::Router;

use state::AppState;

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
        .with_state(state)
}
