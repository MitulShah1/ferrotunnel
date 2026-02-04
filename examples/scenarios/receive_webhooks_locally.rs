//! Example: Receive Webhooks on Your Local Machine
//!
//! Forward webhooks from external services (GitHub, Stripe, etc.) to your local
//! development server so you can develop and test webhook integrations.
//!
//! # Usage
//!
//! 1. Start your local webhook handler on port 3000.
//! 2. Run this example:
//!
//! ```bash
//! cargo run --example receive_webhooks_locally -- --server tunnel.example.com:7835
//! ```
//!
//! 3. Configure the external service to send webhooks to your tunnel URL.

use ferrotunnel::Client;
use std::env;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,ferrotunnel=debug")
        .init();

    let args: Vec<String> = env::args().collect();
    let server_addr = get_arg(&args, "--server").unwrap_or_else(|| "localhost:7835".to_string());
    let local_addr = get_arg(&args, "--local").unwrap_or_else(|| "127.0.0.1:3000".to_string());
    let token = get_arg(&args, "--token").unwrap_or_else(|| "webhook-token".to_string());

    println!("FerroTunnel Webhook Receiver");
    println!("============================");
    println!();
    println!("This client will forward incoming HTTP requests to your local webhook handler.");
    println!();
    println!("Configuration:");
    println!("  Tunnel Server: {}", server_addr);
    println!("  Local Handler: {}", local_addr);
    println!();
    println!(
        "Make sure your webhook handler is running at {}",
        local_addr
    );
    println!();

    let mut client = Client::builder()
        .server_addr(&server_addr)
        .token(&token)
        .local_addr(&local_addr)
        .build()?;

    println!("Connecting to tunnel server...");

    let info = client.start().await?;
    println!();
    println!("Connected!");
    println!("Session ID: {:?}", info.session_id);
    println!();
    println!("Your webhook URL is ready. Configure your external service to send");
    println!("webhooks to your tunnel server's HTTP endpoint.");
    println!();
    println!("Example webhook configuration:");
    println!("  URL: http://<your-tunnel-server>:8080/webhook");
    println!();
    println!("Waiting for webhooks... (Press Ctrl+C to stop)");

    tokio::signal::ctrl_c().await?;

    println!();
    println!("Shutting down...");
    client.shutdown().await?;

    Ok(())
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}
