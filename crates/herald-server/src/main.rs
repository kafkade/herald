use std::env;

use herald_server::{build_router, db, state::AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("HERALD_LOG_LEVEL").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Read configuration from env
    let db_path = env::var("HERALD_DB_PATH").unwrap_or_else(|_| "herald.db".to_string());
    let database_url = format!("sqlite:{db_path}");

    let admin_token = env::var("HERALD_ADMIN_TOKEN").unwrap_or_else(|_| {
        let token = uuid::Uuid::new_v4().to_string();
        tracing::warn!("No HERALD_ADMIN_TOKEN set. Generated token: {token}");
        token
    });

    let port: u16 = env::var("HERALD_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    // Initialize database
    let pool = db::init_pool(&database_url)
        .await
        .expect("failed to initialize database");

    tracing::info!("Database initialized at {db_path}");

    // Build app
    let state = AppState::new(pool, admin_token);
    let app = build_router(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind port");

    tracing::info!("Herald server listening on port {port}");

    axum::serve(listener, app).await.expect("server error");
}
