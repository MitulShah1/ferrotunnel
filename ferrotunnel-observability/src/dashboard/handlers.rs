//! HTTP handlers for the Dashboard API.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use super::models::{
    ApiError, CreateTunnelRequest, DashboardTunnelInfo, HealthResponse, RequestLogEntry,
    SharedDashboardState, TunnelStatus,
};
use chrono::Utc;
use std::str::FromStr;

/// Creates a JSON error response with consistent format.
fn error_response(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    let error = ApiError {
        code: code.to_string(),
        message: message.into(),
    };
    (status, Json(serde_json::json!({ "error": error }))).into_response()
}

/// Health check endpoint.
///
/// GET /api/v1/health
pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// List all active tunnels.
///
/// GET /api/v1/tunnels
pub async fn list_tunnels_handler(
    State(state): State<SharedDashboardState>,
) -> Json<Vec<DashboardTunnelInfo>> {
    let state = state.read().await;
    let tunnels: Vec<DashboardTunnelInfo> = state.tunnels.values().cloned().collect();
    Json(tunnels)
}

/// Get a specific tunnel by ID.
///
/// GET /api/v1/tunnels/:id
pub async fn get_tunnel_handler(
    State(state): State<SharedDashboardState>,
    Path(id_str): Path<String>,
) -> Response {
    let id = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                format!("Invalid ID format: {}", e),
            );
        }
    };

    let state = state.read().await;
    match state.tunnels.get(&id) {
        Some(tunnel) => Json(tunnel.clone()).into_response(),
        None => error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("Tunnel with id '{}' not found", id),
        ),
    }
}

/// Query parameters for listing requests.
#[derive(Debug, Deserialize)]
pub struct ListRequestsQuery {
    /// Maximum number of requests to return (default: 50, max: 200).
    pub limit: Option<usize>,
    /// Filter by tunnel ID.
    pub tunnel_id: Option<Uuid>,
}

/// List recent requests.
///
/// GET /api/v1/requests
pub async fn list_requests_handler(
    State(state): State<SharedDashboardState>,
    Query(query): Query<ListRequestsQuery>,
) -> Json<Vec<RequestLogEntry>> {
    let limit = query.limit.unwrap_or(50).min(200);
    let state = state.read().await;

    let entries: Vec<RequestLogEntry> = state
        .requests
        .iter()
        .rev()
        .filter(|r| query.tunnel_id.is_none() || query.tunnel_id == Some(r.tunnel_id))
        .take(limit)
        .map(RequestLogEntry::from)
        .collect();

    Json(entries)
}

/// Get full details for a specific request.
///
/// GET /api/v1/requests/:id
/// Get full details for a specific request.
///
/// GET /api/v1/requests/:id
pub async fn get_request_handler(
    State(state): State<SharedDashboardState>,
    Path(id_str): Path<String>,
) -> Response {
    tracing::info!("Handling get_request with id: {}", id_str);
    let id = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Invalid UUID format: {}", e);
            return error_response(
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                format!("Invalid ID format: {}", e),
            );
        }
    };

    let state = state.read().await;
    match state.requests.iter().find(|r| r.id == id) {
        Some(request) => Json(request.clone()).into_response(),
        None => {
            tracing::warn!("Request {} not found in state", id);
            error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                format!("Request with id '{}' not found", id),
            )
        }
    }
}

/// Create a new tunnel entry.
///
/// POST /api/v1/tunnels
pub async fn create_tunnel_handler(
    State(state): State<SharedDashboardState>,
    Json(req): Json<CreateTunnelRequest>,
) -> Response {
    let tunnel = DashboardTunnelInfo {
        id: Uuid::new_v4(),
        subdomain: req.subdomain,
        local_addr: req.local_addr,
        public_url: req.public_url,
        created_at: Utc::now(),
        status: TunnelStatus::Connecting,
    };
    let mut state = state.write().await;
    state.add_tunnel(tunnel.clone());
    (StatusCode::CREATED, Json(tunnel)).into_response()
}

/// Delete a tunnel by ID.
///
/// DELETE /api/v1/tunnels/:id
pub async fn delete_tunnel_handler(
    State(state): State<SharedDashboardState>,
    Path(id_str): Path<String>,
) -> Response {
    let id = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                format!("Invalid ID format: {}", e),
            );
        }
    };

    let mut state = state.write().await;
    match state.remove_tunnel(id) {
        Some(_) => StatusCode::NO_CONTENT.into_response(),
        None => error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("Tunnel with id '{}' not found", id),
        ),
    }
}

/// Prometheus metrics endpoint.
///
/// GET /api/v1/metrics
pub async fn metrics_handler() -> Response {
    let metrics = crate::gather_metrics();
    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        metrics,
    )
        .into_response()
}

/// Replay a specific request.
///
/// POST /api/v1/requests/:id/replay
pub async fn replay_request_handler(
    State(state): State<SharedDashboardState>,
    Path(id_str): Path<String>,
) -> Response {
    let id = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                format!("Invalid ID format: {}", e),
            );
        }
    };

    // 1. Fetch request and tunnel info
    let (req_details, tunnel_addr) = {
        let state = state.read().await;
        let req = match state.requests.iter().find(|r| r.id == id) {
            Some(r) => r.clone(),
            None => {
                return error_response(
                    StatusCode::NOT_FOUND,
                    "NOT_FOUND",
                    format!("Request {} not found", id),
                );
            }
        };

        let tunnel = state.tunnels.get(&req.tunnel_id).cloned();
        (req, tunnel)
    };

    // 2. Determine target URL
    // If we have a tunnel record, use its local address.
    // If no tunnel record (maybe restart?), we fail or try to guess?
    // In our case, if tunnel is gone, we can't replay safely to "unknown".
    // But for debugging, maybe we assume the user knows.
    // However, the request details don't store the *original* local target, only the tunnel ID.
    // So we rely on the tunnel being active.
    let target_host = if let Some(t) = tunnel_addr {
        t.local_addr
    } else {
        // Fallback or error?
        // Let's error for safety.
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "TUNNEL_INACTIVE",
            "The tunnel for this request is no longer active",
        );
    };

    // Construct URL (assuming HTTP)
    let url = format!("http://{}{}", target_host, req_details.path);
    tracing::info!("Replaying request {} to {}", id, url);

    // 3. Prepare Client
    let client = reqwest::Client::new();
    let method = match reqwest::Method::from_str(&req_details.method) {
        Ok(m) => m,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_METHOD",
                "Invalid HTTP method",
            )
        }
    };

    let mut request_builder = client.request(method, &url);

    // Add Headers (skip some)
    for (k, v) in &req_details.request_headers {
        if k.eq_ignore_ascii_case("host") || k.eq_ignore_ascii_case("content-length") {
            continue;
        }
        request_builder = request_builder.header(k, v);
    }

    // Add Body
    if let Some(body) = req_details.request_body {
        request_builder = request_builder.body(body);
    }

    // 4. Send Request (fire and forget? or wait?)
    // User probably wants to know if it worked.
    match request_builder.send().await {
        Ok(res) => {
            let status = res.status();
            tracing::info!("Replay success: status {}", status);
            Json(serde_json::json!({
                "status": "replayed",
                "target": url,
                "response_status": status.as_u16()
            }))
            .into_response()
        }
        Err(e) => {
            tracing::error!("Replay failed: {}", e);
            error_response(
                StatusCode::BAD_GATEWAY,
                "REPLAY_FAILED",
                format!("Failed to replay request: {}", e),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::models::DashboardState;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn make_state() -> SharedDashboardState {
        Arc::new(RwLock::new(DashboardState::new(10)))
    }

    #[tokio::test]
    async fn test_create_tunnel_returns_201_with_tunnel() {
        let state = make_state();
        let req = CreateTunnelRequest {
            subdomain: Some("my-service".to_string()),
            local_addr: "127.0.0.1:3000".to_string(),
            public_url: Some("https://my-service.example.com".to_string()),
        };

        let response =
            create_tunnel_handler(State(state.clone()), Json(req.clone())).await;

        assert_eq!(response.status(), StatusCode::CREATED);

        // Tunnel should now be in state
        let locked = state.read().await;
        assert_eq!(locked.tunnels.len(), 1);
        let tunnel = locked.tunnels.values().next().unwrap();
        assert_eq!(tunnel.subdomain, req.subdomain);
        assert_eq!(tunnel.local_addr, req.local_addr);
        assert_eq!(tunnel.public_url, req.public_url);
        assert_eq!(tunnel.status, TunnelStatus::Connecting);
    }

    #[tokio::test]
    async fn test_delete_tunnel_returns_204_when_found() {
        let state = make_state();

        // Pre-populate a tunnel
        let tunnel = DashboardTunnelInfo {
            id: Uuid::new_v4(),
            subdomain: Some("test".to_string()),
            local_addr: "127.0.0.1:4000".to_string(),
            public_url: None,
            created_at: Utc::now(),
            status: TunnelStatus::Connected,
        };
        let tunnel_id = tunnel.id;
        state.write().await.add_tunnel(tunnel);

        let response =
            delete_tunnel_handler(State(state.clone()), Path(tunnel_id.to_string())).await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(state.read().await.tunnels.is_empty());
    }

    #[tokio::test]
    async fn test_delete_tunnel_returns_404_when_not_found() {
        let state = make_state();
        let missing_id = Uuid::new_v4();

        let response =
            delete_tunnel_handler(State(state.clone()), Path(missing_id.to_string())).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_tunnel_returns_400_for_invalid_id() {
        let state = make_state();

        let response =
            delete_tunnel_handler(State(state.clone()), Path("not-a-uuid".to_string())).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
