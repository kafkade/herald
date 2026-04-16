use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use herald_common::{Grid, ServerMessage};
use sqlx::SqlitePool;
use tokio::sync::{Mutex, Notify, broadcast};

const BROADCAST_CAPACITY: usize = 16;

/// Shared application state, wrapped in Arc for cheap cloning.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<InnerState>,
}

struct InnerState {
    pub pool: SqlitePool,
    pub admin_token: String,
    pub started_at: Instant,
    pub broadcast_tx: broadcast::Sender<ServerMessage>,
    pub viewer_count: AtomicUsize,
    pub next_client_id: AtomicU64,
    /// Guards the build-board-state → broadcast path so concurrent mutations
    /// cannot interleave and produce out-of-order previous_grid values.
    pub notify_lock: Mutex<Grid>,
    /// Signals the rotation task to restart its timer (e.g., after config change or queue mutation).
    pub rotation_notify: Notify,
}

impl AppState {
    pub fn new(pool: SqlitePool, admin_token: String) -> Self {
        let (broadcast_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            inner: Arc::new(InnerState {
                pool,
                admin_token,
                started_at: Instant::now(),
                broadcast_tx,
                viewer_count: AtomicUsize::new(0),
                next_client_id: AtomicU64::new(1),
                notify_lock: Mutex::new(Grid::blank()),
                rotation_notify: Notify::new(),
            }),
        }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.inner.pool
    }

    pub fn admin_token(&self) -> &str {
        &self.inner.admin_token
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.inner.started_at.elapsed().as_secs()
    }

    /// Get a reference to the broadcast sender (for sending board updates).
    pub fn broadcast_tx(&self) -> &broadcast::Sender<ServerMessage> {
        &self.inner.broadcast_tx
    }

    /// Subscribe to the broadcast channel (for new WS connections).
    pub fn subscribe_broadcast(&self) -> broadcast::Receiver<ServerMessage> {
        self.inner.broadcast_tx.subscribe()
    }

    /// Increment viewer count and return the new count.
    pub fn add_viewer(&self) -> usize {
        self.inner.viewer_count.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Decrement viewer count and return the new count.
    pub fn remove_viewer(&self) -> usize {
        self.inner.viewer_count.fetch_sub(1, Ordering::Relaxed) - 1
    }

    /// Get the current viewer count.
    pub fn viewer_count(&self) -> usize {
        self.inner.viewer_count.load(Ordering::Relaxed)
    }

    /// Generate a unique client identifier for logging.
    pub fn next_client_id(&self) -> u64 {
        self.inner.next_client_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Build the current board state from the database and broadcast it to all connected viewers.
    /// The entire build → broadcast path is serialized via a Mutex so concurrent
    /// mutations cannot produce out-of-order `previous_grid` values.
    pub async fn notify_board_update(&self) {
        let mut last_grid = self.inner.notify_lock.lock().await;

        match crate::db::build_board_state(self.pool()).await {
            Ok(mut board_state) => {
                board_state.previous_grid = last_grid.clone();
                *last_grid = board_state.grid.clone();
                drop(last_grid);

                let _ = self
                    .inner
                    .broadcast_tx
                    .send(herald_common::ServerMessage::BoardUpdate(board_state));
            }
            Err(e) => {
                drop(last_grid);
                tracing::error!("Failed to build board state for broadcast: {e}");
            }
        }
    }

    /// Signal the rotation task to reset its timer and re-read the interval.
    /// Call this after config changes or queue mutations.
    pub fn reset_rotation_timer(&self) {
        self.inner.rotation_notify.notify_one();
    }

    /// Wait for a rotation timer reset signal.
    /// Used by the rotation task in its select loop.
    pub async fn rotation_notified(&self) {
        self.inner.rotation_notify.notified().await;
    }
}
