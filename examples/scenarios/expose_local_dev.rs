//! Example: Expose Local Development Server
//!
//! Use FerroTunnel to expose your local dev server (e.g. React, Next.js) so you can:
//! - Share work-in-progress with teammates
//! - Test mobile apps against your local backend
//! - Debug integrations with external services
//! - Demo features before deployment
//!
//! # Usage
//!
//! ```bash
//! cargo run --example expose_local_dev
//! cargo run --example expose_local_dev -- --local 127.0.0.1:8000 --server tunnel.example.com:7835
//! ```

use ferrotunnel::Client;
use std::env;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .compact()
        .init();

    let args: Vec<String> = env::args().collect();
    let server_addr = get_arg(&args, "--server").unwrap_or_else(|| "localhost:7835".to_string());
    let local_addr = get_arg(&args, "--local").unwrap_or_else(|| "127.0.0.1:3000".to_string());
    let token = get_arg(&args, "--token").unwrap_or_else(|| "dev-tunnel".to_string());

    println!();
    println!("  ╔═══════════════════════════════════════╗");
    println!("  ║      FerroTunnel Development Mode     ║");
    println!("  ╚═══════════════════════════════════════╝");
    println!();
    println!("  Forwarding: {} -> tunnel -> internet", local_addr);
    println!();

    let mut client = Client::builder()
        .server_addr(&server_addr)
        .token(&token)
        .local_addr(&local_addr)
        .build()?;

    match client.start().await {
        Ok(info) => {
            println!("  ✓ Connected!");
            println!("  ✓ Session: {:?}", info.session_id);
            println!();
            println!(
                "  Your local server at {} is now accessible via the tunnel.",
                local_addr
            );
            println!();
            println!("  Tips:");
            println!("  • Make sure your dev server is running on {}", local_addr);
            println!("  • Share the tunnel URL with teammates for testing");
            println!("  • Press Ctrl+C to disconnect");
            println!();
        }
        Err(e) => {
            eprintln!("  ✗ Failed to connect: {}", e);
            eprintln!();
            eprintln!("  Troubleshooting:");
            eprintln!("  • Is the tunnel server running at {}?", server_addr);
            eprintln!("  • Is the token correct?");
            eprintln!("  • Check firewall settings");
            return Err(e);
        }
    }

    tokio::signal::ctrl_c().await?;

    println!();
    println!("  Disconnecting...");
    client.shutdown().await?;
    println!("  Goodbye!");

    Ok(())
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}
