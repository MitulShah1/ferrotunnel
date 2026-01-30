//! Example: Custom Plugin
#![allow(clippy::unwrap_used)]
#![allow(clippy::print_stdout)]
#![allow(clippy::uninlined_format_args)]
//!
//! This example shows how to create a custom plugin for `FerroTunnel`.
//!
//! Plugins can intercept and modify requests/responses flowing through
//! the tunnel.
//!
//! # Usage
//!
//! ```bash
//! cargo run -p ferrotunnel-examples --example custom_plugin
//! ```

use async_trait::async_trait;
use ferrotunnel_plugin::{Plugin, PluginAction, PluginRegistry, RequestContext, ResponseContext};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// A custom plugin that counts requests and adds timing headers.
pub struct MetricsPlugin {
    request_count: AtomicU64,
}

impl MetricsPlugin {
    pub fn new() -> Self {
        Self {
            request_count: AtomicU64::new(0),
        }
    }

    pub fn count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }
}

impl Default for MetricsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for MetricsPlugin {
    fn name(&self) -> &'static str {
        "metrics"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    async fn init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        println!("[MetricsPlugin] Initialized");
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        println!(
            "[MetricsPlugin] Shutdown - Total requests: {}",
            self.count()
        );
        Ok(())
    }

    async fn on_request(
        &self,
        req: &mut http::Request<()>,
        ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Increment request count
        let count = self.request_count.fetch_add(1, Ordering::Relaxed) + 1;

        println!(
            "[MetricsPlugin] Request #{} - {} {} (tunnel: {})",
            count,
            req.method(),
            req.uri(),
            ctx.tunnel_id
        );

        // Continue processing (allow other plugins and the actual request)
        Ok(PluginAction::Continue)
    }

    async fn on_response(
        &self,
        _res: &mut http::Response<Vec<u8>>,
        ctx: &ResponseContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        println!(
            "[MetricsPlugin] Response {} - {}ms (tunnel: {})",
            ctx.status_code, ctx.duration_ms, ctx.tunnel_id
        );

        Ok(PluginAction::Continue)
    }

    fn needs_response_body(&self) -> bool {
        // We don't need to buffer body
        false
    }
}

/// A plugin that blocks requests to certain paths.
pub struct PathBlockerPlugin {
    blocked_paths: Vec<String>,
}

impl PathBlockerPlugin {
    pub fn new(blocked_paths: Vec<String>) -> Self {
        Self { blocked_paths }
    }
}

#[async_trait]
impl Plugin for PathBlockerPlugin {
    fn name(&self) -> &'static str {
        "path-blocker"
    }

    async fn on_request(
        &self,
        req: &mut http::Request<()>,
        _ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let path = req.uri().path();

        for blocked in &self.blocked_paths {
            if path.starts_with(blocked) {
                println!("[PathBlockerPlugin] Blocked access to: {path}");
                return Ok(PluginAction::Reject {
                    status: 403,
                    reason: format!("Access to {path} is forbidden"),
                });
            }
        }

        Ok(PluginAction::Continue)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("`FerroTunnel` Custom Plugin Example");
    println!("====================================");
    println!();

    // Create plugin registry
    let mut registry = PluginRegistry::new();

    // Register our custom plugins
    let metrics = Arc::new(RwLock::new(MetricsPlugin::new()));
    let blocker = Arc::new(RwLock::new(PathBlockerPlugin::new(vec![
        "/admin".to_string(),
        "/api/internal".to_string(),
    ])));

    registry.register(metrics.clone());
    registry.register(blocker);

    // Initialize all plugins
    registry.init_all().await?;

    println!("Plugins registered:");
    println!("  - MetricsPlugin: Counts requests and logs timing");
    println!("  - PathBlockerPlugin: Blocks /admin and /api/internal paths");
    println!();

    // Simulate some requests for demonstration
    println!("Simulating plugin execution...");
    println!();

    // Create mock request contexts
    let ctx = RequestContext {
        tunnel_id: "demo-tunnel".to_string(),
        session_id: "demo-session".to_string(),
        remote_addr: "127.0.0.1:12345".parse().unwrap(),
        timestamp: std::time::SystemTime::now(),
    };

    // Simulate request to allowed path
    let mut req1 = http::Request::builder()
        .method("GET")
        .uri("/api/users")
        .body(())
        .unwrap();

    let action1 = registry.execute_request_hooks(&mut req1, &ctx).await?;
    println!("  /api/users -> {action1:?}");

    // Simulate request to blocked path
    let mut req2 = http::Request::builder()
        .method("GET")
        .uri("/admin/settings")
        .body(())
        .unwrap();

    let action2 = registry.execute_request_hooks(&mut req2, &ctx).await?;
    println!("  /admin/settings -> {action2:?}");

    // Simulate one more request to show the counter
    let mut req3 = http::Request::builder()
        .method("POST")
        .uri("/api/data")
        .body(())
        .unwrap();

    let action3 = registry.execute_request_hooks(&mut req3, &ctx).await?;
    println!("  /api/data -> {action3:?}");

    // Show final count
    println!();
    println!("Total requests processed: {}", metrics.read().await.count());

    // Shutdown plugins
    registry.shutdown_all().await?;

    println!();
    println!("Example completed successfully!");

    Ok(())
}
