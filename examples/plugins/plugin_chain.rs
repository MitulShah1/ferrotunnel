//! Example: Plugin Chain
#![allow(clippy::unwrap_used)]
#![allow(clippy::print_stdout)]
#![allow(clippy::uninlined_format_args)]
//!
//! This example demonstrates how multiple builtin plugins work together in a chain.
//! Each plugin can:
//! - Continue (pass to next plugin)
//! - Reject (stop chain, return error)
//! - Short-circuit (stop chain, return custom response)
//!
//! # Plugin Execution Order
//!
//! ```text
//! Request → [TokenAuth] → [RateLimit] → [Logger] → Backend
//!               ↓             ↓            ↓
//!            Reject?       Reject?     Continue
//! ```
//!
//! # Usage
//!
//! ```bash
//! cargo run -p ferrotunnel-examples --example plugin_chain
//! ```

use ferrotunnel_plugin::builtin::{LoggerPlugin, RateLimitPlugin, TokenAuthPlugin};
use ferrotunnel_plugin::{PluginAction, PluginRegistry, RequestContext};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging for the LoggerPlugin
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("`FerroTunnel` Plugin Chain Example");
    println!("===================================");
    println!();
    println!("Using builtin plugins:");
    println!("  1. TokenAuthPlugin  - Validates API tokens");
    println!("  2. RateLimitPlugin  - Limits requests per second");
    println!("  3. LoggerPlugin     - Logs request details");
    println!();

    // Create plugin registry and register builtin plugins
    let mut registry = PluginRegistry::new();

    // Plugin 1: Token authentication (valid tokens: "secret-123", "admin-token")
    let auth = TokenAuthPlugin::new(vec!["secret-123".to_string(), "admin-token".to_string()])
        .with_header_name("X-API-Key".to_string());
    registry.register(Arc::new(RwLock::new(auth)));

    // Plugin 2: Rate limiting (10 requests per second per IP)
    let rate_limit = RateLimitPlugin::try_new(10).expect("valid rate limit");
    registry.register(Arc::new(RwLock::new(rate_limit)));

    // Plugin 3: Request/response logging
    let logger = LoggerPlugin::new();
    registry.register(Arc::new(RwLock::new(logger)));

    // Initialize all plugins
    registry.init_all().await?;

    println!("Plugins initialized! Starting tests...");
    println!();

    // Test 1: Request without API key (should fail at Auth)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 1: GET /api/data (no API key)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let result1 =
        execute_request(&registry, "GET", "/api/data", None, "192.168.1.100:54321").await?;
    println!("Result: {result1:?}");
    println!();

    // Test 2: Request with valid API key (should pass all plugins)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 2: GET /api/users (with valid API key)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let result2 = execute_request(
        &registry,
        "GET",
        "/api/users",
        Some("secret-123"),
        "192.168.1.100:54322",
    )
    .await?;
    println!("Result: {result2:?}");
    println!();

    // Test 3: Request with admin token
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 3: POST /api/admin (with admin token)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let result3 = execute_request(
        &registry,
        "POST",
        "/api/admin",
        Some("admin-token"),
        "10.0.0.1:8080",
    )
    .await?;
    println!("Result: {result3:?}");
    println!();

    // Test 4: Request with invalid token
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 4: GET /api/data (invalid token)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let result4 = execute_request(
        &registry,
        "GET",
        "/api/data",
        Some("wrong-token"),
        "192.168.1.200:12345",
    )
    .await?;
    println!("Result: {result4:?}");
    println!();

    // Test 5-7: Multiple requests to demonstrate rate limiting
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Tests 5-7: Rapid requests from same IP");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    for i in 5..=7 {
        let result = execute_request(
            &registry,
            "GET",
            &format!("/api/item/{i}"),
            Some("secret-123"),
            "10.10.10.10:9999", // Same IP for all requests
        )
        .await?;
        println!("Request {i}: {result:?}");
    }
    println!();

    // Shutdown plugins
    registry.shutdown_all().await?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("• TokenAuthPlugin: Blocks requests without valid X-API-Key");
    println!("• RateLimitPlugin: Limits requests per IP (10/sec)");
    println!("• LoggerPlugin: Logs all passing requests via tracing");
    println!();
    println!("Example completed successfully!");

    Ok(())
}

/// Helper function to execute a request through the plugin chain
async fn execute_request(
    registry: &PluginRegistry,
    method: &str,
    uri: &str,
    api_key: Option<&str>,
    remote_addr: &str,
) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync>> {
    let mut builder = http::Request::builder().method(method).uri(uri);

    if let Some(key) = api_key {
        builder = builder.header("X-API-Key", key);
    }

    let mut req = builder.body(()).unwrap();

    let ctx = RequestContext {
        tunnel_id: "demo-tunnel".to_string(),
        session_id: "demo-session".to_string(),
        remote_addr: remote_addr.parse().unwrap(),
        timestamp: std::time::SystemTime::now(),
    };

    registry.execute_request_hooks(&mut req, &ctx).await
}
