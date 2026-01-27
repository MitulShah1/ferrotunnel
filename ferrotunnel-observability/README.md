# ferrotunnel-observability 📊

Observability infrastructure for FerroTunnel, providing high-performance metrics and distributed tracing.

## Features

- **Prometheus Metrics**: Registry-based metrics collection for tunnel health, throughput, and latency.
- **OpenTelemetry Tracing**: Distributed tracing support with OTLP/gRPC exporter.
- **Structured Logging**: Pre-configured `tracing` subscriber with environment filtering.
- **Unified Initialization**: Simple API to set up monitoring with minimal boilerplate.

## Usage

### Initialization

```rust
use ferrotunnel_observability::{init_basic_observability, shutdown_tracing};

#[tokio::main]
async fn main() {
    // Initialize metrics and tracing
    init_basic_observability("my-service-name");

    // ... your application logic ...

    // Flush spans before exit
    shutdown_tracing();
}
```

### Metrics Endpoint (Axum)

If the `axum` feature is enabled, you can easily expose a `/metrics` endpoint:

```rust
use axum::{routing::get, Router};
use ferrotunnel_observability::gather_metrics;

let app = Router::new().route("/metrics", get(|| async { gather_metrics() }));
// serve app...
```

## Configuration

Tracing and metrics can be configured via environment variables:

- `RUST_LOG`: Log level (e.g., `info`, `debug`, `ferrotunnel=trace`).
- `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP collector endpoint (e.g., `http://localhost:4317`).

## Metrics List

- `ferrotunnel_connections_total`: Total tunnel connections.
- `ferrotunnel_active_connections`: Currently active connections.
- `ferrotunnel_bytes_transferred_total`: Bytes moved (labels: `direction`).
- `ferrotunnel_requests_total`: Total HTTP requests processed.
- `ferrotunnel_request_duration_seconds`: Request latency histogram.
- `ferrotunnel_active_streams`: Currently active multiplexed streams.
