//! Plugin integration tests
//!
//! Tests plugin system with auth and rate limiting

use ferrotunnel_plugin::{Plugin, PluginAction, PluginRegistry, RequestContext};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test auth plugin rejects unauthorized requests
#[tokio::test]
async fn test_auth_plugin_rejects_unauthorized() {
    use ferrotunnel_plugin::builtin::TokenAuthPlugin;

    let plugin = TokenAuthPlugin::new(vec!["valid-token".to_string()]);

    // Create request without auth header
    let mut request = http::Request::builder().uri("/test").body(()).unwrap();

    let ctx = RequestContext {
        tunnel_id: "test-tunnel".to_string(),
        session_id: "test-session".to_string(),
        remote_addr: "127.0.0.1:12345".parse().unwrap(),
        timestamp: std::time::SystemTime::now(),
    };

    let action = plugin.on_request(&mut request, &ctx).await;

    assert!(
        match action {
            Ok(PluginAction::Reject { status, .. }) => status == http::StatusCode::UNAUTHORIZED,
            _ => false,
        },
        "Should reject unauthorized request"
    );
}

/// Test auth plugin allows valid token
#[tokio::test]
async fn test_auth_plugin_allows_authorized() {
    use ferrotunnel_plugin::builtin::TokenAuthPlugin;

    let plugin = TokenAuthPlugin::new(vec!["valid-token".to_string()]);

    let mut request = http::Request::builder()
        .uri("/test")
        .header("X-Tunnel-Token", "valid-token")
        .body(())
        .unwrap();

    let ctx = RequestContext {
        tunnel_id: "test-tunnel".to_string(),
        session_id: "test-session".to_string(),
        remote_addr: "127.0.0.1:12345".parse().unwrap(),
        timestamp: std::time::SystemTime::now(),
    };

    let action = plugin.on_request(&mut request, &ctx).await;

    assert!(
        matches!(action, Ok(PluginAction::Continue)),
        "Should allow authorized request"
    );
}

/// Test rate limit plugin enforces limits
#[tokio::test]
async fn test_rate_limit_enforces() {
    use ferrotunnel_plugin::builtin::RateLimitPlugin;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    // 2 requests per second limit
    let plugin = RateLimitPlugin::try_new(2).expect("valid rate limit");

    let mut request = http::Request::builder().uri("/test").body(()).unwrap();

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 12345);

    // First two should pass
    for i in 0..2 {
        let ctx = RequestContext {
            tunnel_id: "test-tunnel".to_string(),
            session_id: "test-session".to_string(),
            remote_addr: addr,
            timestamp: std::time::SystemTime::now(),
        };
        let action = plugin.on_request(&mut request, &ctx).await;
        assert!(
            matches!(action, Ok(PluginAction::Continue)),
            "Request {} should be allowed",
            i + 1
        );
    }

    // Third should be rate limited
    let ctx = RequestContext {
        tunnel_id: "test-tunnel".to_string(),
        session_id: "test-session".to_string(),
        remote_addr: addr,
        timestamp: std::time::SystemTime::now(),
    };
    let action = plugin.on_request(&mut request, &ctx).await;

    assert!(
        match action {
            Ok(PluginAction::Reject { status, .. }) =>
                status == http::StatusCode::TOO_MANY_REQUESTS,
            _ => false,
        },
        "Third request should be rate limited"
    );
}

/// Test plugin registry executes plugins in order
#[tokio::test]
async fn test_registry_executes_in_order() {
    use ferrotunnel_plugin::builtin::LoggerPlugin;

    let mut registry = PluginRegistry::new();
    registry.register(Arc::new(RwLock::new(LoggerPlugin::default())));

    // Initialize plugins
    registry.init_all().await.expect("Failed to init plugins");

    let mut request = http::Request::builder().uri("/test").body(()).unwrap();

    let ctx = RequestContext {
        tunnel_id: "test-tunnel".to_string(),
        session_id: "test-session".to_string(),
        remote_addr: "127.0.0.1:12345".parse().unwrap(),
        timestamp: std::time::SystemTime::now(),
    };

    let action = registry.execute_request_hooks(&mut request, &ctx).await;
    assert!(matches!(action, Ok(PluginAction::Continue)));

    // Shutdown plugins
    registry
        .shutdown_all()
        .await
        .expect("Failed to shutdown plugins");
}
