//! Prometheus metrics for FerroTunnel.
//!
//! Naming follows [Prometheus best practices](https://prometheus.io/docs/practices/naming/):
//! - **Counters**: suffix `_total` (e.g. `ferrotunnel_connections_total`).
//! - **Units**: include in the name (e.g. `_seconds` for duration, `_bytes` for size).
//! - **Gauges**: no `_total`; use descriptive names (e.g. `ferrotunnel_active_connections`).

use prometheus::{
    register_counter, register_counter_vec, register_gauge, register_histogram, Counter,
    CounterVec, Gauge, Histogram, Registry,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;

static METRICS_ENABLED: AtomicBool = AtomicBool::new(false);

pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

// Tunnel metrics (counters use _total; gauges do not)
pub static TOTAL_CONNECTIONS: LazyLock<Counter> = LazyLock::new(|| {
    register_counter!(
        "ferrotunnel_connections_total",
        "Total number of tunnel connections established"
    )
    .unwrap()
});

pub static ACTIVE_CONNECTIONS: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge!(
        "ferrotunnel_active_connections",
        "Number of currently active tunnel connections"
    )
    .unwrap()
});

// Data metrics (_bytes and _total)
pub static BYTES_TRANSFERRED_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    register_counter_vec!(
        "ferrotunnel_bytes_transferred_total",
        "Total bytes transferred through the tunnel",
        &["direction"] // "ingress", "egress"
    )
    .unwrap()
});

// Request metrics
pub static TOTAL_REQUESTS: LazyLock<Counter> = LazyLock::new(|| {
    register_counter!(
        "ferrotunnel_requests_total",
        "Total number of HTTP requests processed"
    )
    .unwrap()
});

pub static REQUEST_LATENCY: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "ferrotunnel_request_duration_seconds",
        "HTTP request latency in seconds"
    )
    .unwrap()
});

pub static ACTIVE_STREAMS: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge!(
        "ferrotunnel_active_streams",
        "Number of currently active multiplexed streams"
    )
    .unwrap()
});

// Error metrics (_total for counter)
pub static ERRORS_TOTAL: LazyLock<CounterVec> = LazyLock::new(|| {
    register_counter_vec!(
        "ferrotunnel_errors_total",
        "Total number of errors by type",
        &["type"] // "connection", "request", "plugin", "protocol"
    )
    .unwrap()
});

/// Initialize the metrics system
pub fn init_metrics() {
    // Explicitly trigger LazyLock initialization
    let _ = LazyLock::force(&REGISTRY);
    METRICS_ENABLED.store(true, Ordering::Relaxed);
    tracing::info!("Metrics infrastructure initialized");
}

/// Returns true if metrics collection is enabled.
pub fn metrics_enabled() -> bool {
    METRICS_ENABLED.load(Ordering::Relaxed)
}

/// Gather all metrics into Prometheus format
pub fn gather_metrics() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
