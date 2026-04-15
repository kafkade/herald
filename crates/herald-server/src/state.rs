use std::sync::Arc;
use std::time::Instant;

use sqlx::SqlitePool;

/// Shared application state, wrapped in Arc for cheap cloning.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<InnerState>,
}

struct InnerState {
    pub pool: SqlitePool,
    pub admin_token: String,
    pub started_at: Instant,
}

impl AppState {
    pub fn new(pool: SqlitePool, admin_token: String) -> Self {
        Self {
            inner: Arc::new(InnerState {
                pool,
                admin_token,
                started_at: Instant::now(),
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
}
