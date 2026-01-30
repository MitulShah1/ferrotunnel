//! Example: Header Filter Plugin
#![allow(clippy::print_stdout)]
use async_trait::async_trait;
use ferrotunnel_plugin::{Plugin, PluginAction, PluginRegistry, RequestContext};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Plugin that removes sensitive headers and adds security headers
pub struct HeaderFilterPlugin {
    blocked_headers: Vec<String>,
}

impl HeaderFilterPlugin {
    pub fn new() -> Self {
        Self {
            blocked_headers: vec![
                "X-Internal-Secret".to_string(),
                "Server".to_string(), // Hide server version
            ],
        }
    }
}

impl Default for HeaderFilterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for HeaderFilterPlugin {
    fn name(&self) -> &'static str {
        "header-filter"
    }

    async fn on_request(
        &self,
        req: &mut http::Request<()>,
        _ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Remove sensitive headers
        for header in &self.blocked_headers {
            if req.headers().contains_key(header) {
                println!("Removing forbidden header: {header}");
                req.headers_mut().remove(header);
            }
        }

        // Add a security header
        req.headers_mut().insert("X-Security-Scan", "Pass".parse()?);

        Ok(PluginAction::Continue)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().init();

    let mut registry = PluginRegistry::new();
    registry.register(Arc::new(RwLock::new(HeaderFilterPlugin::new())));
    registry.init_all().await?;

    println!("Simulating request with forbidden header...");
    let mut req = http::Request::builder()
        .header("X-Internal-Secret", "super-secret-value")
        .header("User-Agent", "Mozilla/5.0")
        .body(())?;

    let ctx = RequestContext {
        tunnel_id: "test".into(),
        session_id: "test".into(),
        remote_addr: "127.0.0.1:0".parse()?,
        timestamp: std::time::SystemTime::now(),
    };

    registry.execute_request_hooks(&mut req, &ctx).await?;

    // Verify
    assert!(req.headers().get("X-Internal-Secret").is_none());
    assert!(req.headers().get("X-Security-Scan").is_some());
    println!("Request processing complete. Sensitive headers removed.");

    Ok(())
}
