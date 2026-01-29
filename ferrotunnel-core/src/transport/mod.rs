//! Transport layer abstraction for TCP and TLS

pub mod batched_sender;
pub mod socket_tuning;
pub mod tcp;
pub mod tls;

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;

pub trait AsyncStream: AsyncRead + AsyncWrite + Send + Unpin {}

impl<T: AsyncRead + AsyncWrite + Send + Unpin> AsyncStream for T {}

pub type BoxedStream = Pin<Box<dyn AsyncStream>>;

#[derive(Debug, Clone, Default)]
pub enum TransportConfig {
    #[default]
    Tcp,
    Tls(tls::TlsTransportConfig),
}

pub async fn connect(config: &TransportConfig, addr: &str) -> io::Result<BoxedStream> {
    match config {
        TransportConfig::Tcp => tcp::connect(addr).await,
        TransportConfig::Tls(tls_config) => tls::connect(addr, tls_config).await,
    }
}

pub async fn accept(
    config: &TransportConfig,
    listener: &TcpListener,
) -> io::Result<(BoxedStream, SocketAddr)> {
    let (tcp_stream, addr) = listener.accept().await?;
    socket_tuning::configure_socket_silent(&tcp_stream);

    match config {
        TransportConfig::Tcp => Ok((Box::pin(tcp_stream), addr)),
        TransportConfig::Tls(tls_config) => {
            let tls_stream = tls::accept_tls(tcp_stream, tls_config).await?;
            Ok((Box::pin(tls_stream), addr))
        }
    }
}
