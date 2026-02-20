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
    ApiError, DashboardTunnelInfo, HealthResponse, RequestLogEntry, SharedDashboardState,
};
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

/// Programmatically create a new tunnel entry via the dashboard.
///
/// POST /api/v1/tunnels
pub async fn create_tunnel_handler(
    State(state): State<SharedDashboardState>,
    Json(payload): Json<super::models::CreateTunnelRequest>,
) -> Response {
    let new_id = Uuid::new_v4();
    let tunnel = DashboardTunnelInfo {
        id: new_id,
        subdomain: payload.subdomain,
        public_url: payload.public_url,
        local_addr: payload.local_addr,
        created_at: chrono::Utc::now(),
        status: super::models::TunnelStatus::Connecting,
    };

    let mut state_writer = state.write().await;
    state_writer.add_tunnel(tunnel.clone());

    (StatusCode::CREATED, Json(tunnel)).into_response()
}

/// Remove a tunnel entry by ID.
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

    let mut state_writer = state.write().await;
    if state_writer.remove_tunnel(id).is_some() {
        StatusCode::NO_CONTENT.into_response()
    } else {
        error_response(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("Tunnel with id '{}' not found", id),
        )
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
    use crate::dashboard::models::{DashboardState, TunnelStatus};
    use axum::body::to_bytes;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Helper to setup a test dashboard state.
    fn test_state() -> SharedDashboardState {
        Arc::new(RwLock::new(DashboardState::new(100)))
    }

    #[tokio::test]
    async fn test_create_tunnel_handler() {
        let state = test_state();
        let payload = super::super::models::CreateTunnelRequest {
            subdomain: Some("test-svc".to_string()),
            local_addr: "127.0.0.1:8080".to_string(),
            public_url: None,
        };

        let response = create_tunnel_handler(State(state.clone()), Json(payload)).await;

        // Verify status code is CREATED
        assert_eq!(response.status(), StatusCode::CREATED);

        // Parse response body
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let created_info: DashboardTunnelInfo = serde_json::from_slice(&body_bytes).unwrap();

        // Check fields match
        assert_eq!(created_info.subdomain.as_deref(), Some("test-svc"));
        assert_eq!(created_info.local_addr, "127.0.0.1:8080");
        assert_eq!(created_info.status, TunnelStatus::Connecting);

        // Verify it was actually added to the state
        let state_guard = state.read().await;
        assert!(state_guard.tunnels.contains_key(&created_info.id));
    }

    #[tokio::test]
    async fn test_delete_tunnel_handler_success() {
        let state = test_state();
        let new_id = Uuid::new_v4();

        // 1. Arrange - Add a tunnel manually
        {
            let mut state_guard = state.write().await;
            state_guard.add_tunnel(DashboardTunnelInfo {
                id: new_id,
                subdomain: None,
                public_url: None,
                local_addr: "127.0.0.1:3000".to_string(),
                created_at: chrono::Utc::now(),
                status: TunnelStatus::Connecting,
            });
        }

        // 2. Act - Try to delete it
        let response = delete_tunnel_handler(State(state.clone()), Path(new_id.to_string())).await;

        // 3. Assert - Should return 204 NO_CONTENT
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Double check it's gone from state
        let state_guard = state.read().await;
        assert!(!state_guard.tunnels.contains_key(&new_id));
    }

    #[tokio::test]
    async fn test_delete_tunnel_handler_not_found() {
        let state = test_state();
        let random_id = Uuid::new_v4(); // Not in state

        let response =
            delete_tunnel_handler(State(state.clone()), Path(random_id.to_string())).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_tunnel_handler_invalid_uuid() {
        let state = test_state();

        let response =
            delete_tunnel_handler(State(state.clone()), Path("not-a-uuid".to_string())).await;

        // Assuming parsing fails gracefully and returns BAD_REQUEST (as impl currently does)
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
