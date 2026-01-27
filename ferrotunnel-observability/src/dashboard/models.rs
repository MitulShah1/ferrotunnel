//! Data models for the Dashboard API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Tunnel connection status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelStatus {
    Connecting,
    Connected,
    Disconnected,
}

/// Extended tunnel information for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardTunnelInfo {
    pub id: Uuid,
    pub subdomain: Option<String>,
    pub public_url: Option<String>,
    pub local_addr: String,
    pub created_at: DateTime<Utc>,
    pub status: TunnelStatus,
}

/// Summary of a request for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLogEntry {
    pub id: Uuid,
    pub tunnel_id: Uuid,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

/// Full request details including headers and bodies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestDetails {
    pub id: Uuid,
    pub tunnel_id: Uuid,
    pub method: String,
    pub path: String,
    pub request_headers: HashMap<String, String>,
    pub request_body: Option<String>,
    pub status: u16,
    pub response_headers: HashMap<String, String>,
    pub response_body: Option<String>,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

impl From<&RequestDetails> for RequestLogEntry {
    fn from(details: &RequestDetails) -> Self {
        Self {
            id: details.id,
            tunnel_id: details.tunnel_id,
            method: details.method.clone(),
            path: details.path.clone(),
            status: details.status,
            duration_ms: details.duration_ms,
            timestamp: details.timestamp,
        }
    }
}

/// API error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Shared dashboard state containing tunnels and request history.
#[derive(Debug)]
pub struct DashboardState {
    pub tunnels: HashMap<Uuid, DashboardTunnelInfo>,
    pub requests: VecDeque<RequestDetails>,
    pub max_requests: usize,
}

impl DashboardState {
    /// Creates a new dashboard state with the specified max request history.
    pub fn new(max_requests: usize) -> Self {
        Self {
            tunnels: HashMap::new(),
            requests: VecDeque::with_capacity(max_requests),
            max_requests,
        }
    }

    /// Adds a request to the history, evicting oldest if at capacity.
    pub fn add_request(&mut self, request: RequestDetails) {
        if self.requests.len() >= self.max_requests {
            self.requests.pop_front();
        }
        self.requests.push_back(request);
    }

    /// Adds or updates a tunnel in the state.
    pub fn add_tunnel(&mut self, tunnel: DashboardTunnelInfo) {
        self.tunnels.insert(tunnel.id, tunnel);
    }

    /// Removes a tunnel from the state.
    pub fn remove_tunnel(&mut self, id: Uuid) -> Option<DashboardTunnelInfo> {
        self.tunnels.remove(&id)
    }
}

/// Thread-safe shared dashboard state.
pub type SharedDashboardState = Arc<RwLock<DashboardState>>;
