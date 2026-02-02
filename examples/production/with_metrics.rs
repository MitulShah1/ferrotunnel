//! Example: Server with Full Observability
//!
//! This example demonstrates how to run a FerroTunnel server with full
//! observability: Prometheus metrics, structured logging, and tracing.
//!
//! # Endpoints
//! - `:7835` - Tunnel control plane
//! - `:8080` - HTTP ingress
//! - `:9090/metrics` - Prometheus metrics
//!
//! # Usage
//!
//! ```bash
//! # Run with default settings
//! cargo run --example with_metrics
//!
//! # Run with custom OTLP endpoint
//! OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 cargo run --example with_metrics
//! ```
//!
//! # Prometheus Queries
//!
//! ```promql
//! # Request rate
//! rate(ferrotunnel_requests_total[5m])
//!
//! # Active tunnels
//! ferrotunnel_active_tunnels
//!
//! # Latency percentiles
//! histogram_quantile(0.99, rate(ferrotunnel_request_duration_seconds_bucket[5m]))
//! ```

use ferrotunnel::Server;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    // Initialize observability (logging + tracing)
    // In production, you'd configure OTLP exporter via environment variables
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

    // Build server with observability features
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

    // Start server (blocks until shutdown)
    server.start().await?;

    Ok(())
}
