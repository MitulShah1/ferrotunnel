//! End-to-end tunnel tests
//!
//! Tests basic tunnel functionality: server, client, HTTP routing

use super::{start_echo_server, wait_for_server, TestConfig};
use ferrotunnel::{Client, Server};
use std::time::Duration;

/// Test that server starts and accepts connections
#[tokio::test]
async fn test_server_starts() {
    let config = TestConfig::default();

    let mut server = Server::builder()
        .bind(config.server_addr)
        .http_bind(config.http_addr)
        .token(config.token)
        .build()
        .expect("Failed to build server");

    // Start server in background
    let server_handle = tokio::spawn(async move { server.start().await });

    // Wait for server to be ready
    assert!(
        wait_for_server(config.server_addr, Duration::from_secs(5)).await,
        "Server did not start in time"
    );

    // Clean up
    server_handle.abort();
}

/// Test that client connects to server
#[tokio::test]
async fn test_client_connects() {
    let config = TestConfig::default();

    // Start local service
    let _echo_handle = start_echo_server(config.local_service_addr).await;

    // Start server
    let mut server = Server::builder()
        .bind(config.server_addr)
        .http_bind(config.http_addr)
        .token(config.token)
        .build()
        .expect("Failed to build server");

    let _server_handle = tokio::spawn(async move {
        let _ = server.start().await;
    });

    // Wait for server
    assert!(
        wait_for_server(config.server_addr, Duration::from_secs(5)).await,
        "Server did not start"
    );

    // Build and start client
    let mut client = Client::builder()
        .server_addr(config.server_addr.to_string())
        .token(config.token)
        .local_addr(config.local_service_addr.to_string())
        .build()
        .expect("Failed to build client");

    let info = client.start().await;
    assert!(info.is_ok(), "Client failed to connect: {:?}", info.err());

    // Verify client got a session
    let tunnel_info = info.unwrap();
    assert!(
        tunnel_info.session_id.is_some(),
        "Session ID should not be empty"
    );

    // Shutdown
    let _ = client.shutdown().await;
}

/// Test full HTTP request through tunnel
#[tokio::test]
async fn test_http_through_tunnel() {
    let config = TestConfig::default();

    // Start local echo service
    let _echo_handle = start_echo_server(config.local_service_addr).await;

    // Start server
    let mut server = Server::builder()
        .bind(config.server_addr)
        .http_bind(config.http_addr)
        .token(config.token)
        .build()
        .expect("Failed to build server");

    let _server_handle = tokio::spawn(async move {
        let _ = server.start().await;
    });

    // Wait for server
    assert!(wait_for_server(config.server_addr, Duration::from_secs(5)).await);

    // Start client
    let mut client = Client::builder()
        .server_addr(config.server_addr.to_string())
        .token(config.token)
        .local_addr(config.local_service_addr.to_string())
        .build()
        .expect("Failed to build client");

    let _ = client.start().await.expect("Client failed to connect");

    // Give time for tunnel to be registered
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Make HTTP request through the tunnel
    // Note: In a real test, we'd send a request to http_addr
    // For now, verify the tunnel is established

    let _ = client.shutdown().await;
}
