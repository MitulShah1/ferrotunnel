//! Configuration types for `FerroTunnel` client and server.
//!
//! These types provide type-safe configuration for embedding `FerroTunnel`
//! in your applications.

use ferrotunnel_common::{Result, TunnelError};
use std::net::SocketAddr;
use std::time::Duration;

/// Configuration for the tunnel client.
///
/// Use [`ClientBuilder`](crate::ClientBuilder) for ergonomic construction.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Server address to connect to (host:port)
    pub server_addr: String,

    /// Authentication token
    pub token: String,

    /// Local address to forward traffic to
    pub local_addr: String,

    /// Enable automatic reconnection on disconnect
    pub auto_reconnect: bool,

    /// Delay between reconnection attempts
    pub reconnect_delay: Duration,
}

impl ClientConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        if self.server_addr.is_empty() {
            return Err(TunnelError::Config("server_addr is required".into()));
        }
        if self.token.is_empty() {
            return Err(TunnelError::Config("token is required".into()));
        }
        if self.local_addr.is_empty() {
            return Err(TunnelError::Config("local_addr is required".into()));
        }
        Ok(())
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_addr: String::new(),
            token: String::new(),
            local_addr: "127.0.0.1:8080".to_string(),
            auto_reconnect: true,
            reconnect_delay: Duration::from_secs(5),
        }
    }
}

/// Configuration for the tunnel server.
///
/// Use [`ServerBuilder`](crate::ServerBuilder) for ergonomic construction.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind the tunnel control plane
    pub bind_addr: SocketAddr,

    /// Address to bind the HTTP ingress
    pub http_bind_addr: SocketAddr,

    /// Authentication token (clients must provide this)
    pub token: String,
}

impl ServerConfig {
    /// Validate the configuration.
    pub fn validate(&self) -> Result<()> {
        if self.token.is_empty() {
            return Err(TunnelError::Config("token is required".into()));
        }
        Ok(())
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: ([0, 0, 0, 0], 7835).into(),
            http_bind_addr: ([0, 0, 0, 0], 8080).into(),
            token: String::new(),
        }
    }
}

/// Information about an established tunnel connection.
#[derive(Debug, Clone)]
pub struct TunnelInfo {
    /// The session ID assigned by the server.
    ///
    /// Note: Currently this is a client-generated placeholder until
    /// the core library exposes the server-assigned session ID.
    pub session_id: Option<uuid::Uuid>,

    /// The public URL where the tunnel is accessible (if applicable)
    pub public_url: Option<String>,
}
