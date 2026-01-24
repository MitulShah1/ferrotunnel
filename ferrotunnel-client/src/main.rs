use anyhow::Result;
use clap::Parser;
use ferrotunnel_core::TunnelClient;
use std::time::Duration;
use tracing::{error, info};

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
    /// Log level
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    log_level: String,

    /// Local service address to forward to (host:port)
    #[arg(long, default_value = "127.0.0.1:8000")]
    local_addr: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(format!(
            "ferrotunnel_client={},ferrotunnel_core={},ferrotunnel_http={}",
            args.log_level, args.log_level, args.log_level
        ))
        .init();

    info!("Starting FerroTunnel Client v{}", env!("CARGO_PKG_VERSION"));

    let proxy = std::sync::Arc::new(ferrotunnel_http::HttpProxy::new(args.local_addr.clone()));

    // Simple reconnection loop
    loop {
        let mut client = TunnelClient::new(args.server.clone(), args.token.clone());
        let proxy_ref = proxy.clone();

        match client
            .connect_and_run(move |stream| {
                let proxy = proxy_ref.clone();
                async move {
                    proxy.handle_stream(stream);
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

    Ok(())
}
