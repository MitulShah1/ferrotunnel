//! Prometheus metrics for FerroTunnel.
//!
//! Naming follows [Prometheus best practices](https://prometheus.io/docs/practices/naming/):
//! - **Counters**: suffix `_total`
//! - **Units**: in the name (e.g. `_seconds`, `_bytes`)
//! - **Gauges**: descriptive names, no `_total`

use prometheus::{register_counter, register_gauge, register_histogram, Counter, Gauge, Histogram};
use std::sync::LazyLock;
use std::sync::OnceLock;
use std::time::Duration;

use prometheus::Registry;

/// Default registry (for custom collectors or dashboard). Tunnel metrics use the global default registry.
pub static REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

/// Global tunnel metrics. Set when [`init_metrics`] is called.
static TUNNEL_METRICS: OnceLock<TunnelMetrics> = OnceLock::new();

/// Tunnel-level metrics: frames, bytes, decode/encode latency, queue depth.
///
/// All values are exported to Prometheus when [`gather_metrics`] is called.
#[derive(Debug)]
pub struct TunnelMetrics {
    frames_processed: Counter,
    bytes_transferred: Counter,
    decode_latency: Histogram,
    encode_latency: Histogram,
    queue_depth: Gauge,
}

impl TunnelMetrics {
    /// Create and register metrics with the default Prometheus registry.
    pub fn new() -> Self {
        let frames_processed = register_counter!(
            "ferrotunnel_tunnel_frames_processed_total",
            "Total number of protocol frames processed (decoded or encoded)"
        )
        .expect("register ferrotunnel_tunnel_frames_processed_total");

        let bytes_transferred = register_counter!(
            "ferrotunnel_tunnel_bytes_transferred_total",
            "Total bytes transferred through the tunnel (data frames only)"
        )
        .expect("register ferrotunnel_tunnel_bytes_transferred_total");

        let decode_latency = register_histogram!(
            "ferrotunnel_tunnel_decode_latency_seconds",
            "Latency of decoding frames from the wire"
        )
        .expect("register ferrotunnel_tunnel_decode_latency_seconds");

        let encode_latency = register_histogram!(
            "ferrotunnel_tunnel_encode_latency_seconds",
            "Latency of encoding frames to the wire"
        )
        .expect("register ferrotunnel_tunnel_encode_latency_seconds");

        let queue_depth = register_gauge!(
            "ferrotunnel_tunnel_queue_depth",
            "Current number of frames queued in the batched sender"
        )
        .expect("register ferrotunnel_tunnel_queue_depth");

        Self {
            frames_processed,
            bytes_transferred,
            decode_latency,
            encode_latency,
            queue_depth,
        }
    }

    /// Record a decode operation (frames decoded, bytes, and latency).
    #[inline]
    pub fn record_decode(&self, frames: usize, bytes: usize, latency: Duration) {
        self.frames_processed.inc_by(frames as f64);
        self.bytes_transferred.inc_by(bytes as f64);
        self.decode_latency.observe(latency.as_secs_f64());
    }

    /// Record an encode operation (frames encoded, bytes, and latency).
    #[inline]
    pub fn record_encode(&self, frames: usize, bytes: usize, latency: Duration) {
        self.frames_processed.inc_by(frames as f64);
        self.bytes_transferred.inc_by(bytes as f64);
        self.encode_latency.observe(latency.as_secs_f64());
    }

    /// Set the current sender queue depth (gauge).
    #[inline]
    pub fn set_queue_depth(&self, depth: usize) {
        self.queue_depth.set(depth as f64);
    }

    /// Record bytes transferred (e.g. from TCP ingress bidirectional copy).
    #[inline]
    pub fn record_bytes(&self, bytes: usize) {
        self.bytes_transferred.inc_by(bytes as f64);
    }
}

impl Default for TunnelMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the global tunnel metrics, if metrics have been initialized.
#[inline]
pub fn tunnel_metrics() -> Option<&'static TunnelMetrics> {
    TUNNEL_METRICS.get()
}

/// Initialize the metrics system and register tunnel metrics.
pub fn init_metrics() {
    let _ = LazyLock::force(&REGISTRY);
    let _ = TUNNEL_METRICS.set(TunnelMetrics::new());
    tracing::info!("Metrics infrastructure initialized");
}

/// Returns true if metrics have been initialized.
#[inline]
pub fn metrics_enabled() -> bool {
    TUNNEL_METRICS.get().is_some()
}

/// Gather all metrics into Prometheus text format.
pub fn gather_metrics() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
