use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use herald_common::ErrorResponse;

/// Simple sliding-window rate limiter.
///
/// Tracks request timestamps within a rolling window and rejects new
/// requests once the count exceeds `max_requests`.
pub struct RateLimiter {
    max_requests: u32,
    window: std::time::Duration,
    requests: std::sync::Mutex<Vec<Instant>>,
}

impl RateLimiter {
    pub fn new(max_per_minute: u32) -> Self {
        Self {
            max_requests: max_per_minute,
            window: std::time::Duration::from_secs(60),
            requests: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Returns `true` if the request is allowed, `false` if rate-limited.
    pub fn check(&self) -> bool {
        let now = Instant::now();
        let mut requests = self.requests.lock().unwrap();

        // Evict entries outside the sliding window
        let cutoff = now - self.window;
        requests.retain(|&t| t > cutoff);

        if (requests.len() as u32) < self.max_requests {
            requests.push(now);
            true
        } else {
            false
        }
    }
}

/// Axum middleware that enforces rate limiting via a shared [`RateLimiter`].
pub async fn rate_limit(
    State(limiter): State<Arc<RateLimiter>>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    if limiter.check() {
        next.run(request).await
    } else {
        tracing::warn!(
            "Rate limit exceeded: {} {}",
            request.method(),
            request.uri()
        );

        (
            StatusCode::TOO_MANY_REQUESTS,
            [("retry-after", "60")],
            Json(ErrorResponse {
                error: "too_many_requests".to_string(),
                message: "rate limit exceeded — try again later".to_string(),
            }),
        )
            .into_response()
    }
}
