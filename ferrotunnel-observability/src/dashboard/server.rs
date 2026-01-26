use axum::{
    extract::State,
    routing::get,
    Router,
};
use ferrotunnel_common::Result;
use metrics_exporter_prometheus::PrometheusHandle;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

/// Start the dashboard HTTP server
pub async fn start_dashboard_server(addr: SocketAddr, metrics_handle: PrometheusHandle) -> Result<()> {
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(metrics_handle);

    info!("Dashboard listening on {}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn metrics_handler(State(handle): State<PrometheusHandle>) -> String {
    handle.render()
}
