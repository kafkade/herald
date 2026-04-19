use std::env;

use herald_server::{build_router, db, state::AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize logging
    let log_filter =
        EnvFilter::try_from_env("HERALD_LOG_LEVEL").unwrap_or_else(|_| EnvFilter::new("info"));
    let log_format = env::var("HERALD_LOG_FORMAT").unwrap_or_default();

    if log_format == "json" {
        tracing_subscriber::fmt()
            .with_env_filter(log_filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(log_filter).init();
    }

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

    // Resolve web assets directory
    let web_dir = env::var("HERALD_WEB_DIR")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let default = std::path::PathBuf::from("./web-dist");
            if default.exists() {
                Some(default)
            } else {
                None
            }
        });

    // Build app
    let state = AppState::new(pool, admin_token);
    herald_server::start_rotation_task(state.clone());
    herald_server::start_cleanup_task(state.clone());
    herald_server::start_schedule_task(state.clone());
    let app = build_router(state.clone(), web_dir);

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind port");

    tracing::info!("Herald server listening on port {port}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(state.clone()))
        .await
        .expect("server error");

    // Post-shutdown cleanup
    tracing::info!("Shutting down — closing database connection pool");
    state.pool().close().await;
    tracing::info!("Herald server stopped");
}

/// Wait for SIGTERM or SIGINT (Ctrl+C), then initiate graceful shutdown.
async fn shutdown_signal(state: AppState) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received SIGINT (Ctrl+C)"),
        _ = terminate => tracing::info!("Received SIGTERM"),
    }

    tracing::info!("Initiating graceful shutdown — notifying WebSocket clients");

    // Broadcast shutdown message to all connected WebSocket clients
    let shutdown_msg = herald_common::ServerMessage::Shutdown {
        reason: "Server is shutting down".to_string(),
    };
    state.broadcast(shutdown_msg);
}
