use ferrotunnel_core::{TunnelClient, TunnelServer};
use ferrotunnel_http::TcpIngress;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn test_tcp_tunnel_echo() {
    // 1. Start local echo server
    let echo_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_addr = echo_listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (mut socket, _) = echo_listener.accept().await.unwrap();
        let mut buf = vec![0u8; 1024];
        let n = socket.read(&mut buf).await.unwrap();
        socket.write_all(&buf[..n]).await.unwrap();
    });

    // 2. Start tunnel server
    let server_addr = "127.0.0.1:17835".parse().unwrap();
    let token = "test-token".to_string();
    let server = TunnelServer::new(server_addr, token.clone());
    let sessions = server.sessions();

    tokio::spawn(async move {
        server.run().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // 3. Start TCP ingress
    let tcp_addr = "127.0.0.1:15000".parse().unwrap();
    let tcp_ingress = TcpIngress::new(tcp_addr, sessions);

    tokio::spawn(async move {
        tcp_ingress.start().await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // 4. Start tunnel client
    let mut client = TunnelClient::new(format!("127.0.0.1:{}", 17835), token);
    let local_addr = echo_addr.to_string();

    tokio::spawn(async move {
        client
            .connect_and_run(move |mut stream| {
                let local_addr = local_addr.clone();
                async move {
                    let mut local = TcpStream::connect(&local_addr).await.unwrap();
                    tokio::io::copy_bidirectional(&mut stream, &mut local)
                        .await
                        .unwrap();
                }
            })
            .await
            .unwrap();
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    // 5. Test the tunnel
    let mut client_stream = TcpStream::connect(tcp_addr).await.unwrap();
    client_stream.write_all(b"HELLO").await.unwrap();

    let mut buf = vec![0u8; 5];
    client_stream.read_exact(&mut buf).await.unwrap();

    assert_eq!(&buf, b"HELLO");
}
