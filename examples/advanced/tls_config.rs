//! Example: TLS Configuration
#![allow(clippy::unwrap_used)]
//!
//! This example demonstrates how to configure TLS for secure tunnel connections.
//!
//! # Prerequisites
//!
//! Generate self-signed certificates for testing:
//! ```bash
//! # Generate CA key and certificate
//! openssl genrsa -out ca.key 4096
//! openssl req -new -x509 -key ca.key -sha256 -subj "/CN=FerroTunnel CA" -days 365 -out ca.crt
//!
//! # Generate server key and certificate
//! openssl genrsa -out server.key 4096
//! openssl req -new -key server.key -subj "/CN=localhost" -out server.csr
//! openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt -days 365
//! ```
//!
//! # Usage
//!
//! ```bash
//! # Start server with TLS
//! cargo run -p ferrotunnel-examples --example tls_config -- --mode server
//!
//! # In another terminal, start client with TLS
//! cargo run -p ferrotunnel-examples --example tls_config -- --mode client
//! ```

use ferrotunnel::common::config::TlsConfig;
use ferrotunnel::{Client, Server};
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,ferrotunnel=debug")
        .init();

    let args: Vec<String> = env::args().collect();
    let mode = get_arg(&args, "--mode").unwrap_or_else(|| "info".to_string());

    println!("`FerroTunnel` TLS Configuration Example");
    println!("========================================");
    println!();

    match mode.as_str() {
        "server" => run_server().await,
        "client" => run_client().await,
        _ => {
            print_usage();
            Ok(())
        }
    }
}

async fn run_server() -> ferrotunnel::Result<()> {
    println!("Starting TLS-enabled server...");
    println!();

    // Configure TLS for server
    let tls = TlsConfig {
        enabled: true,
        cert_path: Some(PathBuf::from("server.crt")),
        key_path: Some(PathBuf::from("server.key")),
        ca_cert_path: Some(PathBuf::from("ca.crt")), // For client certificate verification
        client_auth: false,                          // Set to true to require client certificates
        server_name: None,
    };

    // Check if certificate files exist
    if !tls.cert_path.as_ref().is_some_and(|p| p.exists()) {
        println!("⚠️  Certificate files not found!");
        println!();
        println!("Generate test certificates with:");
        println!("  openssl genrsa -out ca.key 4096");
        println!("  openssl req -new -x509 -key ca.key -sha256 -subj \"/CN=FerroTunnel CA\" -days 365 -out ca.crt");
        println!("  openssl genrsa -out server.key 4096");
        println!("  openssl req -new -key server.key -subj \"/CN=localhost\" -out server.csr");
        println!("  openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt -days 365");
        println!();
        println!("Running in demo mode (showing configuration only)...");
        demo_server_config(&tls);
        return Ok(());
    }

    let mut server = Server::builder()
        .bind("0.0.0.0:7835".parse().unwrap())
        .http_bind("0.0.0.0:8080".parse().unwrap())
        .token("secure-tls-token")
        .tls(tls)
        .build()?;

    println!("TLS Server Configuration:");
    println!("  Tunnel bind: 0.0.0.0:7835 (TLS)");
    println!("  HTTP bind: 0.0.0.0:8080");
    println!();
    println!("Press Ctrl+C to stop");

    server.start().await
}

async fn run_client() -> ferrotunnel::Result<()> {
    println!("Starting TLS-enabled client...");
    println!();

    // Configure TLS for client
    let tls = TlsConfig {
        enabled: true,
        cert_path: None, // Client certificate (optional, for mutual TLS)
        key_path: None,  // Client key (optional)
        ca_cert_path: Some(PathBuf::from("ca.crt")), // CA to verify server
        client_auth: false,
        server_name: Some("localhost".to_string()), // Must match server certificate CN
    };

    // Check if CA certificate exists
    if !tls.ca_cert_path.as_ref().is_some_and(|p| p.exists()) {
        println!("⚠️  CA certificate not found!");
        println!();
        println!("Running in demo mode (showing configuration only)...");
        demo_client_config(&tls);
        return Ok(());
    }

    let mut client = Client::builder()
        .server_addr("localhost:7835")
        .token("secure-tls-token")
        .local_addr("127.0.0.1:8000")
        .tls(tls)
        .auto_reconnect(false)
        .build()?;

    println!("TLS Client Configuration:");
    println!("  Server: localhost:7835 (TLS)");
    println!("  Local: 127.0.0.1:8000");
    println!();
    println!("Connecting...");

    let info = client.start().await?;
    println!("Connected! Session: {:?}", info.session_id);
    println!();
    println!("Press Ctrl+C to stop");

    tokio::signal::ctrl_c().await?;
    client.shutdown().await?;

    Ok(())
}

fn demo_server_config(tls: &TlsConfig) {
    println!();
    println!("Example server configuration:");
    println!("```rust");
    println!("let tls = TlsConfig {{");
    println!("    enabled: {},", tls.enabled);
    println!("    cert_path: Some(PathBuf::from(\"server.crt\")),");
    println!("    key_path: Some(PathBuf::from(\"server.key\")),");
    println!("    ca_cert_path: Some(PathBuf::from(\"ca.crt\")),");
    println!("    client_auth: {},", tls.client_auth);
    println!("    server_name: None,");
    println!("}};");
    println!();
    println!("let server = Server::builder()");
    println!("    .bind(\"0.0.0.0:7835\".parse().unwrap())");
    println!("    .http_bind(\"0.0.0.0:8080\".parse().unwrap())");
    println!("    .token(\"secure-token\")");
    println!("    .tls(tls)");
    println!("    .build()?;");
    println!("```");
}

fn demo_client_config(tls: &TlsConfig) {
    println!();
    println!("Example client configuration:");
    println!("```rust");
    println!("let tls = TlsConfig {{");
    println!("    enabled: {},", tls.enabled);
    println!("    cert_path: None,  // For mutual TLS");
    println!("    key_path: None,");
    println!("    ca_cert_path: Some(PathBuf::from(\"ca.crt\")),");
    println!("    client_auth: {},", tls.client_auth);
    println!("    server_name: Some(\"localhost\".to_string()),");
    println!("}};");
    println!();
    println!("let client = Client::builder()");
    println!("    .server_addr(\"localhost:7835\")");
    println!("    .token(\"secure-token\")");
    println!("    .local_addr(\"127.0.0.1:8000\")");
    println!("    .tls(tls)");
    println!("    .build()?;");
    println!("```");
}

fn print_usage() {
    println!("This example demonstrates TLS configuration for `FerroTunnel`.");
    println!();
    println!("Usage:");
    println!("  cargo run -p ferrotunnel-examples --example tls_config -- --mode server");
    println!("  cargo run -p ferrotunnel-examples --example tls_config -- --mode client");
    println!();
    println!("Security Features:");
    println!("  • TLS 1.3 encryption for tunnel control plane");
    println!("  • Certificate-based server authentication");
    println!("  • Optional mutual TLS (client certificates)");
    println!("  • Custom CA support for private PKI");
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}
