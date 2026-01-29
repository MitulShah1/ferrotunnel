mod middleware;

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use ferrotunnel_core::TunnelClient;
use ferrotunnel_observability::dashboard::models::{DashboardTunnelInfo, TunnelStatus};
use ferrotunnel_observability::{init_basic_observability, shutdown_tracing};
use std::time::Duration;
use tracing::{error, info};

// Import middleware types
use ferrotunnel_http::proxy::ProxyError;
use middleware::DashboardCaptureLayer;

// Need to match the BoxBody type used in ferrotunnel-http
type BoxBody = http_body_util::combinators::BoxBody<bytes::Bytes, ProxyError>;

trait StreamHandler: Send + Sync {
    fn handle(&self, stream: ferrotunnel_core::stream::multiplexer::VirtualStream);
}

// Make sure LocalProxyService is accessible. If not re-exported, we might need to rely on generic bounds or fix lib.rs
// Assuming ferrotunnel_http::proxy::LocalProxyService is pub.
use ferrotunnel_http::proxy::LocalProxyService;

impl<L> StreamHandler for ferrotunnel_http::HttpProxy<L>
where
    L: tower::Layer<LocalProxyService> + Clone + Send + Sync + 'static,
    L::Service: tower::Service<
            hyper::Request<hyper::body::Incoming>,
            Response = hyper::Response<BoxBody>,
            Error = hyper::Error,
        > + Send
        + Clone
        + 'static,
    <L::Service as tower::Service<hyper::Request<hyper::body::Incoming>>>::Future: Send,
{
    fn handle(&self, stream: ferrotunnel_core::stream::multiplexer::VirtualStream) {
        self.handle_stream(stream);
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Server address (host:port)
    #[arg(long, env = "FERROTUNNEL_SERVER")]
    server: String,

    /// Authentication token
    #[arg(long, env = "FERROTUNNEL_TOKEN")]
    token: String,

    /// Log level
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    log_level: String,

    /// Local service address to forward to (host:port)
    #[arg(long, default_value = "127.0.0.1:8000", env = "FERROTUNNEL_LOCAL_ADDR")]
    local_addr: String,

    /// Dashboard port
    #[arg(long, default_value = "4040", env = "FERROTUNNEL_DASHBOARD_PORT")]
    dashboard_port: u16,

    /// Disable dashboard
    #[arg(long)]
    no_dashboard: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup observability
    init_basic_observability("ferrotunnel-client");

    info!("Starting FerroTunnel Client v{}", env!("CARGO_PKG_VERSION"));

    let proxy: std::sync::Arc<dyn StreamHandler>;

    // Start Dashboard and configure proxy
    if args.no_dashboard {
        // Initialize basic Proxy (Identity layer)
        proxy = std::sync::Arc::new(ferrotunnel_http::HttpProxy::new(args.local_addr.clone()));
    } else {
        use ferrotunnel_observability::dashboard::{
            create_router, DashboardState, EventBroadcaster,
        };
        use std::sync::Arc;
        use tokio::sync::RwLock; // Use tokio RwLock

        let dashboard_state = Arc::new(RwLock::new(DashboardState::new(1000)));
        let broadcaster = Arc::new(EventBroadcaster::new(100));

        // No loop_broadcaster.run_loop() needed for broadcast channel

        let app = create_router(dashboard_state.clone(), broadcaster.clone());
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], args.dashboard_port));

        info!("Starting Dashboard at http://{}", addr);
        tokio::spawn(async move {
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    if let Err(e) = axum::serve(listener, app).await {
                        error!("Dashboard server error: {e}");
                    }
                }
                Err(e) => {
                    error!("Failed to bind dashboard server to {addr}: {e}");
                }
            }
        });

        // Register the local tunnel in the dashboard
        let tunnel_id = uuid::Uuid::new_v4();
        {
            let mut state = dashboard_state.write().await;
            let tunnel_info = DashboardTunnelInfo {
                id: tunnel_id,
                subdomain: None,
                public_url: None,
                local_addr: args.local_addr.clone(),
                created_at: Utc::now(),
                status: TunnelStatus::Connected,
            };
            state.add_tunnel(tunnel_info);
            info!("Registered tunnel {} in dashboard", tunnel_id);
        }

        // Initialize Proxy with Middleware
        info!("Traffic inspection enabled");
        let capture_layer = DashboardCaptureLayer {
            state: dashboard_state.clone(),
            broadcaster,
            tunnel_id,
        };

        proxy = Arc::new(
            ferrotunnel_http::HttpProxy::new(args.local_addr.clone()).with_layer(capture_layer),
        );
    }

    // Simple reconnection loop
    loop {
        let mut client = TunnelClient::new(args.server.clone(), args.token.clone());
        let proxy_ref = proxy.clone();

        match client
            .connect_and_run(move |stream| {
                let proxy = proxy_ref.clone();
                async move {
                    proxy.handle(stream);
                }
            })
            .await
        {
            Ok(()) => {
                info!("Client finished normally, exiting.");
                break;
            }
            Err(e) => {
                error!("Connection lost or failed: {}", e);
                info!("Reconnecting in 5 seconds...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    shutdown_tracing();
    Ok(())
}
