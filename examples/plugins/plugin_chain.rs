//! Example: Plugin Chain
//!
//! This example demonstrates how multiple plugins work together in a chain,
//! processing requests in order. Each plugin can:
//! - Continue (pass to next plugin)
//! - Reject (stop chain, return error)
//! - Short-circuit (stop chain, return custom response)
//!
//! # Plugin Execution Order
//!
//! ```text
//! Request → [Auth] → [RateLimit] → [Logger] → [Metrics] → Backend
//!              ↓          ↓           ↓          ↓
//!           Reject?   Reject?     Continue   Continue
//! ```
//!
//! # Usage
//!
//! ```bash
//! cargo run -p ferrotunnel-examples --example plugin_chain
//! ```

use async_trait::async_trait;
use ferrotunnel_plugin::{Plugin, PluginAction, PluginRegistry, RequestContext};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Plugin 1: Authentication
// ============================================================================

/// Validates API keys in request headers
pub struct AuthPlugin {
    valid_keys: HashMap<String, String>, // key -> user_id
}

impl AuthPlugin {
    pub fn new() -> Self {
        let mut keys = HashMap::new();
        keys.insert("key-admin-123".to_string(), "admin".to_string());
        keys.insert("key-user-456".to_string(), "user".to_string());
        Self { valid_keys: keys }
    }
}

#[async_trait]
impl Plugin for AuthPlugin {
    fn name(&self) -> &str {
        "auth"
    }

    async fn on_request(
        &self,
        req: &mut http::Request<()>,
        _ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Check for API key header
        if let Some(key) = req.headers().get("x-api-key") {
            if let Ok(key_str) = key.to_str() {
                if let Some(user) = self.valid_keys.get(key_str) {
                    println!("  [Auth] ✅ Valid key for user: {}", user);
                    return Ok(PluginAction::Continue);
                }
            }
        }

        // Check for public endpoints that don't need auth
        let path = req.uri().path();
        if path == "/health" || path == "/public" {
            println!("  [Auth] ✅ Public endpoint: {}", path);
            return Ok(PluginAction::Continue);
        }

        println!("  [Auth] ❌ Missing or invalid API key");
        Ok(PluginAction::Reject {
            status: 401,
            reason: "Unauthorized: valid API key required".to_string(),
        })
    }
}

// ============================================================================
// Plugin 2: Rate Limiting
// ============================================================================

/// Simple rate limiter (for demo purposes)
pub struct RateLimitPlugin {
    request_count: AtomicU64,
    max_requests: u64,
}

impl RateLimitPlugin {
    pub fn new(max_requests: u64) -> Self {
        Self {
            request_count: AtomicU64::new(0),
            max_requests,
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
        _ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        let count = self.request_count.fetch_add(1, Ordering::Relaxed) + 1;

        if count > self.max_requests {
            println!("  [RateLimit] ❌ Limit exceeded ({}/{})", count, self.max_requests);
            return Ok(PluginAction::Reject {
                status: 429,
                reason: "Too Many Requests".to_string(),
            });
        }

        println!("  [RateLimit] ✅ Request {}/{}", count, self.max_requests);
        Ok(PluginAction::Continue)
    }
}

// ============================================================================
// Plugin 3: Request Logger
// ============================================================================

/// Logs all requests passing through
pub struct LoggerPlugin;

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
        println!(
            "  [Logger] 📝 {} {} from {} (tunnel: {})",
            req.method(),
            req.uri(),
            ctx.remote_addr,
            ctx.tunnel_id
        );
        Ok(PluginAction::Continue)
    }
}

// ============================================================================
// Plugin 4: Metrics Collector
// ============================================================================

/// Collects request metrics
pub struct MetricsPlugin {
    total: AtomicU64,
    by_method: RwLock<HashMap<String, u64>>,
}

impl MetricsPlugin {
    pub fn new() -> Self {
        Self {
            total: AtomicU64::new(0),
            by_method: RwLock::new(HashMap::new()),
        }
    }

    pub async fn print_stats(&self) {
        let methods = self.by_method.read().await;
        println!("\n📊 Metrics Summary:");
        println!("  Total requests: {}", self.total.load(Ordering::Relaxed));
        for (method, count) in methods.iter() {
            println!("    {}: {}", method, count);
        }
    }
}

impl Default for MetricsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for MetricsPlugin {
    fn name(&self) -> &str {
        "metrics"
    }

    async fn on_request(
        &self,
        req: &mut http::Request<()>,
        _ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.total.fetch_add(1, Ordering::Relaxed);

        let method = req.method().to_string();
        let mut methods = self.by_method.write().await;
        *methods.entry(method.clone()).or_insert(0) += 1;

        println!("  [Metrics] 📈 Recorded {} request", method);
        Ok(PluginAction::Continue)
    }
}

// ============================================================================
// Main: Demonstrate Plugin Chain
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("`FerroTunnel` Plugin Chain Example");
    println!("===================================");
    println!();
    println!("Plugin execution order:");
    println!("  1. AuthPlugin     - Validates API keys");
    println!("  2. RateLimitPlugin - Limits requests (max 5)");
    println!("  3. LoggerPlugin   - Logs request details");
    println!("  4. MetricsPlugin  - Collects statistics");
    println!();

    // Create and register plugins in order
    let mut registry = PluginRegistry::new();

    registry.register(Arc::new(RwLock::new(AuthPlugin::new())));
    registry.register(Arc::new(RwLock::new(RateLimitPlugin::new(5))));
    registry.register(Arc::new(RwLock::new(LoggerPlugin)));
    let metrics = Arc::new(RwLock::new(MetricsPlugin::new()));
    registry.register(metrics.clone());

    // Initialize all plugins
    registry.init_all().await?;

    // Request context for all tests
    let ctx = RequestContext {
        tunnel_id: "demo-tunnel".to_string(),
        session_id: "demo-session".to_string(),
        remote_addr: "192.168.1.100:54321".parse().unwrap(),
        timestamp: std::time::SystemTime::now(),
    };

    // Test 1: Request without API key (should fail at Auth)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 1: GET /api/data (no API key)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let mut req1 = http::Request::builder()
        .method("GET")
        .uri("/api/data")
        .body(())
        .unwrap();
    let result1 = registry.execute_request_hooks(&mut req1, &ctx).await?;
    println!("  Result: {:?}", result1);
    println!();

    // Test 2: Request with valid API key (should pass all plugins)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 2: GET /api/users (with valid API key)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let mut req2 = http::Request::builder()
        .method("GET")
        .uri("/api/users")
        .header("x-api-key", "key-admin-123")
        .body(())
        .unwrap();
    let result2 = registry.execute_request_hooks(&mut req2, &ctx).await?;
    println!("  Result: {:?}", result2);
    println!();

    // Test 3: Public endpoint (no auth needed)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Test 3: GET /health (public endpoint)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let mut req3 = http::Request::builder()
        .method("GET")
        .uri("/health")
        .body(())
        .unwrap();
    let result3 = registry.execute_request_hooks(&mut req3, &ctx).await?;
    println!("  Result: {:?}", result3);
    println!();

    // Test 4-7: More requests to hit rate limit
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Tests 4-7: POST requests to trigger rate limit");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    for i in 4..=7 {
        let mut req = http::Request::builder()
            .method("POST")
            .uri("/api/data")
            .header("x-api-key", "key-user-456")
            .body(())
            .unwrap();
        println!("Request {}:", i);
        let result = registry.execute_request_hooks(&mut req, &ctx).await?;
        println!("  Result: {:?}", result);
        println!();
    }

    // Print metrics summary
    metrics.read().await.print_stats().await;

    // Shutdown
    registry.shutdown_all().await?;

    println!();
    println!("Example completed successfully!");

    Ok(())
}
