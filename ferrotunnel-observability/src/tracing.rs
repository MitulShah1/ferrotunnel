use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::Config;
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

/// Initialization options for tracing
pub struct TracingConfig {
    pub service_name: String,
    pub otlp_endpoint: Option<String>,
}

/// Initialize the tracing system with OpenTelemetry and Jaeger/OTLP support
pub fn init_tracing(config: TracingConfig) -> Result<(), anyhow::Error> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    // 1. Logging layer (Stdout/EnvFilter)
    // Default to "info" level only - "debug" adds overhead in hot paths
    // Use RUST_LOG=debug for development/debugging
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_ids(true)
        .with_target(true);

    // 2. OpenTelemetry layer (if endpoint provided)
    if let Some(endpoint) = config.otlp_endpoint {
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint);

        let resource = Resource::new(vec![KeyValue::new("service.name", config.service_name)]);

        let tracer_provider = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(exporter)
            .with_trace_config(Config::default().with_resource(resource))
            .install_simple()?;

        let tracer = tracer_provider.tracer("ferrotunnel");

        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .with(otel_layer)
            .init();
    } else {
        Registry::default().with(env_filter).with(fmt_layer).init();
    }

    tracing::info!("Tracing infrastructure initialized");
    Ok(())
}

/// Shutdown the tracing system and flush spans
pub fn shutdown_tracing() {
    global::shutdown_tracer_provider();
}
