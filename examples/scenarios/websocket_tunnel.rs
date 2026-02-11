//! Example: WebSocket Tunneling
//!
//! This example demonstrates how to use FerroTunnel to expose a local WebSocket server.
//! It starts a local WebSocket echo server, exposes it via a FerroTunnel server and client,
//! and then connects to it through the tunnel to verify the connection.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example websocket_tunnel
//! ```

use ferrotunnel::{Client, Server};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, info_span, Instrument};

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,ferrotunnel=debug")
        .compact()
        .init();

    let server_port = 7835;
    let http_port = 8080;
    let local_port = 3000;

    let server_addr = format!("127.0.0.1:{server_port}").parse().unwrap();
    let http_addr = format!("127.0.0.1:{http_port}").parse().unwrap();
    let local_addr = format!("127.0.0.1:{local_port}").parse().unwrap();
    let token = "websocket-secret-token";

    println!();
    println!("  ╔═══════════════════════════════════════╗");
    println!("  ║      FerroTunnel WebSocket Example    ║");
    println!("  ╚═══════════════════════════════════════╝");
    println!();

    // 1. Start a local WebSocket echo server
    info!("Starting local WebSocket echo server on {}", local_addr);
    let ws_handle = start_ws_echo_server(local_addr).await;

    // 2. Start the FerroTunnel server
    info!(
        "Starting FerroTunnel server (Control: {}, HTTP Ingress: {})",
        server_addr, http_addr
    );
    let mut server = Server::builder()
        .bind(server_addr)
        .http_bind(http_addr)
        .token(token)
        .build()?;

    let _server_task = tokio::spawn(async move {
        let _ = server.start().await;
    });

    // Wait for server to bind
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 3. Start the FerroTunnel client
    info!("Starting FerroTunnel client (Target: {})", local_addr);
    let mut client = Client::builder()
        .server_addr(server_addr.to_string())
        .token(token)
        .local_addr(local_addr.to_string())
        .tunnel_id("ws-demo")
        .build()?;

    let info = client.start().await?;
    info!("Client connected! Session ID: {:?}", info.session_id);

    // 4. Connect to the WebSocket echo server THROUGH THE TUNNEL
    let ws_url = format!("ws://127.0.0.1:{http_port}/ws");
    info!("Connecting to WebSocket via tunnel: {}", ws_url);

    // We use the tunnel-id in the Host header for routing
    let mut request =
        tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(ws_url)
            .unwrap();
    request
        .headers_mut()
        .insert("Host", "ws-demo".parse().unwrap());

    // Connect via the HTTP ingress port
    let (ws_stream, response) = tokio_tungstenite::connect_async(request)
        .await
        .expect("Failed to connect");
    info!(
        "Connected to WebSocket! Response status: {}",
        response.status()
    );

    let (mut write, mut read) = ws_stream.split();

    // 5. Send and receive messages
    let messages = vec![
        Message::Text("Hello FerroTunnel!".into()),
        Message::Binary(vec![1, 2, 3, 4, 5].into()),
        Message::Text("WebSocket tunneling works! 🚀".into()),
    ];

    for msg in messages {
        info!("Sending: {:?}", msg);
        write
            .send(msg.clone())
            .await
            .expect("Failed to send message");

        if let Some(Ok(echo)) = read.next().await {
            info!("Received: {:?}", echo);
            assert_eq!(msg, echo);
        }
    }

    info!("WebSocket demo completed successfully!");

    // Cleanup
    ws_handle.abort();
    let _ = client.shutdown().await;

    Ok(())
}

async fn start_ws_echo_server(addr: std::net::SocketAddr) -> tokio::task::JoinHandle<()> {
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind WS echo server");

    tokio::spawn(async move {
        while let Ok((stream, peer)) = listener.accept().await {
            let span = info_span!("ws_echo_server", peer = %peer);
            tokio::spawn(
                async move {
                    let ws = tokio_tungstenite::accept_async(stream)
                        .await
                        .expect("WS handshake failed");
                    info!("WebSocket client connected");
                    let (mut write, mut read) = ws.split();
                    while let Some(Ok(msg)) = read.next().await {
                        if msg.is_close() {
                            break;
                        }
                        if msg.is_text() || msg.is_binary() {
                            let _ = write.send(msg).await;
                        }
                    }
                    info!("WebSocket client disconnected");
                }
                .instrument(span),
            );
        }
    })
}
