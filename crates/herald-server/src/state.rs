use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use herald_common::ServerMessage;
use sqlx::SqlitePool;
use tokio::sync::broadcast;

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
}
