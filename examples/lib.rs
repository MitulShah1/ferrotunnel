//! `FerroTunnel` Examples
//!
//! This crate provides examples demonstrating how to use `FerroTunnel`
//! in your own Rust applications.
//!
//! ## Available Examples
//!
//! ### Basic Usage
//!
//! - **`embedded_server`** - Embed a `FerroTunnel` server in your application
//! - **`embedded_client`** - Embed a `FerroTunnel` client in your application
//!
//! ### Advanced Features
//!
//! - **`custom_plugin`** - Create custom plugins to intercept/modify requests
//! - **`tls_config`** - Configure TLS for secure tunnel connections
//! - **`auto_reconnect`** - Demonstrate auto-reconnect with custom settings
//! - **`multi_tunnel`** - Run multiple tunnel clients for different services
//!
//! ## Running Examples
//!
//! ```bash
//! # Basic examples
//! cargo run -p ferrotunnel-examples --example embedded_server
//! cargo run -p ferrotunnel-examples --example embedded_client
//!
//! # Custom plugin example
//! cargo run -p ferrotunnel-examples --example custom_plugin
//!
//! # TLS example (server mode)
//! cargo run -p ferrotunnel-examples --example tls_config -- --mode server
//!
//! # Auto-reconnect example
//! cargo run -p ferrotunnel-examples --example auto_reconnect
//!
//! # Multi-tunnel example
//! cargo run -p ferrotunnel-examples --example multi_tunnel
//! ```
//!
//! ## Quick Start
//!
//! The fastest way to get started is with the embedded examples:
//!
//! 1. Start a local HTTP server (e.g., `python3 -m http.server 8000`)
//! 2. Run the server: `cargo run -p ferrotunnel-examples --example embedded_server`
//! 3. Run the client: `cargo run -p ferrotunnel-examples --example embedded_client`
//! 4. Access your local server through the tunnel!
