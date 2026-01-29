//! Example: Embedded `FerroTunnel` Client
//!
//! This example shows how to embed `FerroTunnel` client in your application.
//!
//! # Usage
//!
//! ```bash
//! # Start a local HTTP server on port 8000 (e.g., with Python)
//! python3 -m http.server 8000
//!
//! # Run this example
//! cargo run --example embedded_client -- \
//!     --server localhost:7835 \
//!     --token my-secret-token \
//!     --local-addr 127.0.0.1:8000
//! ```

use ferrotunnel::Client;
use std::env;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,ferrotunnel=debug")
        .init();

    // Parse command line arguments (simple parsing for example)
    let args: Vec<String> = env::args().collect();

    let server_addr = get_arg(&args, "--server").unwrap_or_else(|| "localhost:7835".to_string());
    let token = get_arg(&args, "--token").unwrap_or_else(|| "secret".to_string());
    let local_addr = get_arg(&args, "--local-addr").unwrap_or_else(|| "127.0.0.1:8000".to_string());

    println!("`FerroTunnel` Embedded Client Example");
    println!("====================================");
    println!("Server:     {server_addr}");
    println!("Local addr: {local_addr}");
    println!();

    // Build and start the client using the builder pattern
    let mut client = Client::builder()
        .server_addr(&server_addr)
        .token(&token)
        .local_addr(&local_addr)
        .auto_reconnect(true)
        .build()?;

    println!("Connecting to server...");

    let info = client.start().await?;
    println!("Connected!");

    if let Some(url) = &info.public_url {
        println!("Public URL: {url}");
    }

    println!();
    println!("Press Ctrl+C to stop");

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    println!("\nShutting down...");
    client.shutdown().await?;

    Ok(())
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}
