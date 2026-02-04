pub mod metrics;
pub mod tracing;

#[cfg(feature = "dashboard")]
pub mod dashboard;

pub use metrics::{
    gather_metrics, init_metrics, metrics_enabled, tunnel_metrics, TunnelMetrics, REGISTRY,
};
pub use tracing::{init_tracing, shutdown_tracing, TracingConfig};

/// Basic initialization for minimal overhead
pub fn init_basic_observability(service_name: &str, enable_tracing: bool, enable_metrics: bool) {
    if enable_metrics {
        init_metrics();
    }

    if enable_tracing {
        let _ = init_tracing(TracingConfig {
            service_name: service_name.to_string(),
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
        });
    } else {
        init_minimal_logging();
    }
}

/// Minimal logging setup without metrics or OpenTelemetry infrastructure
/// Use this for latency-sensitive deployments where observability overhead matters
pub fn init_minimal_logging() {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}
