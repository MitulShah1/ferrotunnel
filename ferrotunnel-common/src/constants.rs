//! Default ports and addresses for FerroTunnel services.
//!
//! Use these constants instead of magic numbers so defaults stay consistent
//! across the main library, CLI, and examples.

/// Default port for the tunnel control plane (client–server protocol).
pub const DEFAULT_TUNNEL_PORT: u16 = 7835;

/// Default port for HTTP ingress (public traffic to tunneled services).
pub const DEFAULT_HTTP_PORT: u16 = 8080;

/// Default port for the metrics endpoint (e.g. Prometheus).
pub const DEFAULT_METRICS_PORT: u16 = 9090;

/// Default port for the observability dashboard.
pub const DEFAULT_DASHBOARD_PORT: u16 = 4040;

/// Default bind address for the tunnel control plane as a string (`0.0.0.0:7835`).
pub const DEFAULT_TUNNEL_BIND: &str = "0.0.0.0:7835";

/// Default bind address for HTTP ingress as a string (`0.0.0.0:8080`).
pub const DEFAULT_HTTP_BIND: &str = "0.0.0.0:8080";

/// Default local address for client forwarding as a string (`127.0.0.1:8080`).
pub const DEFAULT_LOCAL_ADDR: &str = "127.0.0.1:8080";
