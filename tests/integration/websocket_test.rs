use super::{get_free_port, wait_for_server};
use ferrotunnel::{Client, Server};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

async fn start_ws_echo_server(addr: std::net::SocketAddr) -> tokio::task::JoinHandle<()> {
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind WS echo server");

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let ws = tokio_tungstenite::accept_async(stream)
                        .await
                        .expect("WS handshake failed");
                    let (mut write, mut read) = ws.split();
                    while let Some(Ok(msg)) = read.next().await {
                        if msg.is_close() {
                            break;
                        }
                        if msg.is_text() || msg.is_binary() {
                            let _ = write.send(msg).await;
                        }
                    }
                });
            }
        }
    })
}

#[tokio::test]
async fn test_websocket_upgrade_through_tunnel() {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let server_port = get_free_port();
    let http_port = get_free_port();
    let local_port = get_free_port();

    let server_addr: std::net::SocketAddr = format!("127.0.0.1:{server_port}").parse().unwrap();
    let http_addr: std::net::SocketAddr = format!("127.0.0.1:{http_port}").parse().unwrap();
    let local_addr: std::net::SocketAddr = format!("127.0.0.1:{local_port}").parse().unwrap();

    let _ws_handle = start_ws_echo_server(local_addr).await;

    let mut server = Server::builder()
        .bind(server_addr)
        .http_bind(http_addr)
        .token("test-secret-token")
        .build()
        .expect("Failed to build server");

    let _server_handle = tokio::spawn(async move {
        let _ = server.start().await;
    });

    assert!(
        wait_for_server(server_addr, Duration::from_secs(5)).await,
        "Server did not start"
    );

    let mut client = Client::builder()
        .server_addr(server_addr.to_string())
        .token("test-secret-token")
        .local_addr(local_addr.to_string())
        .build()
        .expect("Failed to build client");

    let info = client.start().await.expect("Client failed to connect");
    let session_id = info
        .session_id
        .expect("Session ID should be present")
        .to_string();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let tcp_stream = tokio::net::TcpStream::connect(http_addr)
        .await
        .expect("Failed to connect to HTTP ingress");

    let ws_url = format!("ws://127.0.0.1:{http_port}/ws");
    let mut request = ws_url.into_client_request().unwrap();
    request
        .headers_mut()
        .insert("Host", session_id.parse().unwrap());

    let (ws_stream, response) = tokio_tungstenite::client_async(request, tcp_stream)
        .await
        .expect("WebSocket connection failed");

    assert_eq!(
        response.status(),
        http::StatusCode::SWITCHING_PROTOCOLS,
        "Expected 101 Switching Protocols"
    );

    let (mut write, mut read) = ws_stream.split();

    write
        .send(Message::Text("hello tunnel".into()))
        .await
        .expect("Failed to send WS message");

    let echo = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout waiting for echo")
        .expect("Stream ended")
        .expect("Failed to read message");

    assert_eq!(echo, Message::Text("hello tunnel".into()));

    write
        .send(Message::Text("second message".into()))
        .await
        .expect("Failed to send second WS message");

    let echo2 = tokio::time::timeout(Duration::from_secs(5), read.next())
        .await
        .expect("Timeout waiting for second echo")
        .expect("Stream ended")
        .expect("Failed to read second message");

    assert_eq!(echo2, Message::Text("second message".into()));

    drop(write);
    drop(read);

    let _ = client.shutdown().await;
}

#[tokio::test]
async fn test_websocket_raw_upgrade_101() {
    let server_port = get_free_port();
    let http_port = get_free_port();
    let local_port = get_free_port();

    let server_addr: std::net::SocketAddr = format!("127.0.0.1:{server_port}").parse().unwrap();
    let http_addr: std::net::SocketAddr = format!("127.0.0.1:{http_port}").parse().unwrap();
    let local_addr: std::net::SocketAddr = format!("127.0.0.1:{local_port}").parse().unwrap();

    let _ws_handle = start_ws_echo_server(local_addr).await;

    let mut server = Server::builder()
        .bind(server_addr)
        .http_bind(http_addr)
        .token("test-secret-token")
        .build()
        .expect("Failed to build server");

    let _server_handle = tokio::spawn(async move {
        let _ = server.start().await;
    });

    assert!(
        wait_for_server(server_addr, Duration::from_secs(5)).await,
        "Server did not start"
    );

    let mut client = Client::builder()
        .server_addr(server_addr.to_string())
        .token("test-secret-token")
        .local_addr(local_addr.to_string())
        .build()
        .expect("Failed to build client");

    let info = client.start().await.expect("Client failed to connect");
    let session_id = info
        .session_id
        .expect("Session ID should be present")
        .to_string();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut tcp = tokio::net::TcpStream::connect(http_addr)
        .await
        .expect("Failed to connect to HTTP ingress");

    let ws_key = "dGhlIHNhbXBsZSBub25jZQ==";
    let raw_request = format!(
        "GET /ws HTTP/1.1\r\n\
         Host: {session_id}\r\n\
         Connection: Upgrade\r\n\
         Upgrade: websocket\r\n\
         Sec-WebSocket-Version: 13\r\n\
         Sec-WebSocket-Key: {ws_key}\r\n\
         \r\n"
    );
    tcp.write_all(raw_request.as_bytes())
        .await
        .expect("Failed to send upgrade request");

    let mut buf = vec![0u8; 4096];
    let n = tokio::time::timeout(Duration::from_secs(5), tcp.read(&mut buf))
        .await
        .expect("Timeout reading response")
        .expect("Failed to read response");

    let response_str = String::from_utf8_lossy(&buf[..n]);
    assert!(
        response_str.contains("101 Switching Protocols"),
        "Expected 101 response, got: {response_str}"
    );
    assert!(
        response_str.to_lowercase().contains("upgrade: websocket"),
        "Expected Upgrade: websocket header"
    );

    let _ = client.shutdown().await;
}
