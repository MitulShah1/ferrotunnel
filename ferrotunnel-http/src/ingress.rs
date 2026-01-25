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
use std::time::SystemTime;
use tokio::net::TcpListener;
use tracing::{error, info};

pub struct HttpIngress {
    addr: SocketAddr,
    sessions: SessionStore,
    registry: Arc<PluginRegistry>,
}

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

impl HttpIngress {
    pub fn new(addr: SocketAddr, sessions: SessionStore, registry: Arc<PluginRegistry>) -> Self {
        Self {
            addr,
            sessions,
            registry,
        }
    }

    pub async fn start(self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        info!("HTTP Ingress listening on {}", self.addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let registry = self.registry.clone();
            let sessions = self.sessions.clone();

            tokio::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |req| {
                            handle_request(req, sessions.clone(), registry.clone())
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
) -> std::result::Result<Response<BoxBody>, hyper::Error> {
    // 0. Prepare Plugin Context
    // We assume tunnel_id/session_id might be derived from Host header or subdomain.
    // For now we use placeholders or extract from Host.
    // Let's look for Host header.
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // In a real implementation we would extract tunnel_id from host (e.g. tunnel_id.ferrotunnel.com)
    // Here we'll just pass the host as tunnel_id for context.

    let ctx = RequestContext {
        tunnel_id: host.to_string(),
        session_id: uuid::Uuid::new_v4().to_string(), // Request ID essentially
        remote_addr: "0.0.0.0:0".parse().expect("valid address"), // We don't have remote addr here easily without passing it down
        timestamp: SystemTime::now(),
    };

    // Buffer Request Body for Plugins
    let (parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();
    let mut plugin_req = Request::from_parts(parts.clone(), body_bytes.to_vec());

    // 1. Run Request Hooks
    match registry.execute_request_hooks(&mut plugin_req, &ctx).await {
        Ok(PluginAction::Continue | PluginAction::Modify { .. }) => {
            // Modification not fully implemented yet in example, treat as Continue
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

    // Reconstruct request for forwarding (using potentially modified parts/body)
    // Note: plugin_req has Vec<u8> body. hyper::client needs Incoming or something simpler.
    // We can use Full<Bytes>.
    let (p_parts, p_body) = plugin_req.into_parts();
    let forward_req = Request::from_parts(p_parts, http_body_util::Full::new(Bytes::from(p_body)));

    // 2. Identify Target Session
    let Some(multiplexer) = sessions.find_multiplexer() else {
        return Ok(full_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "No active tunnels",
        ));
    };

    // 2. Open Stream
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

    // 3. Handshake and Send Request
    let io = TokioIo::new(stream);

    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(res) => res,
        Err(e) => {
            error!("Handshake failed: {}", e);
            return Ok(full_response(
                StatusCode::BAD_GATEWAY,
                "Tunnel handshake failed",
            ));
        }
    };

    tokio::spawn(async move {
        if let Err(err) = conn.await {
            error!("Connection failed: {:?}", err);
        }
    });

    // 4. Send Request (Header manipulation could happen here)
    // We might want to strip specific headers or add X-Forwarded-For.

    // 5. Send Request
    // We need to map the body type because `forward_req` uses Full<Bytes> but `send_request` expects Body.
    // However, `send_request` takes `B`, and we have `Full<Bytes>`. This should work if compatible.
    // Actually `sender` is bound to a specific body type?
    // `handshake` infers type? No.
    // `sender.send_request` takes `impl Body`.

    match sender.send_request(forward_req).await {
        Ok(res) => {
            let (parts, body) = res.into_parts();

            // Buffer response for plugins
            let body_bytes = body.collect().await?.to_bytes();
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
        Err(e) => {
            error!("Failed to send request: {}", e);
            Ok(full_response(
                StatusCode::BAD_GATEWAY,
                "Failed to send request",
            ))
        }
    }
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
