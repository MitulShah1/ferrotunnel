// Use mimalloc as the global allocator for better performance
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod middleware;

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use ferrotunnel_core::TunnelClient;
use ferrotunnel_observability::dashboard::models::{DashboardTunnelInfo, TunnelStatus};
use ferrotunnel_observability::{init_basic_observability, init_minimal_logging, shutdown_tracing};
use ferrotunnel_protocol::frame::Protocol;
use std::time::Duration;
use tokio::net::TcpStream;
use tracing::{debug, error, info};

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
#[allow(clippy::struct_excessive_bools)]
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

    /// Enable TLS for server connection
    #[arg(long, env = "FERROTUNNEL_TLS")]
    tls: bool,

    /// Skip TLS certificate verification (insecure, for self-signed certs)
    #[arg(long, env = "FERROTUNNEL_TLS_SKIP_VERIFY")]
    tls_skip_verify: bool,

    /// Path to CA certificate for TLS verification
    #[arg(long, env = "FERROTUNNEL_TLS_CA")]
    tls_ca: Option<std::path::PathBuf>,

    /// server name (SNI) for TLS verification
    #[arg(long, env = "FERROTUNNEL_TLS_SERVER_NAME")]
    tls_server_name: Option<String>,

    /// Path to client certificate file (PEM format) for mutual TLS
    #[arg(long, env = "FERROTUNNEL_TLS_CERT")]
    tls_cert: Option<std::path::PathBuf>,

    /// Path to client private key file (PEM format) for mutual TLS
    #[arg(long, env = "FERROTUNNEL_TLS_KEY")]
    tls_key: Option<std::path::PathBuf>,

    /// Enable observability (metrics and tracing) - disabled by default for lower latency
    #[arg(long, env = "FERROTUNNEL_OBSERVABILITY")]
    observability: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup observability only if enabled (disabled by default for lower latency)
    if args.observability {
        init_basic_observability("ferrotunnel-client");
    } else {
        // Minimal logging setup without full observability infrastructure
        init_minimal_logging();
    }

    info!("Starting FerroTunnel Client v{}", env!("CARGO_PKG_VERSION"));

    // Start Dashboard and configure proxy
    let proxy: std::sync::Arc<dyn StreamHandler> = if args.no_dashboard {
        // Initialize basic Proxy (Identity layer)
        std::sync::Arc::new(ferrotunnel_http::HttpProxy::new(args.local_addr.clone()))
    } else {
        setup_dashboard(&args).await
    };

    // Simple reconnection loop
    loop {
        let mut client = TunnelClient::new(args.server.clone(), args.token.clone());
        client = setup_tls(client, &args);

        let proxy_ref = proxy.clone();

        let local_addr_config = args.local_addr.clone();
        match client
            .connect_and_run(move |stream| {
                let proxy = proxy_ref.clone();
                let local_addr = local_addr_config.clone();
                async move {
                    if stream.protocol() == Protocol::TCP {
                        // Handle raw TCP stream
                        let peer_id = stream.id();
                        debug!("Handling raw TCP stream {}", peer_id);
                        tokio::spawn(async move {
                            match TcpStream::connect(&local_addr).await {
                                Ok(mut local_stream) => {
                                    let mut tunnel_stream = stream;
                                    if let Err(e) = tokio::io::copy_bidirectional(
                                        &mut tunnel_stream,
                                        &mut local_stream,
                                    )
                                    .await
                                    {
                                        debug!("TCP tunnel copy error: {}", e);
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to connect to local TCP service {}: {}",
                                        local_addr, e
                                    );
                                }
                            }
                        });
                    } else {
                        // Handle HTTP/WebSocket stream via proxy
                        proxy.handle(stream);
                    }
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

async fn setup_dashboard(args: &Args) -> std::sync::Arc<dyn StreamHandler> {
    use ferrotunnel_observability::dashboard::{create_router, DashboardState, EventBroadcaster};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let dashboard_state = Arc::new(RwLock::new(DashboardState::new(1000)));
    let broadcaster = Arc::new(EventBroadcaster::new(100));

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

    Arc::new(ferrotunnel_http::HttpProxy::new(args.local_addr.clone()).with_layer(capture_layer))
}

fn setup_tls(mut client: TunnelClient, args: &Args) -> TunnelClient {
    if args.tls {
        if args.tls_skip_verify {
            info!("TLS enabled with certificate verification skipped (insecure)");
            client = client.with_tls_skip_verify();
        } else if let Some(ref ca_path) = args.tls_ca {
            info!("TLS enabled with CA: {:?}", ca_path);
            client = client.with_tls_ca(ca_path.clone());
        } else {
            info!("TLS enabled with certificate verification skipped (no CA provided)");
            client = client.with_tls_skip_verify();
        }

        if let Some(ref server_name) = args.tls_server_name {
            info!("TLS SNI enabled with server name: {}", server_name);
            client = client.with_server_name(server_name.clone());
        }

        if let (Some(ref cert_path), Some(ref key_path)) = (&args.tls_cert, &args.tls_key) {
            info!(
                "Mutual TLS enabled with cert: {:?}, key: {:?}",
                cert_path, key_path
            );
            client = client.with_tls(cert_path.clone(), key_path.clone());
        }
    }
    client
}
