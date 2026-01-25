use anyhow::Result;
use clap::Parser;
use ferrotunnel_core::TunnelServer;
use std::net::SocketAddr;
use tracing::info;

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(format!(
            "ferrotunnel_server={},ferrotunnel_core={},ferrotunnel_http={}",
            args.log_level, args.log_level, args.log_level
        ))
        .init();

    info!("Starting FerroTunnel Server v{}", env!("CARGO_PKG_VERSION"));

    let server = TunnelServer::new(args.bind, args.token.clone());
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
