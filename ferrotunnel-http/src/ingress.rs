use ferrotunnel_common::Result;
use ferrotunnel_core::tunnel::session::SessionStore;
use ferrotunnel_protocol::frame::Protocol;
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{error, info};

pub struct HttpIngress {
    addr: SocketAddr,
    sessions: SessionStore,
}

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

impl HttpIngress {
    pub fn new(addr: SocketAddr, sessions: SessionStore) -> Self {
        Self { addr, sessions }
    }

    pub async fn start(self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        info!("HTTP Ingress listening on {}", self.addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let sessions = self.sessions.clone();

            tokio::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |req| handle_request(req, sessions.clone())),
                    )
                    .await
                {
                    error!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    sessions: SessionStore,
) -> std::result::Result<Response<BoxBody>, hyper::Error> {
    // 1. Identify Target Session
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

    match sender.send_request(req).await {
        Ok(res) => {
            let (parts, body) = res.into_parts();
            let boxed_body = body.map_err(|e| e).boxed();
            Ok(Response::from_parts(parts, boxed_body))
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

fn _empty_response(status: StatusCode) -> Response<BoxBody> {
    #[allow(clippy::unwrap_used)]
    Response::builder()
        .status(status)
        .body(Empty::new().map_err(|never| match never {}).boxed())
        .unwrap()
}
