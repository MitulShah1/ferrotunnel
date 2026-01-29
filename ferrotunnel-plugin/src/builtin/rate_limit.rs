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
        _req: &mut http::Request<()>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(ip: &str) -> RequestContext {
        RequestContext {
            tunnel_id: "test".into(),
            session_id: "session".into(),
            remote_addr: ip.parse().unwrap(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    #[tokio::test]
    async fn test_rate_limit_allows_first_request() {
        let plugin = RateLimitPlugin::new(10);
        let mut req = http::Request::builder().body(()).unwrap();
        let ctx = make_ctx("192.168.1.1:12345");

        let action = plugin.on_request(&mut req, &ctx).await.unwrap();
        assert_eq!(action, PluginAction::Continue);
    }

    #[tokio::test]
    async fn test_rate_limit_rejects_after_limit() {
        // Create a limiter with 1 request per second
        let plugin = RateLimitPlugin::new(1);
        let ctx = make_ctx("10.0.0.1:8080");

        // First request should pass
        let mut req1 = http::Request::builder().body(()).unwrap();
        let action1 = plugin.on_request(&mut req1, &ctx).await.unwrap();
        assert_eq!(action1, PluginAction::Continue);

        // Second immediate request should be rejected
        let mut req2 = http::Request::builder().body(()).unwrap();
        let action2 = plugin.on_request(&mut req2, &ctx).await.unwrap();
        match action2 {
            PluginAction::Reject { status, .. } => assert_eq!(status, 429),
            _ => panic!("Expected Reject"),
        }
    }

    #[tokio::test]
    async fn test_rate_limit_separate_limits_per_ip() {
        let plugin = RateLimitPlugin::new(1);

        // First client
        let ctx1 = make_ctx("1.1.1.1:1000");
        let mut req1 = http::Request::builder().body(()).unwrap();
        let action1 = plugin.on_request(&mut req1, &ctx1).await.unwrap();
        assert_eq!(action1, PluginAction::Continue);

        // Second client (different IP) should also be allowed
        let ctx2 = make_ctx("2.2.2.2:2000");
        let mut req2 = http::Request::builder().body(()).unwrap();
        let action2 = plugin.on_request(&mut req2, &ctx2).await.unwrap();
        assert_eq!(action2, PluginAction::Continue);
    }
}
