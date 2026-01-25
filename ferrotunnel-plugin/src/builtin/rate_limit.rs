use crate::traits::*;
use async_trait::async_trait;
use governor::{clock::DefaultClock, state::keyed::DefaultKeyedStateStore, Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Rate limiting plugin using token bucket algorithm
pub struct RateLimitPlugin {
    limiter: Arc<RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>>,
}

impl RateLimitPlugin {
    /// Create rate limiter allowing `requests_per_second` per client IP
    pub fn new(requests_per_second: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap());
        Self {
            limiter: Arc::new(RateLimiter::keyed(quota)),
        }
    }
}

#[async_trait]
impl Plugin for RateLimitPlugin {
    fn name(&self) -> &str {
        "rate-limit"
    }

    async fn on_request(
        &self,
        _req: &mut http::Request<Vec<u8>>,
        ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let key = ctx.remote_addr.ip().to_string();

        match self.limiter.check_key(&key) {
            Ok(_) => Ok(PluginAction::Continue),
            Err(_) => Ok(PluginAction::Reject {
                status: 429,
                reason: "Rate limit exceeded".to_string(),
            }),
        }
    }
}
