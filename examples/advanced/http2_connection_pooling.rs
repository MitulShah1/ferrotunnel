//! Example: HTTP/2 Support and Connection Pooling
//!
//! This example demonstrates the HTTP/2 support and connection pooling features
//! introduced in FerroTunnel v1.0.3+.
//!
//! # Features Demonstrated
//!
//! - **HTTP/2 Auto-Detection**: Server ingress automatically handles HTTP/1.1 and HTTP/2
//! - **Connection Pooling**: Client proxy reuses connections to local services
//! - **Custom Pool Configuration**: Configure pool size, timeouts, and HTTP/2 preference
//!
//! # Performance Benefits
//!
//! - Eliminates TCP handshake overhead per request
//! - HTTP/2 multiplexing reduces connection count
//! - Background eviction prevents resource leaks
//! - Significantly improves throughput (target: 800-1000 MB/s)
//!
//! # Usage
//!
//! ```bash
//! # Terminal 1: Start a server (the ingress will auto-detect HTTP/1.1 and HTTP/2)
//! cargo run -p ferrotunnel-examples --example http2_connection_pooling -- server
//!
//! # Terminal 2: Start a local HTTP server on port 8000
//! python3 -m http.server 8000
//!
//! # Terminal 3: Start the client with custom pool configuration
//! cargo run -p ferrotunnel-examples --example http2_connection_pooling -- client
//!
//! # Terminal 4: Test with HTTP/1.1
//! curl -v http://localhost:8080
//!
//! # Terminal 5: Test with HTTP/2 (if you have curl with HTTP/2 support)
//! curl -v --http2 http://localhost:8080
//! ```
//!
//! # Notes
//!
//! The server ingress automatically detects and handles both HTTP/1.1 and HTTP/2 connections
//! from external clients. The connection pool on the client side reuses connections to the
//! local service for improved performance.

use ferrotunnel::{Client, Server};
use std::env;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,ferrotunnel=debug")
        .init();

    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map_or("client", String::as_str);

    match mode {
        "server" => run_server().await,
        "client" => run_client().await,
        _ => {
            eprintln!("Usage: {} [server|client]", args[0]);
            std::process::exit(1);
        }
    }
}

async fn run_server() -> ferrotunnel::Result<()> {
    println!("FerroTunnel HTTP/2 Example - Server Mode");
    println!("=========================================");
    println!();
    println!("Server ingress automatically supports:");
    println!("  • HTTP/1.1 connections");
    println!("  • HTTP/2 connections");
    println!("  • WebSocket upgrades");
    println!();
    println!("HTTP ingress will listen on: 0.0.0.0:8080");
    println!("Control plane will listen on: 0.0.0.0:7835");
    println!();

    let mut server = Server::builder()
        .bind("0.0.0.0:7835".parse().expect("Valid address"))
        .token("example-secret-token")
        .http_bind("0.0.0.0:8080".parse().expect("Valid address"))
        .build()?;

    println!("Starting server with HTTP/2 auto-detection...");
    println!("Press Ctrl+C to stop");
    println!();

    tokio::select! {
        result = server.start() => result,
        _ = tokio::signal::ctrl_c() => {
            println!("\nShutting down server...");
            server.shutdown().await?;
            Ok(())
        }
    }
}

async fn run_client() -> ferrotunnel::Result<()> {
    println!("FerroTunnel HTTP/2 Example - Client Mode");
    println!("=========================================");
    println!();
    println!("Connection pooling enabled with:");
    println!("  • Max idle connections per host: 32");
    println!("  • Idle timeout: 90 seconds");
    println!("  • HTTP/2 preference: disabled (uses HTTP/1.1 by default)");
    println!();
    println!("Pool benefits:");
    println!("  🚀 Eliminates TCP handshake overhead per request");
    println!("  🔄 HTTP/2 multiplexing reduces connection count");
    println!("  🧹 Background eviction prevents resource leaks");
    println!();

    // Build client with connection pooling (enabled by default in v1.0.3+)
    let mut client = Client::builder()
        .server_addr("localhost:7835")
        .token("example-secret-token")
        .local_addr("127.0.0.1:8000")
        .tunnel_id("http2-example")
        .auto_reconnect(true)
        .build()?;

    println!("Connecting to server...");
    let info = client.start().await?;

    println!("Connected!");
    if let Some(url) = &info.public_url {
        println!("Public URL: {url}");
    }
    println!();

    println!("Client is now using connection pooling for all requests to 127.0.0.1:8000");
    println!();
    println!("To customize the pool configuration, use the library API directly:");
    println!("  use ferrotunnel_http::{{HttpProxy, PoolConfig}};");
    println!("  use std::time::Duration;");
    println!();
    println!("  let pool_config = PoolConfig {{");
    println!("      max_idle_per_host: 64,                    // Increase pool size");
    println!("      idle_timeout: Duration::from_secs(120),   // Longer timeout");
    println!("      prefer_h2: true,                          // Prefer HTTP/2");
    println!("  }};");
    println!();
    println!("  let proxy = HttpProxy::with_pool_config(\"127.0.0.1:8000\".into(), pool_config);");
    println!();
    println!("Press Ctrl+C to stop");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    println!("\nShutting down client...");
    client.shutdown().await?;

    Ok(())
}
