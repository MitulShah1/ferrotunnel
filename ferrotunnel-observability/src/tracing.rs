use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

/// Initialization options for tracing
pub struct TracingConfig {
    pub service_name: String,
    pub otlp_endpoint: Option<String>,
}

// Store the tracer provider for shutdown
static TRACER_PROVIDER: std::sync::OnceLock<SdkTracerProvider> = std::sync::OnceLock::new();

/// Initialize the tracing system with OpenTelemetry and Jaeger/OTLP support
pub fn init_tracing(config: TracingConfig) -> Result<(), anyhow::Error> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    // 1. Logging layer (Stdout/EnvFilter)
    // Default to "info" level only - "debug" adds overhead in hot paths
    // Use RUST_LOG=debug for development/debugging
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_ids(true)
        .with_target(true);

    // 2. OpenTelemetry layer (if endpoint provided)
    if let Some(endpoint) = config.otlp_endpoint {
        let exporter = SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()?;

        let resource = Resource::builder()
            .with_service_name(config.service_name)
            .build();

        let tracer_provider = SdkTracerProvider::builder()
            .with_resource(resource)
            .with_batch_exporter(exporter)
            .build();

        let tracer = tracer_provider.tracer("ferrotunnel");

        // Store the provider for shutdown
        let _ = TRACER_PROVIDER.set(tracer_provider.clone());

        // Set as global provider
        global::set_tracer_provider(tracer_provider);

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
    if let Some(provider) = TRACER_PROVIDER.get() {
        if let Err(e) = provider.shutdown() {
            tracing::error!("Failed to shutdown tracer provider: {}", e);
        }
    }
}
