//! Example: Server with Graceful Shutdown
//!
//! Demonstrates running a FerroTunnel server with signal handling (SIGTERM/SIGINT)
//! and graceful shutdown so in-flight requests can drain before exit.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example server_graceful_shutdown
//! ```

use ferrotunnel::Server;
use std::time::Duration;
use tokio::signal;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,ferrotunnel=info".to_string()),
        )
        .init();

    tracing::info!("Starting FerroTunnel server with graceful shutdown support");

    let mut server = Server::builder()
        .bind("0.0.0.0:7835".parse().expect("valid address"))
        .http_bind("0.0.0.0:8080".parse().expect("valid address"))
        .token(&std::env::var("FERROTUNNEL_TOKEN").unwrap_or_else(|_| "secret".to_string()))
        .build()?;

    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.start().await {
            tracing::error!("Server error: {}", e);
        }
    });

    tracing::info!("Server running. Press Ctrl+C or send SIGTERM to stop.");
    shutdown_signal().await;

    tracing::info!("Shutdown signal received, initiating graceful shutdown...");

    let shutdown_timeout = Duration::from_secs(30);
    tracing::info!(
        "Waiting up to {:?} for connections to drain",
        shutdown_timeout
    );

    tokio::select! {
        _ = tokio::time::sleep(shutdown_timeout) => {
            tracing::warn!("Shutdown timeout reached, forcing shutdown");
        }
        _ = server_handle => {
            tracing::info!("Server shut down gracefully");
        }
    }

    tracing::info!("FerroTunnel server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM");
        }
    }
}
