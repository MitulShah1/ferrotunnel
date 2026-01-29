# Plugin Development Guide

This guide explains how to create custom plugins for FerroTunnel.

## Plugin Architecture

Plugins extend FerroTunnel's functionality through hooks that intercept HTTP requests and responses.

```
Request → [Auth Plugin] → [Rate Limit] → [Your Plugin] → Local Service
                                              ↓
Response ← [Logger Plugin] ← [Your Plugin] ← Local Service
```

## Creating a Plugin

### 1. Implement the `Plugin` Trait

```rust
use ferrotunnel_plugin::{Plugin, PluginAction, RequestContext, ResponseContext};
use async_trait::async_trait;

#[derive(Default)]
pub struct MyPlugin {
    // Your plugin state
}

#[async_trait]
impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        "my-plugin"
    }

    async fn on_request(&self, ctx: &mut RequestContext<'_>) -> PluginAction {
        // Inspect or modify the request
        if ctx.request.uri().path().starts_with("/blocked") {
            return PluginAction::Reject {
                status: http::StatusCode::FORBIDDEN,
                body: Some("Blocked by plugin".into()),
            };
        }

        PluginAction::Continue
    }

    async fn on_response(&self, ctx: &mut ResponseContext<'_>) -> PluginAction {
        // Inspect or modify the response
        PluginAction::Continue
    }
}
```

### 2. Register Your Plugin

```rust
use ferrotunnel::Server;

let server = Server::builder()
    .bind("0.0.0.0:7835".parse().unwrap())
    .token("secret")
    .plugin(Box::new(MyPlugin::default()))
    .build()?;
```

## PluginAction Options

| Action | Purpose |
|--------|---------|
| `Continue` | Pass to next plugin |
| `Reject { status, body }` | Reject with HTTP error |
| `Respond { response }` | Short-circuit with custom response |
| `Modify` | Request/response was modified, continue |

## Built-in Plugins

| Plugin | Purpose |
|--------|---------|
| `LoggerPlugin` | Logs request/response details |
| `TokenAuthPlugin` | Header-based token authentication |
| `RateLimitPlugin` | IP-based rate limiting |
| `CircuitBreakerPlugin` | Failure isolation |

## Best Practices

1. **Keep plugins fast** - Avoid blocking I/O in hooks
2. **Use async** - All plugin methods are async
3. **Handle errors gracefully** - Return `Reject` on errors
4. **Log appropriately** - Use `tracing` for observability

## Example: IP Blocklist Plugin

```rust
use std::collections::HashSet;

pub struct IpBlocklistPlugin {
    blocked_ips: HashSet<String>,
}

#[async_trait]
impl Plugin for IpBlocklistPlugin {
    fn name(&self) -> &str { "ip-blocklist" }

    async fn on_request(&self, ctx: &mut RequestContext<'_>) -> PluginAction {
        if let Some(addr) = ctx.remote_addr {
            if self.blocked_ips.contains(&addr.ip().to_string()) {
                return PluginAction::Reject {
                    status: http::StatusCode::FORBIDDEN,
                    body: Some("IP blocked".into()),
                };
            }
        }
        PluginAction::Continue
    }
}
```

## Testing Plugins

```rust
#[tokio::test]
async fn test_my_plugin() {
    let plugin = MyPlugin::default();

    let mut request = http::Request::builder()
        .uri("/test")
        .body(())
        .unwrap();

    let mut ctx = RequestContext {
        request: &mut request,
        remote_addr: None,
    };

    let action = plugin.on_request(&mut ctx).await;
    assert!(matches!(action, PluginAction::Continue));
}
```
