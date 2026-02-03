//! Socket tuning for optimal TCP performance
//!
//! Applies low-latency and high-throughput optimizations:
//! - `TCP_NODELAY`: Disable Nagle's algorithm for lower latency
//! - Increased buffer sizes: Better throughput for sustained traffic
//! - TCP keepalive: Detect dead connections faster

use socket2::SockRef;
use std::io;
use std::time::Duration;
use tokio::net::TcpStream;

const RECV_BUFFER_SIZE: usize = 1024 * 1024;
const SEND_BUFFER_SIZE: usize = 1024 * 1024;
const KEEPALIVE_TIME: Duration = Duration::from_secs(30);
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);

pub fn configure_socket(stream: &TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?;

    let socket = SockRef::from(stream);

    socket.set_recv_buffer_size(RECV_BUFFER_SIZE)?;
    socket.set_send_buffer_size(SEND_BUFFER_SIZE)?;

    let keepalive = socket2::TcpKeepalive::new()
        .with_time(KEEPALIVE_TIME)
        .with_interval(KEEPALIVE_INTERVAL);
    socket.set_tcp_keepalive(&keepalive)?;

    Ok(())
}

pub fn configure_socket_silent(stream: &TcpStream) {
    let _ = configure_socket(stream);
}
