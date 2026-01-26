use ferrotunnel_common::Result;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

/// Initialize the Prometheus exporter and return the handle.
/// This handle is used to retrieve the current metrics state as a string.
pub fn setup_metrics_recorder() -> Result<PrometheusHandle> {
    let builder = PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .map_err(|e| ferrotunnel_common::TunnelError::Config(e.to_string()))?;
    Ok(handle)
}
