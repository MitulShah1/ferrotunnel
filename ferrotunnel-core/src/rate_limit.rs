//! Rate limiting for tunnel sessions

use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Session rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum streams opened per second
    pub streams_per_sec: NonZeroU32,
    /// Maximum bytes per second
    pub bytes_per_sec: NonZeroU32,
    /// Burst multiplier
    pub burst_factor: NonZeroU32,
}

impl Default for RateLimiterConfig {
    #[allow(clippy::expect_used)]
    fn default() -> Self {
        Self {
            streams_per_sec: NonZeroU32::new(100).expect("100 is non-zero"),
            bytes_per_sec: NonZeroU32::new(10 * 1024 * 1024).expect("10MB is non-zero"),
            burst_factor: NonZeroU32::new(2).expect("2 is non-zero"),
        }
    }
}

/// Rate limiter for a single session
#[derive(Clone)]
pub struct SessionRateLimiter {
    /// Limits stream open rate
    stream_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    /// Limits data throughput
    bytes_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
}

impl SessionRateLimiter {
    /// Create a new session rate limiter
    #[must_use]
    pub fn new(config: &RateLimiterConfig) -> Self {
        let stream_quota =
            Quota::per_second(config.streams_per_sec).allow_burst(config.burst_factor);
        let bytes_quota = Quota::per_second(config.bytes_per_sec).allow_burst(config.burst_factor);

        Self {
            stream_limiter: Arc::new(RateLimiter::direct(stream_quota)),
            bytes_limiter: Arc::new(RateLimiter::direct(bytes_quota)),
        }
    }

    /// Check if a new stream can be opened
    /// Returns Ok(()) if allowed, Err if rate limited
    pub fn check_stream_open(&self) -> Result<(), RateLimitError> {
        self.stream_limiter
            .check()
            .map_err(|_| RateLimitError::StreamRateLimited)
    }

    /// Check if data can be sent (by byte count)
    /// For simplicity, we check once per frame rather than exact bytes
    pub fn check_data(&self, _bytes: usize) -> Result<(), RateLimitError> {
        self.bytes_limiter
            .check()
            .map_err(|_| RateLimitError::BytesRateLimited)
    }

    /// Async version - waits until allowed
    pub async fn wait_for_stream_open(&self) {
        self.stream_limiter.until_ready().await;
    }

    /// Async version - waits until data can be sent
    pub async fn wait_for_data(&self, _bytes: usize) {
        self.bytes_limiter.until_ready().await;
    }
}

impl std::fmt::Debug for SessionRateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionRateLimiter").finish_non_exhaustive()
    }
}

/// Rate limit errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum RateLimitError {
    #[error("stream open rate limited")]
    StreamRateLimited,
    #[error("data transfer rate limited")]
    BytesRateLimited,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let config = RateLimiterConfig::default();
        let limiter = SessionRateLimiter::new(&config);

        // Should allow first stream
        assert!(limiter.check_stream_open().is_ok());
    }

    #[test]
    fn test_burst_allowance() {
        let config = RateLimiterConfig {
            streams_per_sec: NonZeroU32::new(1).unwrap(),
            bytes_per_sec: NonZeroU32::new(1000).unwrap(),
            burst_factor: NonZeroU32::new(5).unwrap(),
        };
        let limiter = SessionRateLimiter::new(&config);

        // Burst should allow multiple quick opens
        for _ in 0..5 {
            assert!(limiter.check_stream_open().is_ok());
        }
        // Should be rate limited after burst
        assert!(limiter.check_stream_open().is_err());
    }
}
