use crate::traits::*;
use async_trait::async_trait;
use tracing::{info, warn};

/// Logs all requests and responses
pub struct LoggerPlugin {
    log_bodies: bool,
}

impl LoggerPlugin {
    pub fn new() -> Self {
        Self { log_bodies: false }
    }

    pub fn with_body_logging(mut self) -> Self {
        self.log_bodies = true;
        self
    }
}

impl Default for LoggerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for LoggerPlugin {
    fn name(&self) -> &str {
        "logger"
    }

    async fn on_request(
        &self,
        req: &mut http::Request<()>,
        ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        info!(
            tunnel_id = %ctx.tunnel_id,
            session_id = %ctx.session_id,
            method = %req.method(),
            uri = %req.uri(),
            remote_addr = %ctx.remote_addr,
            "Incoming request"
        );

        // Body logging disabled until streaming support is added
        // if self.log_bodies && !req.body().is_empty() {
        //     info!(body_size = req.body().len(), "Request body");
        // }

        Ok(PluginAction::Continue)
    }

    async fn on_response(
        &self,
        res: &mut http::Response<Vec<u8>>,
        ctx: &ResponseContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let status = res.status();

        if status.is_success() {
            info!(
                tunnel_id = %ctx.tunnel_id,
                status = status.as_u16(),
                duration_ms = ctx.duration_ms,
                "Response sent"
            );
        } else {
            warn!(
                tunnel_id = %ctx.tunnel_id,
                status = status.as_u16(),
                duration_ms = ctx.duration_ms,
                "Response sent (error)"
            );
        }

        Ok(PluginAction::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logger_plugin_name() {
        let plugin = LoggerPlugin::new();
        assert_eq!(plugin.name(), "logger");
    }

    #[test]
    fn test_logger_plugin_default() {
        let plugin = LoggerPlugin::default();
        assert!(!plugin.log_bodies);
    }

    #[test]
    fn test_logger_plugin_with_body_logging() {
        let plugin = LoggerPlugin::new().with_body_logging();
        assert!(plugin.log_bodies);
    }

    #[tokio::test]
    async fn test_logger_on_request_returns_continue() {
        let plugin = LoggerPlugin::new();
        let mut req = http::Request::builder()
            .method("GET")
            .uri("/api/test")
            .body(())
            .unwrap();

        let ctx = RequestContext {
            tunnel_id: "tunnel123".into(),
            session_id: "session456".into(),
            remote_addr: "192.168.1.100:54321".parse().unwrap(),
            timestamp: std::time::SystemTime::now(),
        };

        let action = plugin.on_request(&mut req, &ctx).await.unwrap();
        assert_eq!(action, PluginAction::Continue);
    }

    #[tokio::test]
    async fn test_logger_on_response_returns_continue() {
        let plugin = LoggerPlugin::new();
        let mut res = http::Response::builder().status(200).body(vec![]).unwrap();

        let ctx = ResponseContext {
            tunnel_id: "tunnel123".into(),
            session_id: "session456".into(),
            status_code: 200,
            duration_ms: 42,
        };

        let action = plugin.on_response(&mut res, &ctx).await.unwrap();
        assert_eq!(action, PluginAction::Continue);
    }
}
