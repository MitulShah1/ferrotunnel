#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Integration tests for `FerroTunnel`
//!
//! These tests verify end-to-end functionality of the tunnel system.

mod multi_client_test;
mod plugin_test;
mod tunnel_test;

use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::sleep;

/// Test configuration with high ports to avoid conflicts
pub struct TestConfig {
    pub server_addr: SocketAddr,
    pub http_addr: SocketAddr,
    pub local_service_addr: SocketAddr,
    pub token: &'static str,
}

static NEXT_PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(30000);

pub fn get_free_port() -> u16 {
    use std::sync::atomic::Ordering;
    loop {
        let port = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
        if std::net::TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        let server_port = get_free_port();
        let http_port = get_free_port();
        let local_port = get_free_port();

        Self {
            server_addr: format!("127.0.0.1:{server_port}").parse().unwrap(),
            http_addr: format!("127.0.0.1:{http_port}").parse().unwrap(),
            local_service_addr: format!("127.0.0.1:{local_port}").parse().unwrap(),
            token: "test-secret-token",
        }
    }
}

/// Wait for a server to start listening
pub async fn wait_for_server(addr: SocketAddr, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return true;
        }
        sleep(Duration::from_millis(50)).await;
    }
    false
}

/// Start a simple HTTP server that echoes requests
pub async fn start_echo_server(addr: SocketAddr) -> tokio::task::JoinHandle<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind echo server");

    tokio::spawn(async move {
        loop {
            if let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    if n > 0 {
                        let response = "HTTP/1.1 200 OK\r\n\
                             Content-Type: text/plain\r\n\
                             Content-Length: 13\r\n\
                             \r\n\
                             Hello, World!"
                            .to_string();
                        let _ = socket.write_all(response.as_bytes()).await;
                    }
                });
            }
        }
    })
}
