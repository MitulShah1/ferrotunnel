//! Example: Server with Full Observability
//!
//! Runs a FerroTunnel server with Prometheus metrics, structured logging, and
//! optional tracing. Use this when you need to monitor the server in deployment.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example server_observability
//! OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 cargo run --example server_observability
//! ```

use ferrotunnel::Server;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,ferrotunnel=debug".to_string()),
        )
        .init();

    println!("FerroTunnel Server with Full Observability");
    println!("==========================================");
    println!();
    println!("Endpoints:");
    println!("  Tunnel:  0.0.0.0:7835");
    println!("  HTTP:    0.0.0.0:8080");
    println!("  Metrics: 0.0.0.0:9090/metrics");
    println!();
    println!("Example Prometheus queries:");
    println!("  rate(ferrotunnel_requests_total[5m])");
    println!("  ferrotunnel_active_tunnels");
    println!();

    let mut server = Server::builder()
        .bind("0.0.0.0:7835".parse().expect("valid address"))
        .http_bind("0.0.0.0:8080".parse().expect("valid address"))
        .token(&std::env::var("FERROTUNNEL_TOKEN").unwrap_or_else(|_| "metrics-demo".to_string()))
        .build()?;

    tracing::info!("Starting server with metrics enabled");
    tracing::info!(
        target: "ferrotunnel::metrics",
        "Prometheus metrics available at http://0.0.0.0:9090/metrics"
    );

    server.start().await?;

    Ok(())
}
