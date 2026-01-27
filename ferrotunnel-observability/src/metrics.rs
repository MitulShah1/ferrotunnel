use lazy_static::lazy_static;
use prometheus::{
    register_counter, register_counter_vec, register_gauge, register_histogram, Counter,
    CounterVec, Gauge, Histogram, Registry,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    // Tunnel metrics
    pub static ref TOTAL_CONNECTIONS: Counter = register_counter!(
        "ferrotunnel_connections_total",
        "Total number of tunnel connections established"
    ).unwrap();

    pub static ref ACTIVE_CONNECTIONS: Gauge = register_gauge!(
        "ferrotunnel_active_connections",
        "Number of currently active tunnel connections"
    ).unwrap();

    // Data metrics
    pub static ref BYTES_TRANSFERRED_TOTAL: CounterVec = register_counter_vec!(
        "ferrotunnel_bytes_transferred_total",
        "Total bytes transferred through the tunnel",
        &["direction"] // "ingress", "egress"
    ).unwrap();

    // Request metrics
    pub static ref TOTAL_REQUESTS: Counter = register_counter!(
        "ferrotunnel_requests_total",
        "Total number of HTTP requests processed"
    ).unwrap();

    pub static ref REQUEST_LATENCY: Histogram = register_histogram!(
        "ferrotunnel_request_duration_seconds",
        "HTTP request latency in seconds"
    ).unwrap();

    pub static ref ACTIVE_STREAMS: Gauge = register_gauge!(
        "ferrotunnel_active_streams",
        "Number of currently active multiplexed streams"
    ).unwrap();
}

/// Initialize the metrics system
pub fn init_metrics() {
    // Explicitly trigger lazy_static initialization
    let _ = &*REGISTRY;
    tracing::info!("Metrics infrastructure initialized");
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
