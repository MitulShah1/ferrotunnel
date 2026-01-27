pub mod metrics;
pub mod tracing;

#[cfg(feature = "dashboard")]
pub mod dashboard;

pub use metrics::{gather_metrics, init_metrics, REGISTRY};
pub use tracing::{init_tracing, shutdown_tracing, TracingConfig};

/// Basic initialization for minimal overhead
pub fn init_basic_observability(service_name: &str) {
    init_metrics();
    let _ = init_tracing(TracingConfig {
        service_name: service_name.to_string(),
        otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok(),
    });
}
