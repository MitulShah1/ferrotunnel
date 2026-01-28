//! Circuit breaker plugin for fail-fast behavior

use crate::traits::{Plugin, PluginAction, RequestContext, ResponseContext};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Duration;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation, requests pass through
    Closed,
    /// Circuit is open, requests are rejected
    Open,
    /// Testing if service has recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Duration to keep circuit open before testing
    pub open_duration: Duration,
    /// Number of successful requests in half-open to close circuit
    pub half_open_success_threshold: u32,
    /// HTTP status codes considered as failures
    pub failure_status_codes: Vec<u16>,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            open_duration: Duration::from_secs(30),
            half_open_success_threshold: 3,
            failure_status_codes: vec![500, 502, 503, 504],
        }
    }
}

/// Circuit breaker plugin
pub struct CircuitBreakerPlugin {
    config: CircuitBreakerConfig,
    state: RwLock<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: AtomicU64,
}

impl CircuitBreakerPlugin {
    /// Create a new circuit breaker plugin
    #[must_use]
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
        }
    }

    /// Get current circuit state
    #[must_use]
    pub fn state(&self) -> CircuitState {
        *self.state.read().unwrap_or_else(|e| e.into_inner())
    }

    /// Check if request should be allowed
    fn should_allow(&self) -> bool {
        let state = self.state();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let elapsed = now.saturating_sub(last_failure);

                if elapsed >= self.config.open_duration.as_secs() {
                    if let Ok(mut guard) = self.state.write() {
                        if *guard == CircuitState::Open {
                            *guard = CircuitState::HalfOpen;
                            self.success_count.store(0, Ordering::Relaxed);
                        }
                    }
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful request
    fn record_success(&self) {
        let state = self.state();

        match state {
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.config.half_open_success_threshold {
                    if let Ok(mut guard) = self.state.write() {
                        *guard = CircuitState::Closed;
                        self.failure_count.store(0, Ordering::Relaxed);
                    }
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Record a failed request
    fn record_failure(&self) {
        let state = self.state();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_failure_time.store(now, Ordering::Relaxed);

        match state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.config.failure_threshold {
                    if let Ok(mut guard) = self.state.write() {
                        *guard = CircuitState::Open;
                    }
                }
            }
            CircuitState::HalfOpen => {
                if let Ok(mut guard) = self.state.write() {
                    *guard = CircuitState::Open;
                    self.success_count.store(0, Ordering::Relaxed);
                }
            }
            CircuitState::Open => {}
        }
    }

    fn is_failure_status(&self, status: u16) -> bool {
        self.config.failure_status_codes.contains(&status)
    }
}

impl std::fmt::Debug for CircuitBreakerPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CircuitBreakerPlugin")
            .field("state", &self.state())
            .field("failure_count", &self.failure_count.load(Ordering::Relaxed))
            .finish()
    }
}

#[async_trait]
impl Plugin for CircuitBreakerPlugin {
    fn name(&self) -> &str {
        "circuit-breaker"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    async fn on_request(
        &self,
        _req: &mut http::Request<()>,
        _ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        if self.should_allow() {
            Ok(PluginAction::Continue)
        } else {
            Ok(PluginAction::Reject {
                status: 503,
                reason: "Service temporarily unavailable (circuit breaker open)".into(),
            })
        }
    }

    async fn on_response(
        &self,
        res: &mut http::Response<Vec<u8>>,
        _ctx: &ResponseContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let status = res.status().as_u16();

        if self.is_failure_status(status) {
            self.record_failure();
        } else {
            self.record_success();
        }

        Ok(PluginAction::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_starts_closed() {
        let plugin = CircuitBreakerPlugin::new(CircuitBreakerConfig::default());
        assert_eq!(plugin.state(), CircuitState::Closed);
        assert!(plugin.should_allow());
    }

    #[test]
    fn test_circuit_opens_after_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let plugin = CircuitBreakerPlugin::new(config);

        plugin.record_failure();
        plugin.record_failure();
        assert_eq!(plugin.state(), CircuitState::Closed);

        plugin.record_failure();
        assert_eq!(plugin.state(), CircuitState::Open);
    }

    #[test]
    fn test_success_resets_failure_count() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let plugin = CircuitBreakerPlugin::new(config);

        plugin.record_failure();
        plugin.record_failure();
        plugin.record_success();
        plugin.record_failure();
        plugin.record_failure();

        assert_eq!(plugin.state(), CircuitState::Closed);
    }
}
