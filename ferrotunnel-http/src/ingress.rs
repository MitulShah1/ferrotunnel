use ferrotunnel_common::Result;
use ferrotunnel_core::tunnel::session::SessionStore;
use ferrotunnel_plugin::{PluginAction, PluginRegistry, RequestContext, ResponseContext};
use ferrotunnel_protocol::frame::Protocol;
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

/// Configuration for HTTP ingress limits and timeouts
#[derive(Debug, Clone)]
pub struct IngressConfig {
    /// Maximum concurrent connections (default: 10000)
    pub max_connections: usize,
    /// Maximum response body size in bytes (default: 100MB)
    pub max_response_size: usize,
    /// Timeout for upstream handshake (default: 10s)
    pub handshake_timeout: Duration,
    /// Timeout for upstream response (default: 60s)
    pub response_timeout: Duration,
}

impl Default for IngressConfig {
    fn default() -> Self {
        Self {
            max_connections: 10000,
            max_response_size: 100 * 1024 * 1024, // 100MB
            handshake_timeout: Duration::from_secs(10),
            response_timeout: Duration::from_secs(60),
        }
    }
}

pub struct HttpIngress {
    addr: SocketAddr,
    sessions: SessionStore,
    registry: Arc<PluginRegistry>,
    config: IngressConfig,
    connection_semaphore: Arc<Semaphore>,
}

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

impl HttpIngress {
    pub fn new(addr: SocketAddr, sessions: SessionStore, registry: Arc<PluginRegistry>) -> Self {
        Self::with_config(addr, sessions, registry, IngressConfig::default())
    }

    pub fn with_config(
        addr: SocketAddr,
        sessions: SessionStore,
        registry: Arc<PluginRegistry>,
        config: IngressConfig,
    ) -> Self {
        let connection_semaphore = Arc::new(Semaphore::new(config.max_connections));
        Self {
            addr,
            sessions,
            registry,
            config,
            connection_semaphore,
        }
    }

    pub async fn start(self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        info!("HTTP Ingress listening on {}", self.addr);

        loop {
            let (stream, peer_addr) = listener.accept().await?;

            // Acquire connection permit (limit concurrent connections)
            let Ok(permit) = self.connection_semaphore.clone().try_acquire_owned() else {
                warn!(
                    "Max connections reached, rejecting connection from {}",
                    peer_addr
                );
                drop(stream);
                continue;
            };

            let io = TokioIo::new(stream);
            let registry = self.registry.clone();
            let sessions = self.sessions.clone();
            let config = self.config.clone();

            tokio::spawn(async move {
                let _permit = permit; // Hold permit until connection closes

                if let Err(err) = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |req| {
                            handle_request(
                                req,
                                sessions.clone(),
                                registry.clone(),
                                peer_addr,
                                config.clone(),
                            )
                        }),
                    )
                    .await
                {
                    error!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

#[allow(clippy::too_many_lines, clippy::expect_used)]
async fn handle_request(
    req: Request<hyper::body::Incoming>,
    sessions: SessionStore,
    registry: Arc<PluginRegistry>,
    peer_addr: SocketAddr,
    config: IngressConfig,
) -> std::result::Result<Response<BoxBody>, hyper::Error> {
    // 0. Parse and normalize Host header
    let tunnel_id = match parse_and_normalize_host(req.headers().get("host")) {
        Ok(host) => host,
        Err(msg) => {
            return Ok(full_response(StatusCode::BAD_REQUEST, msg));
        }
    };

    let ctx = RequestContext {
        tunnel_id: tunnel_id.clone(),
        session_id: uuid::Uuid::new_v4().to_string(),
        remote_addr: peer_addr,
        timestamp: SystemTime::now(),
    };

    // 1. Run Request Hooks (On Headers Only - No Body Buffering)
    let (mut parts, body) = req.into_parts();

    // Create a temporary request with empty body for plugins to inspect headers
    let mut plugin_req = Request::from_parts(parts.clone(), ());

    match registry.execute_request_hooks(&mut plugin_req, &ctx).await {
        Ok(PluginAction::Continue | PluginAction::Modify { .. }) => {
            // If modified, update parts (headers/uri/method)
            // Note: Body modification is not supported in streaming mode yet
            let (new_parts, ()) = plugin_req.into_parts();
            parts = new_parts;
        }
        Ok(PluginAction::Reject { status, reason }) => {
            return Ok(full_response(
                StatusCode::from_u16(status).unwrap_or(StatusCode::FORBIDDEN),
                &reason,
            ));
        }
        Ok(PluginAction::Respond {
            status,
            headers,
            body,
        }) => {
            let mut res =
                Response::builder().status(StatusCode::from_u16(status).unwrap_or(StatusCode::OK));
            for (k, v) in headers {
                res = res.header(k, v);
            }
            return Ok(res
                .body(full_body(Bytes::from(body)))
                .expect("failed to build response"));
        }

        Err(e) => {
            error!("Plugin error: {}", e);
            return Ok(full_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Plugin processing error",
            ));
        }
    }

    // 2. Identify Target Session (Routing Fix)
    // FIX #27: Use get_by_tunnel_id instead of find_multiplexer
    // We try to find by exact match of host (tunnel_id).
    // If not found, we could fallback to find_multiplexer() ONLY for local dev/testing if needed,
    // but for security we should be strict.
    // However, for verify plan "Routing Fix", strict lookup is key.

    // We need to clone multiplexer from the Ref
    let multiplexer = if let Some(session) = sessions.get_by_tunnel_id(&tunnel_id) {
        if let Some(m) = &session.multiplexer {
            m.clone()
        } else {
            return Ok(full_response(StatusCode::BAD_GATEWAY, "Tunnel not ready"));
        }
    } else {
        // Fallback for "unknown" host or direct IP access (development mode?)
        // If we want to support default tunnel for testing, we can keep find_multiplexer logic
        // BUT strict multi-tenancy requires us to fail.
        // Let's assume strict for now as per issue description "Insecure Global Routing".
        return Ok(full_response(
            StatusCode::NOT_FOUND,
            format!("Tunnel '{tunnel_id}' not found").as_str(),
        ));
    };

    // Reconstruct request for forwarding using the ORIGINAL streaming body
    // FIX #28: No body buffering here.
    let forward_req = Request::from_parts(parts, body.boxed());

    // 3. Open Stream
    let stream = match multiplexer.open_stream(Protocol::HTTP).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to open stream: {}", e);
            return Ok(full_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to open stream",
            ));
        }
    };

    // 4. Handshake and Send Request (with timeout)
    let io = TokioIo::new(stream);

    let handshake_result = tokio::time::timeout(
        config.handshake_timeout,
        hyper::client::conn::http1::handshake(io),
    )
    .await;

    let (mut sender, conn) = match handshake_result {
        Ok(Ok(res)) => res,
        Ok(Err(e)) => {
            error!("Handshake failed: {}", e);
            return Ok(full_response(
                StatusCode::BAD_GATEWAY,
                "Tunnel handshake failed",
            ));
        }
        Err(_) => {
            error!("Handshake timeout");
            return Ok(full_response(
                StatusCode::GATEWAY_TIMEOUT,
                "Tunnel handshake timeout",
            ));
        }
    };

    tokio::spawn(async move {
        if let Err(err) = conn.await {
            error!("Connection failed: {:?}", err);
        }
    });

    // 5. Send Request and receive response (with timeout)
    let response_result =
        tokio::time::timeout(config.response_timeout, sender.send_request(forward_req)).await;

    let res = match response_result {
        Ok(Ok(res)) => res,
        Ok(Err(e)) => {
            error!("Failed to send request: {}", e);
            return Ok(full_response(
                StatusCode::BAD_GATEWAY,
                "Failed to send request",
            ));
        }
        Err(_) => {
            error!("Response timeout");
            return Ok(full_response(
                StatusCode::GATEWAY_TIMEOUT,
                "Upstream response timeout",
            ));
        }
    };

    let (parts, body) = res.into_parts();

    // Buffer response with size limit (for plugin processing)
    let body_bytes = match collect_body_with_limit(body, config.max_response_size).await {
        Ok(bytes) => bytes,
        Err(msg) => {
            error!("Response body error: {}", msg);
            return Ok(full_response(StatusCode::BAD_GATEWAY, msg));
        }
    };

    let mut proxy_res = Response::from_parts(parts, body_bytes.to_vec());

    let response_ctx = ResponseContext {
        tunnel_id: ctx.tunnel_id.clone(),
        session_id: ctx.session_id.clone(),
        status_code: proxy_res.status().as_u16(),
        duration_ms: u64::try_from(ctx.timestamp.elapsed().unwrap_or_default().as_millis())
            .unwrap_or(u64::MAX),
    };

    // Run Response Hooks
    match registry
        .execute_response_hooks(&mut proxy_res, &response_ctx)
        .await
    {
        Ok(PluginAction::Continue | _) => {}
        Err(e) => error!("Plugin response hook error: {}", e),
    }

    let (final_parts, final_body) = proxy_res.into_parts();
    let boxed_body = http_body_util::Full::new(Bytes::from(final_body))
        .map_err(|never| match never {})
        .boxed();

    Ok(Response::from_parts(final_parts, boxed_body))
}

/// Parse and normalize the Host header for secure multi-tenant routing.
/// Handles IPv6 addresses, port stripping, and case normalization.
fn parse_and_normalize_host(
    host_header: Option<&hyper::header::HeaderValue>,
) -> std::result::Result<String, &'static str> {
    let host_str = host_header
        .and_then(|h| h.to_str().ok())
        .ok_or("Missing or invalid Host header")?;

    if host_str.is_empty() {
        return Err("Empty Host header");
    }

    let host = host_str.trim();

    // Handle IPv6 addresses in bracket notation: [::1]:8080 -> ::1
    let normalized = if host.starts_with('[') {
        // IPv6 with brackets
        if let Some(bracket_end) = host.find(']') {
            // Extract the IPv6 address without brackets
            &host[1..bracket_end]
        } else {
            return Err("Invalid IPv6 Host header format");
        }
    } else if host.contains(':') && host.matches(':').count() > 1 {
        // IPv6 without brackets (unusual but possible)
        host
    } else {
        // IPv4 or hostname - strip port if present
        host.split(':').next().unwrap_or(host)
    };

    // Normalize: lowercase and strip trailing dot (FQDN format)
    let normalized = normalized.to_lowercase();
    let normalized = normalized.strip_suffix('.').unwrap_or(&normalized);

    if normalized.is_empty() {
        return Err("Empty host after normalization");
    }

    Ok(normalized.to_string())
}

/// Collect response body with a size limit to prevent denial-of-service attacks
async fn collect_body_with_limit(
    body: hyper::body::Incoming,
    max_size: usize,
) -> std::result::Result<Bytes, &'static str> {
    use http_body_util::BodyExt;

    let mut collected = Vec::new();
    let mut body = body;

    while let Some(frame_result) = body.frame().await {
        let Ok(frame) = frame_result else {
            return Err("Error reading response body");
        };

        if let Some(data) = frame.data_ref() {
            if collected.len() + data.len() > max_size {
                return Err("Response body too large");
            }
            collected.extend_from_slice(data);
        }
    }

    Ok(Bytes::from(collected))
}

fn full_response(status: StatusCode, body: &str) -> Response<BoxBody> {
    let bytes = Bytes::copy_from_slice(body.as_bytes());
    #[allow(clippy::unwrap_used)]
    Response::builder()
        .status(status)
        .body(
            http_body_util::Full::new(bytes)
                .map_err(|never| match never {})
                .boxed(),
        )
        .unwrap()
}

fn full_body(bytes: Bytes) -> BoxBody {
    http_body_util::Full::new(bytes)
        .map_err(|never| match never {})
        .boxed()
}

fn _empty_response(status: StatusCode) -> Response<BoxBody> {
    #[allow(clippy::unwrap_used)]
    Response::builder()
        .status(status)
        .body(Empty::new().map_err(|never| match never {}).boxed())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_host_simple() {
        let hv = hyper::header::HeaderValue::from_static("example.com");
        assert_eq!(parse_and_normalize_host(Some(&hv)).unwrap(), "example.com");
    }

    #[test]
    fn test_parse_host_with_port() {
        let hv = hyper::header::HeaderValue::from_static("example.com:8080");
        assert_eq!(parse_and_normalize_host(Some(&hv)).unwrap(), "example.com");
    }

    #[test]
    fn test_parse_host_uppercase() {
        let hv = hyper::header::HeaderValue::from_static("EXAMPLE.COM");
        assert_eq!(parse_and_normalize_host(Some(&hv)).unwrap(), "example.com");
    }

    #[test]
    fn test_parse_host_trailing_dot() {
        let hv = hyper::header::HeaderValue::from_static("example.com.");
        assert_eq!(parse_and_normalize_host(Some(&hv)).unwrap(), "example.com");
    }

    #[test]
    fn test_parse_host_ipv6() {
        let hv = hyper::header::HeaderValue::from_static("[::1]:8080");
        assert_eq!(parse_and_normalize_host(Some(&hv)).unwrap(), "::1");
    }

    #[test]
    fn test_parse_host_ipv4() {
        let hv = hyper::header::HeaderValue::from_static("192.168.1.1:3000");
        assert_eq!(parse_and_normalize_host(Some(&hv)).unwrap(), "192.168.1.1");
    }

    #[test]
    fn test_parse_host_empty() {
        let hv = hyper::header::HeaderValue::from_static("");
        assert!(parse_and_normalize_host(Some(&hv)).is_err());
    }

    #[test]
    fn test_parse_host_missing() {
        assert!(parse_and_normalize_host(None).is_err());
    }
}
