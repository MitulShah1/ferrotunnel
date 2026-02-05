//! Configuration types for `FerroTunnel` hardening

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    /// Enable TLS
    pub enabled: bool,
    /// Path to CA certificate (for client verification or server trust)
    pub ca_cert_path: Option<PathBuf>,
    /// Path to certificate file
    pub cert_path: Option<PathBuf>,
    /// Path to private key file
    pub key_path: Option<PathBuf>,
    /// Server name for SNI (client-side)
    pub server_name: Option<String>,
    /// Require client certificate authentication
    pub client_auth: bool,
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    /// Maximum frame size in bytes (default: 16MB)
    pub max_frame_bytes: u64,
    /// Maximum concurrent sessions per server
    pub max_sessions: usize,
    /// Maximum streams per session
    pub max_streams_per_session: usize,
    /// Maximum in-flight frames per session
    pub max_inflight_frames: usize,
    /// Maximum token length in bytes
    pub max_token_len: usize,
    /// Maximum number of capabilities
    pub max_capabilities: usize,
    /// Maximum capability string length
    pub max_capability_len: usize,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_frame_bytes: 16 * 1024 * 1024, // 16MB
            max_sessions: 1000,
            max_streams_per_session: 100,
            max_inflight_frames: 100,
            max_token_len: 256,
            max_capabilities: 32,
            max_capability_len: 64,
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum new streams per second per session
    pub streams_per_sec: u32,
    /// Maximum bytes per second per session
    pub bytes_per_sec: u64,
    /// Burst allowance multiplier
    pub burst_factor: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            streams_per_sec: 100,
            bytes_per_sec: 10 * 1024 * 1024, // 10MB/s
            burst_factor: 2,
        }
    }
}
