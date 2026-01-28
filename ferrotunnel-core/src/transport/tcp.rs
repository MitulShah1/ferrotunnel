//! Plain TCP transport

use super::socket_tuning::configure_socket_silent;
use super::BoxedStream;
use ferrotunnel_common::Result;
use std::io;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

pub struct TcpTransport;

impl TcpTransport {
    pub async fn bind(addr: SocketAddr) -> Result<TcpListener> {
        let listener = TcpListener::bind(addr).await?;
        Ok(listener)
    }

    pub async fn connect(addr: SocketAddr) -> Result<TcpStream> {
        let stream = TcpStream::connect(addr).await?;
        configure_socket_silent(&stream);
        Ok(stream)
    }
}

pub async fn connect(addr: &str) -> io::Result<BoxedStream> {
    let stream = TcpStream::connect(addr).await?;
    configure_socket_silent(&stream);
    Ok(Box::pin(stream))
}
