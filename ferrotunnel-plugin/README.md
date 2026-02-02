# Ferrotunnel Plugin System

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel-plugin.svg)](https://crates.io/crates/ferrotunnel-plugin)
[![Documentation](https://docs.rs/ferrotunnel-plugin/badge.svg)](https://docs.rs/ferrotunnel-plugin)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

This crate contains the core traits and infrastructure for the Ferrotunnel plugin system.

## Plugin Developer Guide

FerroTunnel supports a powerful trait-based plugin system that allows you to intercept and modify request/response traffic, enforce authentication, rate limiting, and more.

### Quick Start

To create a new plugin, implement the `Plugin` trait from `ferrotunnel-plugin`.

```rust
use ferrotunnel_plugin::{Plugin, PluginAction, RequestContext, ResponseContext};
use async_trait::async_trait;

pub struct MyPlugin;

#[async_trait]
impl Plugin for MyPlugin {
    fn name(&self) -> &str {
        "my-plugin"
    }

    async fn on_request(
        &self,
        req: &mut http::Request<Vec<u8>>,
        _ctx: &RequestContext,
    ) -> Result<PluginAction, Box<dyn std::error::Error + Send + Sync + 'static>> {
        println!("Received request: {}", req.uri());
        Ok(PluginAction::Continue)
    }
}
```

### Plugin Lifecycle

1. **Init**: Called when the server starts. Use this to set up database connections or other resources.
2. **Hooks**:
   - `on_request`: Called before the request is forwarded to the tunnel.
   - `on_response`: Called after the response is received from the tunnel.
   - `on_stream_data`: Called when raw data flows through the stream (TCP mode).
3. **Shutdown**: Called when the server shuts down.

### Plugin Actions

- `PluginAction::Continue`: Allow the request to proceed to the next plugin or target.
- `PluginAction::Reject { status, reason }`: Stop processing and return an error response immediately.
- `PluginAction::Respond { status, headers, body }`: Return a custom response immediately.
- `PluginAction::Modify`: (Upcoming) Modify the request/response extensively.

### Examples

See `ferrotunnel-plugin/src/builtin/` for built-in plugins (Logger, TokenAuth, RateLimit).

Check the `examples/` directory for more:
- `hello_plugin.rs`: Simple header injection.
- `header_filter.rs`: Removing sensitive headers.
- `ip_blocklist.rs`: Blocking requests by IP.

### Testing

You can test plugins in two ways:

1. **Unit Tests**: Mock `RequestContext` and assert on `PluginAction` results.
   ```rust
   #[tokio::test]
   async fn test_my_plugin() {
       let plugin = MyPlugin::new();
       let mut req = http::Request::builder().body(vec![]).unwrap();
       let ctx = RequestContext { ... };

       let action = plugin.on_request(&mut req, &ctx).await.unwrap();
       assert_eq!(action, PluginAction::Continue);
   }
   ```

2. **Run Examples**:
   ```bash
   cargo run -p ferrotunnel-plugin --example hello_plugin
   cargo run -p ferrotunnel-plugin --example header_filter
   cargo run -p ferrotunnel-plugin --example ip_blocklist
   ```

### Usage

Register your plugin in `ferrotunnel-cli/src/commands/server.rs`:

```rust
registry.register(Arc::new(RwLock::new(MyPlugin)));
```
