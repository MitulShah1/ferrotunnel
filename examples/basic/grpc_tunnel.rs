//! Example: Tunneling a local gRPC service with FerroTunnel
//!
//! FerroTunnel automatically detects gRPC traffic by inspecting the
//! `Content-Type: application/grpc` header and switches to HTTP/2
//! end-to-end, preserving gRPC trailers (`grpc-status`, `grpc-message`)
//! and supporting all RPC types: unary, server-streaming, client-streaming,
//! and bidirectional-streaming.
//!
//! No special configuration is required — just point the client at your
//! local gRPC server port.
//!
//! # Usage
//!
//! ```bash
//! # Start a local gRPC server on port 50051
//! # (any gRPC server works: tonic, grpc-go, grpc-python, ...)
//!
//! # Start a FerroTunnel server (or use a hosted one)
//! ferrotunnel server --bind 127.0.0.1:7835 --http-bind 0.0.0.0:8080 --token my-secret
//!
//! # Run this example
//! cargo run --example grpc_tunnel -- \
//!     --server localhost:7835 \
//!     --token my-secret \
//!     --local-addr 127.0.0.1:50051
//! ```

use ferrotunnel::Client;
use std::env;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,ferrotunnel=debug")
        .init();

    let args: Vec<String> = env::args().collect();
    let server_addr =
        get_arg(&args, "--server").unwrap_or_else(|| "localhost:7835".to_string());
    let token = get_arg(&args, "--token").unwrap_or_else(|| "secret".to_string());
    let local_addr =
        get_arg(&args, "--local-addr").unwrap_or_else(|| "127.0.0.1:50051".to_string());
    let tunnel_id = get_arg(&args, "--tunnel-id");

    println!("FerroTunnel — gRPC Tunnel Example");
    println!("==================================");
    println!("Server:          {server_addr}");
    println!("Local gRPC addr: {local_addr}");
    if let Some(ref id) = tunnel_id {
        println!("Tunnel ID:       {id}");
    }
    println!();

    let mut builder = Client::builder()
        .server_addr(&server_addr)
        .token(&token)
        .local_addr(&local_addr)
        .auto_reconnect(true);

    if let Some(id) = tunnel_id {
        builder = builder.tunnel_id(id);
    }

    let mut client = builder.build()?;

    println!("Connecting to FerroTunnel server...");
    let info = client.start().await?;

    println!("Tunnel active!");
    if let Some(id) = info.session_id {
        println!(
            "\nRoute gRPC clients to this tunnel via the server's HTTP ingress.\n\
             Set the gRPC target host header to: {id}\n"
        );
        println!(
            "Example (grpcurl):\n  grpcurl -H 'Host: {id}' <server-ingress-addr> \\\n    \
             list"
        );
    }

    println!("\nFerroTunnel automatically:");
    println!("  • Detects Content-Type: application/grpc");
    println!("  • Uses HTTP/2 end-to-end (required by gRPC)");
    println!("  • Preserves gRPC trailers (grpc-status, grpc-message)");
    println!("  • Supports all RPC types including streaming");

    // Keep the tunnel alive until Ctrl-C
    println!("\nPress Ctrl-C to stop.");
    tokio::signal::ctrl_c().await.ok();
    println!("Shutting down.");

    Ok(())
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}
