//! `FerroTunnel` Examples
//!
//! This crate provides examples demonstrating how to use `FerroTunnel`
//! in your own Rust applications, organized by category.
//!
//! ## Directory Structure
//!
//! ```text
//! examples/
//! ├── basic/              # Getting started examples
//! │   ├── embedded_server.rs
//! │   ├── embedded_client.rs
//! │   └── auto_reconnect.rs
//! ├── plugins/            # Plugin development examples
//! │   ├── custom_plugin.rs
//! │   ├── hello_plugin.rs
//! │   ├── header_filter.rs
//! │   └── ip_blocklist.rs
//! └── advanced/           # Production patterns
//!     ├── tls_config.rs
//!     └── multi_tunnel.rs
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
//! - **`hello_plugin`** - Simple "Hello World" plugin
//! - **`custom_plugin`** - Request counting and path blocking
//! - **`header_filter`** - Filter/modify HTTP headers
//! - **`ip_blocklist`** - Block requests by IP address
//!
//! ```bash
//! cargo run -p ferrotunnel-examples --example hello_plugin
//! cargo run -p ferrotunnel-examples --example custom_plugin
//! cargo run -p ferrotunnel-examples --example header_filter
//! cargo run -p ferrotunnel-examples --example ip_blocklist
//! ```
//!
//! ## Advanced Examples
//!
//! Production configurations and patterns:
//!
//! - **`tls_config`** - Configure TLS for secure connections
//! - **`multi_tunnel`** - Run multiple tunnels for different services
//!
//! ```bash
//! cargo run -p ferrotunnel-examples --example tls_config -- --mode server
//! cargo run -p ferrotunnel-examples --example multi_tunnel
//! ```
//!
//! ## Quick Start
//!
//! 1. Start a local HTTP server: `python3 -m http.server 8000`
//! 2. Run the server: `cargo run -p ferrotunnel-examples --example embedded_server`
//! 3. Run the client: `cargo run -p ferrotunnel-examples --example embedded_client`
//! 4. Access your local server through the tunnel!
