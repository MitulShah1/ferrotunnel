//! `FerroTunnel` Examples
//!
//! This crate provides examples demonstrating how to use `FerroTunnel`
//! in your own Rust applications, organized by category.
//!
//! ## Directory Structure
//!
//! ```text
//! examples/
//! ├── basic/              # Getting started
//! │   ├── embedded_server.rs
//! │   ├── embedded_client.rs
//! │   └── auto_reconnect.rs
//! ├── plugins/            # Plugin development
//! │   ├── custom_plugin.rs
//! │   ├── header_filter.rs
//! │   ├── ip_blocklist.rs
//! │   └── plugin_chain.rs
//! ├── advanced/           # TLS and multi-tunnel
//! │   ├── tls_config.rs
//! │   └── multi_tunnel.rs
//! ├── operational/        # Server lifecycle and observability
//! │   ├── server_graceful_shutdown.rs
//! │   └── server_observability.rs
//! └── scenarios/          # Common usage scenarios
//!     ├── expose_local_dev.rs
//!     └── receive_webhooks_locally.rs
//! ```
//!
//! ## Basic Examples
//!
//! Start here if you're new to `FerroTunnel`:
//!
//! - **`embedded_server`** - Embed a tunnel server in your application
//! - **`embedded_client`** - Embed a tunnel client in your application
//! - **`auto_reconnect`** - Client with auto-reconnect and custom settings
//!
//! ```bash
//! cargo run -p ferrotunnel-examples --example embedded_server
//! cargo run -p ferrotunnel-examples --example embedded_client
//! cargo run -p ferrotunnel-examples --example auto_reconnect
//! ```
//!
//! ## Plugin Examples
//!
//! Learn how to extend `FerroTunnel` with custom plugins:
//!
//! - **`custom_plugin`** - Request counting and path blocking
//! - **`header_filter`** - Filter/modify HTTP headers
//! - **`ip_blocklist`** - Block requests by IP address
//! - **`plugin_chain`** - Multiple plugins working together
//!
//! ```bash
//! cargo run -p ferrotunnel-examples --example custom_plugin
//! cargo run -p ferrotunnel-examples --example header_filter
//! cargo run -p ferrotunnel-examples --example ip_blocklist
//! cargo run -p ferrotunnel-examples --example plugin_chain
//! ```
//!
//! ## Advanced Examples
//!
//! - **`tls_config`** - Configure TLS for secure connections
//! - **`multi_tunnel`** - Run multiple tunnels for different services
//!
//! ```bash
//! cargo run -p ferrotunnel-examples --example tls_config -- --mode server
//! cargo run -p ferrotunnel-examples --example multi_tunnel
//! ```
//!
//! ## Operational Examples
//!
//! Server lifecycle and observability:
//!
//! - **`server_graceful_shutdown`** - Shutdown on SIGTERM/SIGINT and drain connections
//! - **`server_observability`** - Server with Prometheus metrics and logging
//!
//! ```bash
//! cargo run -p ferrotunnel-examples --example server_graceful_shutdown
//! cargo run -p ferrotunnel-examples --example server_observability
//! ```
//!
//! ## Scenario Examples
//!
//! Common usage scenarios:
//!
//! - **`expose_local_dev`** - Expose your local dev server (e.g. React) for sharing and testing
//! - **`receive_webhooks_locally`** - Forward webhooks (GitHub, Stripe) to your local machine
//!
//! ```bash
//! cargo run -p ferrotunnel-examples --example expose_local_dev
//! cargo run -p ferrotunnel-examples --example receive_webhooks_locally
//! ```
//!
//! ## Quick Start
//!
//! 1. Start a local HTTP server: `python3 -m http.server 8000`
//! 2. Run the server: `cargo run -p ferrotunnel-examples --example embedded_server`
//! 3. Run the client: `cargo run -p ferrotunnel-examples --example embedded_client`
//! 4. Access your local server through the tunnel!
