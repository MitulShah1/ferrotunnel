use anyhow::Result;
use clap::Parser;
use ferrotunnel_core::TunnelServer;
use ferrotunnel_observability::{gather_metrics, init_basic_observability};
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Address to bind to
    #[arg(long, default_value = "0.0.0.0:7835", env = "FERROTUNNEL_BIND")]
    bind: SocketAddr,

    /// Authentication token
    #[arg(long, env = "FERROTUNNEL_TOKEN")]
    token: String,

    /// Log level
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    log_level: String,

    /// HTTP Ingress bind address
    #[arg(long, default_value = "0.0.0.0:8080", env = "FERROTUNNEL_HTTP_BIND")]
    http_bind: SocketAddr,

    /// Metrics bind address
    #[arg(long, default_value = "0.0.0.0:9090", env = "FERROTUNNEL_METRICS_BIND")]
    metrics_bind: SocketAddr,

    /// Path to TLS certificate file (PEM format)
    #[arg(long, env = "FERROTUNNEL_TLS_CERT")]
    tls_cert: Option<PathBuf>,

    /// Path to TLS private key file (PEM format)
    #[arg(long, env = "FERROTUNNEL_TLS_KEY")]
    tls_key: Option<PathBuf>,

    /// Path to CA certificate for client authentication (PEM format)
    #[arg(long, env = "FERROTUNNEL_TLS_CA")]
    tls_ca: Option<PathBuf>,

    /// Require client certificate authentication
    #[arg(long, env = "FERROTUNNEL_TLS_CLIENT_AUTH")]
    tls_client_auth: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup observability
    init_basic_observability("ferrotunnel-server");

    // Start metrics endpoint in background
    let metrics_addr = args.metrics_bind;
    tokio::spawn(async move {
        use axum::{routing::get, Router};
        let app = Router::new().route("/metrics", get(|| async { gather_metrics() }));
        info!("Metrics server listening on http://{}", metrics_addr);
        match tokio::net::TcpListener::bind(metrics_addr).await {
            Ok(listener) => {
                if let Err(e) = axum::serve(listener, app).await {
                    error!("Metrics server error: {}", e);
                }
            }
            Err(e) => error!("Failed to bind metrics server to {}: {}", metrics_addr, e),
        }
    });

    info!("Starting FerroTunnel Server v{}", env!("CARGO_PKG_VERSION"));

    let mut server = TunnelServer::new(args.bind, args.token.clone());

    if let (Some(cert_path), Some(key_path)) = (&args.tls_cert, &args.tls_key) {
        info!(
            "TLS enabled with cert: {:?}, key: {:?}",
            cert_path, key_path
        );
        server = server.with_tls(cert_path.clone(), key_path.clone());

        if let Some(ca_path) = &args.tls_ca {
            info!("TLS Client Authentication enabled with CA: {:?}", ca_path);
            server = server.with_client_auth(ca_path.clone());
        } else if args.tls_client_auth {
            error!("--tls-client-auth requires --tls-ca to be provided");
            std::process::exit(1);
        }
    }
    let sessions = server.sessions();

    info!("Initializing Plugin System");
    let mut registry = ferrotunnel_plugin::PluginRegistry::new();

    // built-in plugins
    // 1. Logger
    registry.register(std::sync::Arc::new(tokio::sync::RwLock::new(
        ferrotunnel_plugin::builtin::LoggerPlugin::new().with_body_logging(),
    )));

    // 2. Token Auth
    registry.register(std::sync::Arc::new(tokio::sync::RwLock::new(
        ferrotunnel_plugin::builtin::TokenAuthPlugin::new(vec![args.token.clone()]),
    )));

    // Initialize plugins
    registry
        .init_all()
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let registry = std::sync::Arc::new(registry);

    info!("Starting HTTP Ingress on {}", args.http_bind);
    let ingress = ferrotunnel_http::HttpIngress::new(args.http_bind, sessions, registry);

    tokio::try_join!(server.run(), ingress.start())?;

    Ok(())
}
