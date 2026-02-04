use bytes::Bytes;
use ferrotunnel_core::stream::multiplexer::VirtualStream;
use ferrotunnel_core::transport::socket_tuning::configure_socket_silent;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use hyper_util::service::TowerToHyperService;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::TcpStream;
use tower::{Layer, Service};
#[derive(Debug)]
pub enum ProxyError {
    Hyper(hyper::Error),
    Custom(String),
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyError::Hyper(e) => write!(f, "Hyper error: {e}"),
            ProxyError::Custom(s) => write!(f, "Proxy error: {s}"),
        }
    }
}

impl std::error::Error for ProxyError {}

impl From<hyper::Error> for ProxyError {
    fn from(e: hyper::Error) -> Self {
        ProxyError::Hyper(e)
    }
}

impl From<std::convert::Infallible> for ProxyError {
    fn from(_: std::convert::Infallible) -> Self {
        unreachable!()
    }
}

use tracing::error;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, ProxyError>;

/// Service that forwards requests to a local TCP port.
#[derive(Clone)]
pub struct LocalProxyService {
    target_addr: String,
}

impl LocalProxyService {
    pub fn new(target_addr: String) -> Self {
        Self { target_addr }
    }
}

use hyper::body::Body;

impl<B> Service<Request<B>> for LocalProxyService
where
    B: Body + Send + Sync + 'static,
    B::Data: Send,
    B::Error: Into<ProxyError>,
{
    type Response = Response<BoxBody>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let target = self.target_addr.clone();
        Box::pin(async move {
            let stream = match TcpStream::connect(&target).await {
                Ok(s) => {
                    configure_socket_silent(&s);
                    s
                }
                Err(e) => {
                    error!("Failed to connect to local service {target}: {e}");
                    return Ok(error_response(
                        StatusCode::BAD_GATEWAY,
                        &format!("Failed to connect to local service: {e}"),
                    ));
                }
            };

            let io = TokioIo::new(stream);
            let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
                Ok(h) => h,
                Err(e) => {
                    error!("Local handshake failed: {e}");
                    return Ok(error_response(
                        StatusCode::BAD_GATEWAY,
                        &format!("Local handshake failed: {e}"),
                    ));
                }
            };

            tokio::spawn(async move {
                if let Err(e) = conn.await {
                    error!("Connection error: {:?}", e);
                }
            });

            // Map generic body to BoxBody for hyper client
            let req = req.map(|b| BodyExt::boxed(b).map_err(Into::into));

            match sender.send_request(req).await {
                Ok(res) => {
                    let (parts, body) = res.into_parts();
                    // Map hyper::Error to ProxyError
                    let boxed_body = body.map_err(Into::into).boxed();
                    Ok(Response::from_parts(parts, boxed_body))
                }
                Err(e) => {
                    error!("Failed to proxy request: {e}");
                    Ok(error_response(StatusCode::BAD_GATEWAY, "Proxy error"))
                }
            }
        })
    }
}

/// Pre-allocated bytes for common error bodies (avoids allocation in hot/error path).
const MSG_PROXY_ERROR: &[u8] = b"Proxy error";
const MSG_INTERNAL_ERROR: &[u8] = b"Internal error";

/// Builds a plain-text error response. Shared by proxy and CLI dashboard middleware.
/// Uses static bytes for common messages to avoid allocation.
pub fn error_response(status: StatusCode, msg: &str) -> Response<BoxBody> {
    let bytes = if msg == "Proxy error" {
        Bytes::from_static(MSG_PROXY_ERROR)
    } else {
        Bytes::copy_from_slice(msg.as_bytes())
    };
    Response::builder()
        .status(status)
        .body(
            Full::new(bytes)
                .map_err(|_| ProxyError::Custom("Error construction failed".into()))
                .boxed(),
        )
        .unwrap_or_else(|_| {
            Response::new(
                Full::new(Bytes::from_static(MSG_INTERNAL_ERROR))
                    .map_err(|_| ProxyError::Custom("Error construction failed".into()))
                    .boxed(),
            )
        })
}

#[derive(Clone)]
pub struct HttpProxy<L> {
    target_addr: String,
    layer: L,
}

impl HttpProxy<tower::layer::util::Identity> {
    pub fn new(target_addr: String) -> Self {
        Self {
            target_addr,
            layer: tower::layer::util::Identity::new(),
        }
    }
}

impl<L> HttpProxy<L> {
    pub fn with_layer<NewL>(self, layer: NewL) -> HttpProxy<NewL> {
        HttpProxy {
            target_addr: self.target_addr,
            layer,
        }
    }

    pub fn handle_stream(&self, stream: VirtualStream)
    where
        L: Layer<LocalProxyService> + Clone + Send + 'static,
        L::Service: Service<Request<Incoming>, Response = Response<BoxBody>, Error = hyper::Error>
            + Send
            + Clone
            + 'static,
        <L::Service as Service<Request<Incoming>>>::Future: Send,
    {
        let service = self
            .layer
            .clone()
            .layer(LocalProxyService::new(self.target_addr.clone()));
        let hyper_service = TowerToHyperService::new(service);
        let io = TokioIo::new(stream);

        tokio::spawn(async move {
            let _ = http1::Builder::new()
                .serve_connection(io, hyper_service)
                .await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use hyper::{body::Bytes, Request};
    use tower::Service;

    #[test]
    fn test_proxy_error_display_hyper() {
        // We can't easily create a real hyper error, but we can test Custom
        let err = ProxyError::Custom("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_proxy_error_custom_display() {
        let err = ProxyError::Custom("connection failed".to_string());
        let display = format!("{err}");
        assert!(display.contains("Proxy error"));
        assert!(display.contains("connection failed"));
    }

    #[test]
    fn test_local_proxy_service_new() {
        let service = LocalProxyService::new("127.0.0.1:8080".to_string());
        assert_eq!(service.target_addr, "127.0.0.1:8080");
    }

    #[test]
    fn test_local_proxy_service_clone() {
        let service = LocalProxyService::new("localhost:3000".to_string());
        let cloned = service.clone();
        assert_eq!(cloned.target_addr, "localhost:3000");
    }

    #[tokio::test]
    async fn test_proxy_connection_error() {
        // Create a service pointing to a closed port (assuming 127.0.0.1:12345 is closed)
        let mut service = LocalProxyService::new("127.0.0.1:12345".to_string());

        let req = Request::builder()
            .uri("http://example.com")
            .body(Full::new(Bytes::from("test")))
            .unwrap();

        // The service should return a 502 Bad Gateway response
        let response = service.call(req).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body_str.contains("Failed to connect"));
    }

    #[test]
    fn test_error_response_bad_gateway() {
        let resp = error_response(StatusCode::BAD_GATEWAY, "Backend unavailable");
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn test_error_response_not_found() {
        let resp = error_response(StatusCode::NOT_FOUND, "Route not found");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_error_response_internal_error() {
        let resp = error_response(StatusCode::INTERNAL_SERVER_ERROR, "Unexpected error");
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_http_proxy_new() {
        let proxy = HttpProxy::new("127.0.0.1:8080".to_string());
        assert_eq!(proxy.target_addr, "127.0.0.1:8080");
    }

    #[test]
    fn test_http_proxy_with_layer() {
        let proxy = HttpProxy::new("127.0.0.1:8080".to_string());
        let _layered = proxy.with_layer(tower::layer::util::Identity::new());
        // Just verify it compiles and runs
    }
}
