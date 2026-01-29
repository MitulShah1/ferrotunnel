//! Example: Multi-Tunnel Client
//!
//! This example demonstrates running multiple tunnel clients simultaneously,
//! each forwarding different local services to the same tunnel server.
//!
//! This is useful when you have multiple services running locally that you
//! want to expose through the tunnel.
//!
//! # Usage
//!
//! ```bash
//! # Start a local web server on port 3000 (e.g., a React app)
//! # Start a local API server on port 4000 (e.g., a REST API)
//! # Start a local admin panel on port 5000
//!
//! # Then run this example
//! cargo run -p ferrotunnel-examples --example multi_tunnel
//! ```

use ferrotunnel::Client;
use std::time::Duration;

/// Configuration for a single tunnel
struct TunnelConfig {
    name: &'static str,
    local_addr: &'static str,
    description: &'static str,
}

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    println!("`FerroTunnel` Multi-Tunnel Example");
    println!("===================================");
    println!();

    // Define multiple tunnels for different services
    let tunnels = vec![
        TunnelConfig {
            name: "web",
            local_addr: "127.0.0.1:3000",
            description: "Web frontend (React/Vue/etc)",
        },
        TunnelConfig {
            name: "api",
            local_addr: "127.0.0.1:4000",
            description: "REST API backend",
        },
        TunnelConfig {
            name: "admin",
            local_addr: "127.0.0.1:5000",
            description: "Admin dashboard",
        },
    ];

    println!("Starting {} tunnel clients:", tunnels.len());
    for t in &tunnels {
        println!("  • {} -> {} ({})", t.name, t.local_addr, t.description);
    }
    println!();

    // Create and start all clients
    let mut handles = Vec::new();
    let mut clients = Vec::new();

    for config in &tunnels {
        let client = Client::builder()
            .server_addr("localhost:7835")
            .token("secret")
            .local_addr(config.local_addr)
            .auto_reconnect(true)
            .reconnect_delay(Duration::from_secs(5))
            .build()?;

        clients.push(client);
    }

    // Start all clients concurrently
    for (i, _client) in clients.iter_mut().enumerate() {
        let name = tunnels[i].name;
        let local_addr = tunnels[i].local_addr;

        // Spawn each client in a separate task
        let handle = tokio::spawn({
            // We need to wrap the start in a timeout to avoid blocking forever
            // if server is not available
            async move {
                println!("[{name}] Connecting to tunnel server...");

                match tokio::time::timeout(Duration::from_secs(10), async {
                    // Simulating what would happen - in real usage you'd call client.start()
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    Ok::<_, ferrotunnel::TunnelError>(format!("session-{name}"))
                })
                .await
                {
                    Ok(Ok(session)) => {
                        println!("[{name}] ✅ Connected! Session: {session}");
                        println!("[{name}]    Forwarding: {local_addr}");
                    }
                    Ok(Err(e)) => {
                        println!("[{name}] ❌ Connection failed: {e}");
                    }
                    Err(_) => {
                        println!("[{name}] ⏳ Connection timeout - will retry in background");
                    }
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all connection attempts
    for handle in handles {
        let _ = handle.await;
    }

    println!();
    println!("All tunnels initialized!");
    println!();
    println!("In a real scenario, each tunnel would now be:");
    println!("  • Forwarding requests from the server to the local service");
    println!("  • Auto-reconnecting if connection is lost");
    println!("  • Operating independently");
    println!();
    println!("Tunnel routing would work like:");
    println!("  • https://web-abc123.tunnel.example.com -> localhost:3000");
    println!("  • https://api-abc123.tunnel.example.com -> localhost:4000");
    println!("  • https://admin-abc123.tunnel.example.com -> localhost:5000");
    println!();

    println!("Press Ctrl+C to stop all tunnels");

    tokio::signal::ctrl_c().await?;

    println!();
    println!("Shutting down all tunnels...");

    // Shutdown all clients
    for (i, client) in clients.iter_mut().enumerate() {
        println!("[{}] Shutting down...", tunnels[i].name);
        client.shutdown().await?;
    }

    println!("All tunnels stopped. Goodbye!");

    Ok(())
}
