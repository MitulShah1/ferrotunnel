//! Example: Auto-Reconnect Client
//!
//! This example demonstrates the auto-reconnect feature for resilient tunnel connections.
//!
//! The client will automatically reconnect to the server if the connection is lost,
//! using exponential backoff to avoid overwhelming the server.
//!
//! # Usage
//!
//! ```bash
//! # First, start a server (in another terminal)
//! cargo run -p ferrotunnel-examples --example embedded_server
//!
//! # Then run this example
//! cargo run -p ferrotunnel-examples --example auto_reconnect
//! ```

use ferrotunnel::Client;
use std::time::Duration;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,ferrotunnel=debug")
        .init();

    println!("`FerroTunnel` Auto-Reconnect Example");
    println!("=====================================");
    println!();

    // Build client with auto-reconnect enabled
    let mut client = Client::builder()
        .server_addr("localhost:7835")
        .token("secret")
        .local_addr("127.0.0.1:8000")
        // Enable auto-reconnect (this is the default)
        .auto_reconnect(true)
        // Set custom reconnect delay
        .reconnect_delay(Duration::from_secs(3))
        .build()?;

    println!("Configuration:");
    println!("  Server: localhost:7835");
    println!("  Local: 127.0.0.1:8000");
    println!("  Auto-reconnect: enabled");
    println!("  Reconnect delay: 3 seconds");
    println!();

    println!("Connecting to server...");
    println!("(If the server is not running, the client will keep trying to reconnect)");
    println!();

    // Start the client - this will block until connected
    match client.start().await {
        Ok(info) => {
            println!("✅ Connected!");
            if let Some(session_id) = &info.session_id {
                println!("   Session ID: {session_id}");
            }
            if let Some(url) = &info.public_url {
                println!("   Public URL: {url}");
            }
            println!();
            println!("The client will automatically reconnect if:");
            println!("  • The server restarts");
            println!("  • Network connection is interrupted");
            println!("  • Any temporary connection failure occurs");
            println!();
            println!("Press Ctrl+C to stop");

            // Wait for Ctrl+C
            tokio::signal::ctrl_c().await?;

            println!();
            println!("Shutting down...");
            client.shutdown().await?;
            println!("Goodbye!");
        }
        Err(e) => {
            println!("❌ Failed to connect: {e}");
            println!();
            println!("Make sure the server is running:");
            println!("  cargo run -p ferrotunnel-examples --example embedded_server");
        }
    }

    Ok(())
}
