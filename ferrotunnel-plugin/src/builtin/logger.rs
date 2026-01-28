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
