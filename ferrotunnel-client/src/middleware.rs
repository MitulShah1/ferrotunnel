use bytes::Bytes;
use chrono::Utc;
use ferrotunnel_http::proxy::ProxyError;
use ferrotunnel_observability::dashboard::{
    DashboardEvent, EventBroadcaster, RequestDetails, SharedDashboardState,
};
use http_body_util::{BodyExt, Full};
use hyper::body::Body;
use hyper::{Request, Response, StatusCode};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use uuid::Uuid;

type BoxBody = http_body_util::combinators::BoxBody<Bytes, ProxyError>;

#[derive(Clone)]
pub struct DashboardCaptureLayer {
    pub state: SharedDashboardState,
    pub broadcaster: Arc<EventBroadcaster>,
    pub tunnel_id: Uuid,
}

impl<S> Layer<S> for DashboardCaptureLayer {
    type Service = DashboardCaptureService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DashboardCaptureService {
            inner,
            state: self.state.clone(),
            broadcaster: self.broadcaster.clone(),
            tunnel_id: self.tunnel_id,
        }
    }
}

#[derive(Clone)]
pub struct DashboardCaptureService<S> {
    inner: S,
    state: SharedDashboardState,
    broadcaster: Arc<EventBroadcaster>,
    tunnel_id: Uuid,
}

impl<S, B> Service<Request<B>> for DashboardCaptureService<S>
where
    S: Service<Request<BoxBody>, Response = Response<BoxBody>, Error = hyper::Error>
        + Send
        + Clone
        + 'static,
    S::Future: Send + 'static,
    B: Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<ProxyError>,
{
    type Response = Response<BoxBody>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx) // Assuming inner service manages backpressure
    }

    #[allow(clippy::similar_names)]
    fn call(&mut self, req: Request<B>) -> Self::Future {
        let mut inner = self.inner.clone();
        let state = self.state.clone();
        let broadcaster = self.broadcaster.clone();
        let tunnel_id = self.tunnel_id;

        Box::pin(async move {
            let start_time = Utc::now();
            let req_id = Uuid::new_v4();

            // 1. Buffer Request
            let (parts, body) = req.into_parts();

            // Capture Headers
            let mut req_headers = HashMap::new();
            for (k, v) in &parts.headers {
                if let Ok(val) = v.to_str() {
                    req_headers.insert(k.to_string(), val.to_string());
                }
            }

            let req_method = parts.method.to_string();
            let req_path = parts.uri.path().to_string();

            let req_bytes = match body.collect().await {
                Ok(c) => c.to_bytes(),
                Err(_) => {
                    return Ok(error_response(
                        StatusCode::BAD_REQUEST,
                        "Failed to read request body",
                    ))
                }
            };

            let req_body_str = if req_bytes.len() < 1024 * 1024 {
                // Cap capture at 1MB
                String::from_utf8(req_bytes.to_vec()).ok()
            } else {
                Some("<Body too large>".to_string())
            };

            // Reconstruct Request with Box<dyn Error>
            let inner_req = Request::from_parts(
                parts,
                Full::new(req_bytes)
                    .map_err(|_| ProxyError::Custom("Request body error".into()))
                    .boxed(),
            );

            // 2. Call Inner Service
            let res = inner.call(inner_req).await;

            // 3. Process Response
            match res {
                Ok(response) => {
                    let (parts, body) = response.into_parts();

                    // Capture Headers
                    let mut res_headers = HashMap::new();
                    for (k, v) in &parts.headers {
                        if let Ok(val) = v.to_str() {
                            res_headers.insert(k.to_string(), val.to_string());
                        }
                    }
                    let status = parts.status.as_u16();

                    let res_bytes = match body.collect().await {
                        Ok(c) => c.to_bytes(),
                        Err(e) => {
                            return Ok(error_response(
                                StatusCode::BAD_GATEWAY,
                                &format!("Failed to read upstream response: {e}"),
                            ))
                        }
                    };

                    let res_body_str = if res_bytes.len() < 1024 * 1024 {
                        String::from_utf8(res_bytes.to_vec()).ok()
                    } else {
                        Some("<Body too large>".to_string())
                    };

                    let duration = Utc::now()
                        .signed_duration_since(start_time)
                        .num_milliseconds();

                    #[allow(clippy::cast_sign_loss)]
                    let duration = if duration < 0 { 0 } else { duration as u64 };

                    // Record to Dashboard State
                    let details = RequestDetails {
                        id: req_id,
                        tunnel_id,
                        method: req_method,
                        path: req_path,
                        request_headers: req_headers,
                        request_body: req_body_str,
                        status,
                        response_headers: res_headers,
                        response_body: res_body_str,
                        duration_ms: duration,
                        timestamp: start_time,
                    };

                    {
                        let mut guard = state.write().await;
                        guard.add_request(details.clone());
                    }

                    // Broadcast Event
                    let log_entry =
                        ferrotunnel_observability::dashboard::RequestLogEntry::from(&details);
                    broadcaster.send(DashboardEvent::NewRequest(log_entry));

                    // Reconstruct Response
                    let inner_res = Response::from_parts(
                        parts,
                        Full::new(res_bytes)
                            .map_err(|_| ProxyError::Custom("Response body error".into()))
                            .boxed(),
                    );

                    Ok(inner_res)
                }
                Err(e) => Err(e),
            }
        })
    }
}

#[allow(clippy::expect_used)]
fn error_response(status: StatusCode, msg: &str) -> Response<BoxBody> {
    Response::builder()
        .status(status)
        .body(
            Full::new(Bytes::from(msg.to_string()))
                .map_err(|_| ProxyError::Custom("Error construction failed".into()))
                .boxed(),
        )
        .expect("building error response should never fail")
}
