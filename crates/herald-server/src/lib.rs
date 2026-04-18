pub mod api;
pub mod db;
pub mod state;
pub mod ws;

use axum::Router;
use axum::middleware;
use axum::routing::{delete, get, post, put};

use herald_common::{QueueItemKind, ZeroBehavior};
use state::AppState;

/// Spawn the background rotation task that advances the queue on a timer.
///
/// When the current queue item is a countdown, a secondary 1-second refresh
/// interval runs in parallel with the main rotation timer so the board updates
/// every second. When the countdown reaches zero, `ZeroBehavior` determines
/// what happens next (show zero, remove, pause, or show a message).
pub fn start_rotation_task(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("Rotation task started");

        loop {
            let pool = state.pool();

            let interval_secs = match db::get_rotation_interval(pool).await {
                Ok(secs) => secs,
                Err(e) => {
                    tracing::error!("Failed to read rotation interval: {e}");
                    10
                }
            };

            if interval_secs == 0 {
                tracing::debug!("Rotation disabled (interval = 0), waiting for config change");
                state.rotation_notified().await;
                continue;
            }

            let current_item = match db::get_current_queue_item(pool).await {
                Ok(item) => item,
                Err(e) => {
                    tracing::error!("Failed to read current queue item: {e}");
                    tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
                    continue;
                }
            };

            let current_item = match current_item {
                Some(item) => item,
                None => {
                    tracing::trace!("Rotation tick: queue is empty, waiting");
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_secs(interval_secs)) => {}
                        _ = state.rotation_notified() => {
                            tracing::debug!("Rotation timer reset (empty queue)");
                        }
                    }
                    continue;
                }
            };

            match current_item.kind {
                QueueItemKind::Countdown => {
                    run_countdown_mode(&state, interval_secs, &current_item).await;
                }
                QueueItemKind::Message => {
                    run_normal_mode(&state, interval_secs).await;
                }
            }
        }
    })
}

/// Normal rotation mode: sleep for the rotation interval, then advance.
async fn run_normal_mode(state: &AppState, interval_secs: u64) {
    tracing::debug!("Normal mode: next rotation tick in {interval_secs}s");
    let pool = state.pool();

    tokio::select! {
        _ = tokio::time::sleep(std::time::Duration::from_secs(interval_secs)) => {
            match db::get_queue(pool).await {
                Ok(queue) if queue.is_empty() => {
                    tracing::trace!("Rotation tick: queue is empty");
                }
                Ok(queue) if queue.len() == 1 => {
                    match db::delete_expired_messages(pool).await {
                        Ok(0) => {
                            tracing::trace!("Rotation tick: single item, skipping");
                        }
                        Ok(deleted) => {
                            tracing::info!("Rotation: removed {deleted} expired message(s)");
                            state.notify_board_update().await;
                        }
                        Err(e) => tracing::error!("Failed to clean expired messages: {e}"),
                    }
                }
                Ok(_) => {
                    match db::advance_to_next_valid_item(pool).await {
                        Ok(deleted) => {
                            if deleted > 0 {
                                tracing::info!("Rotation: removed {deleted} expired message(s)");
                            }
                            tracing::debug!("Rotation tick: advanced to next item");
                            state.notify_board_update().await;
                        }
                        Err(e) => tracing::error!("Failed to advance rotation: {e}"),
                    }
                }
                Err(e) => tracing::error!("Failed to read queue: {e}"),
            }
        }
        _ = state.rotation_notified() => {
            tracing::debug!("Rotation timer reset");
        }
    }
}

/// Countdown mode: a 1-second refresh interval runs in parallel with the main
/// rotation timer. Each tick broadcasts a board update so viewers see the
/// countdown decrement. When it reaches zero, `ZeroBehavior` takes effect.
async fn run_countdown_mode(state: &AppState, interval_secs: u64, item: &herald_common::QueueItem) {
    let pool = state.pool();
    let item_id = item.id.to_string();

    let countdown = match db::get_countdown(pool, &item_id).await {
        Ok(Some(cd)) => cd,
        Ok(None) => {
            tracing::warn!("Countdown '{item_id}' not found, advancing");
            db::advance_to_next_valid_item(pool).await.ok();
            state.notify_board_update().await;
            return;
        }
        Err(e) => {
            tracing::error!("Failed to load countdown '{item_id}': {e}");
            return;
        }
    };

    tracing::debug!(
        "Countdown mode: '{}' with rotation in {interval_secs}s",
        item.label
    );

    let rotation_sleep = tokio::time::sleep(std::time::Duration::from_secs(interval_secs));
    tokio::pin!(rotation_sleep);

    let mut refresh_interval = tokio::time::interval(std::time::Duration::from_secs(1));
    refresh_interval.tick().await; // consume the immediate first tick

    let mut is_at_zero = false;

    loop {
        tokio::select! {
            _ = &mut rotation_sleep => {
                // Rotation timer fired — advance to next item
                match db::get_queue(pool).await {
                    Ok(queue) if queue.len() <= 1 => {
                        match db::delete_expired_messages(pool).await {
                            Ok(deleted) if deleted > 0 => {
                                tracing::info!("Rotation: removed {deleted} expired message(s)");
                            }
                            Err(e) => tracing::error!("Failed to clean expired messages: {e}"),
                            _ => {}
                        }
                        state.notify_board_update().await;
                    }
                    Ok(_) => {
                        match db::advance_to_next_valid_item(pool).await {
                            Ok(deleted) => {
                                if deleted > 0 {
                                    tracing::info!("Rotation: removed {deleted} expired message(s)");
                                }
                                tracing::debug!("Rotation tick: advanced past countdown");
                                state.notify_board_update().await;
                            }
                            Err(e) => tracing::error!("Failed to advance rotation: {e}"),
                        }
                    }
                    Err(e) => tracing::error!("Failed to read queue: {e}"),
                }
                break;
            }
            _ = refresh_interval.tick() => {
                let now = chrono::Utc::now();

                if !is_at_zero && countdown.target <= now {
                    is_at_zero = true;
                    match &countdown.zero_behavior {
                        ZeroBehavior::ShowZero => {
                            state.notify_board_update().await;
                        }
                        ZeroBehavior::ShowMessage { .. } => {
                            // TODO: ShowMessage grid override is not yet wired
                            // through build_board_state; for now, falls through
                            // to the default zero display (same as ShowZero).
                            state.notify_board_update().await;
                        }
                        ZeroBehavior::Remove => {
                            db::soft_delete_countdown(pool, &item_id).await.ok();
                            tracing::info!(
                                "Countdown '{}' reached zero, removed from queue",
                                item.label
                            );
                            db::advance_to_next_valid_item(pool).await.ok();
                            state.notify_board_update().await;
                            break;
                        }
                        ZeroBehavior::Pause => {
                            tracing::info!(
                                "Countdown '{}' reached zero, pausing rotation",
                                item.label
                            );
                            state.notify_board_update().await;
                            state.rotation_notified().await;
                            break;
                        }
                    }
                } else {
                    state.notify_board_update().await;
                }
            }
            _ = state.rotation_notified() => {
                tracing::debug!("Rotation timer reset during countdown");
                break;
            }
        }
    }
}

/// Spawn a periodic cleanup task that soft-expires items every 60 seconds.
/// If the currently-displayed item expires, resets the rotation timer to trigger an advance.
pub fn start_cleanup_task(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("Cleanup task started (60s interval)");
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.tick().await; // consume immediate first tick

        loop {
            interval.tick().await;

            // Check which item is currently displayed BEFORE cleanup
            let current_item_id = match db::get_current_queue_item(state.pool()).await {
                Ok(Some(item)) => Some(item.id),
                _ => None,
            };

            // Soft-expire all messages past their expires_at
            match db::delete_expired_messages(state.pool()).await {
                Ok(0) => {
                    tracing::trace!("Cleanup: no expired items found");
                }
                Ok(expired) => {
                    tracing::info!("Cleanup: soft-expired {expired} message(s)");

                    // Check if the currently-displayed item was among those expired
                    if let Some(prev_id) = current_item_id {
                        match db::get_current_queue_item(state.pool()).await {
                            Ok(Some(new_item)) if new_item.id != prev_id => {
                                // Current item changed — the displayed item was expired
                                tracing::info!(
                                    "Cleanup: currently displayed item was expired, triggering rotation"
                                );
                                state.reset_rotation_timer();
                            }
                            Ok(None) => {
                                // Queue is now empty
                                tracing::info!("Cleanup: queue is now empty after expiry");
                                state.reset_rotation_timer();
                            }
                            _ => {
                                // Current item unchanged — just broadcast updated state
                                // (queue may have shrunk but current item is still valid)
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Cleanup: failed to expire messages: {e}");
                }
            }
        }
    })
}

/// Build the full Axum router with all routes.
///
/// When `web_dir` is `Some` and the directory exists, static files from that
/// directory are served as a fallback for any path that doesn't match an API or
/// WebSocket route. `index.html` is returned for unknown paths to support SPA
/// client-side routing.
pub fn build_router(state: AppState, web_dir: Option<std::path::PathBuf>) -> Router {
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

    let mut router = Router::new()
        .nest("/api", admin_api)
        .nest("/api", public_api)
        .route("/ws", get(ws::ws_upgrade))
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    // Serve static web assets if web_dir is configured and exists
    if let Some(dir) = web_dir {
        if dir.exists() {
            tracing::info!("Serving web assets from {}", dir.display());
            let serve_dir = tower_http::services::ServeDir::new(&dir)
                .append_index_html_on_directories(true)
                .fallback(tower_http::services::ServeFile::new(dir.join("index.html")));
            router = router.fallback_service(serve_dir);
        } else {
            tracing::warn!(
                "Web directory {} does not exist, static file serving disabled",
                dir.display()
            );
        }
    }

    router
}
