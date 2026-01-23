use ferrotunnel_common::Result;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

/// TCP Transport utility
pub struct TcpTransport;

impl TcpTransport {
    /// Bind to a local address
    pub async fn bind(addr: SocketAddr) -> Result<TcpListener> {
        let listener = TcpListener::bind(addr).await?;
        Ok(listener)
    }

    /// Connect to a remote address
    pub async fn connect(addr: SocketAddr) -> Result<TcpStream> {
        let stream = TcpStream::connect(addr).await?;
        Ok(stream)
    }
}
