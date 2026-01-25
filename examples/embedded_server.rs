//! Example: Embedded `FerroTunnel` Server
//!
//! This example shows how to embed `FerroTunnel` server in your application.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example embedded_server -- \
//!     --bind 0.0.0.0:7835 \
//!     --http-bind 0.0.0.0:8080 \
//!     --token my-secret-token
//! ```

use ferrotunnel::Server;
use std::env;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,ferrotunnel=debug")
        .init();

    // Parse command line arguments (simple parsing for example)
    let args: Vec<String> = env::args().collect();

    let bind_addr = get_arg(&args, "--bind")
        .unwrap_or_else(|| "0.0.0.0:7835".to_string())
        .parse()
        .unwrap_or_else(|_| ([0, 0, 0, 0], 7835).into());

    let http_bind_addr = get_arg(&args, "--http-bind")
        .unwrap_or_else(|| "0.0.0.0:8080".to_string())
        .parse()
        .unwrap_or_else(|_| ([0, 0, 0, 0], 8080).into());

    let token = get_arg(&args, "--token").unwrap_or_else(|| "secret".to_string());

    println!("`FerroTunnel` Embedded Server Example");
    println!("====================================");
    println!("Tunnel bind: {bind_addr}");
    println!("HTTP bind:   {http_bind_addr}");
    println!();

    // Build and start the server using the builder pattern
    let mut server = Server::builder()
        .bind(bind_addr)
        .http_bind(http_bind_addr)
        .token(&token)
        .build()?;

    println!("Starting server...");
    println!("Press Ctrl+C to stop");
    println!();

    // Start the server (this will run until shutdown)
    server.start().await?;

    Ok(())
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}
