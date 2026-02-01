//! Dashboard API module for tunnel inspection and monitoring.
//!
//! This module provides a REST API and SSE endpoint for the FerroTunnel dashboard.
//!
//! # Usage
//!
//! ```ignore
//! use ferrotunnel_observability::dashboard::{create_router, DashboardState, EventBroadcaster};
//! use std::sync::Arc;
//! use tokio::sync::RwLock;
//!
//! // Create shared state
//! let state = Arc::new(RwLock::new(DashboardState::new(1000)));
//! let broadcaster = Arc::new(EventBroadcaster::new(100));
//!
//! // Create the router
//! let app = create_router(state, broadcaster);
//!
//! // Run the server
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:4040").await?;
//! axum::serve(listener, app).await?;
//! ```

pub mod events;
pub mod handlers;
pub mod models;

pub use events::{DashboardEvent, EventBroadcaster};
pub use models::{
    ApiError, DashboardState, DashboardTunnelInfo, HealthResponse, RequestDetails, RequestLogEntry,
    SharedDashboardState, TunnelStatus,
};

use std::sync::Arc;

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

/// Creates the dashboard API router with all endpoints.
///
/// # Arguments
///
/// * `state` - Shared dashboard state for tunnel and request data.
/// * `broadcaster` - Event broadcaster for SSE streaming.
///
/// # Endpoints
///
/// - `GET /api/v1/health` - Health check
/// - `GET /api/v1/tunnels` - List all tunnels
/// - `GET /api/v1/tunnels/:id` - Get tunnel by ID
/// - `GET /api/v1/requests` - List recent requests
/// - `GET /api/v1/requests/:id` - Get request details
/// - `GET /api/v1/requests/:id/replay` - Replay a request
/// - `GET /api/v1/metrics` - Prometheus metrics
/// - `GET /api/v1/events` - SSE event stream
// Embedded assets
#[derive(rust_embed::RustEmbed)]
#[folder = "src/dashboard/static/"]
struct Assets;

async fn static_handler(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(axum::http::header::CONTENT_TYPE, mime.as_ref())],
                content.data,
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    }
}

pub fn create_router(state: SharedDashboardState, broadcaster: Arc<EventBroadcaster>) -> Router {
    let api_routes = Router::new()
        .route("/health", get(handlers::health_handler))
        .route("/tunnels", get(handlers::list_tunnels_handler))
        .route("/tunnels/:id", get(handlers::get_tunnel_handler))
        .route("/requests", get(handlers::list_requests_handler))
        .route("/requests/:id", get(handlers::get_request_handler))
        .route(
            "/requests/:id/replay",
            post(handlers::replay_request_handler),
        )
        .route("/metrics", get(handlers::metrics_handler))
        .with_state(state)
        .route("/events", get(events::events_handler))
        .with_state(broadcaster);

    Router::new()
        .nest("/api/v1", api_routes)
        .fallback(static_handler)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}

/// Configuration for the dashboard server.
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// Address to bind the dashboard server.
    pub bind_addr: std::net::SocketAddr,
    /// Maximum number of requests to keep in history.
    pub max_requests: usize,
    /// Optional authentication token.
    pub auth_token: Option<String>,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            bind_addr: ([127, 0, 0, 1], 4040).into(),
            max_requests: 1000,
            auth_token: None,
        }
    }
}
