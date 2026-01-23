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
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(format!(
            "ferrotunnel_client={},ferrotunnel_core={}",
            args.log_level, args.log_level
        ))
        .init();

    info!("Starting FerroTunnel Client v{}", env!("CARGO_PKG_VERSION"));

    // Simple reconnection loop
    loop {
        let mut client = TunnelClient::new(args.server.clone(), args.token.clone());

        match client.connect_and_run().await {
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
