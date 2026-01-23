use ferrotunnel_common::{Result, TunnelError};
use ferrotunnel_protocol::codec::TunnelCodec;
use ferrotunnel_protocol::frame::{Frame, HandshakeStatus};
use futures::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::interval;
use tokio_util::codec::Framed;
use tracing::{debug, error, info};
use uuid::Uuid;

pub struct TunnelClient {
    server_addr: String,
    auth_token: String,
    session_id: Option<Uuid>,
}

impl TunnelClient {
    pub fn new(server_addr: String, auth_token: String) -> Self {
        Self {
            server_addr,
            auth_token,
            session_id: None,
        }
    }

    /// Connect to the server and start the session
    pub async fn connect_and_run(&mut self) -> Result<()> {
        info!("Connecting to {}", self.server_addr);

        // Resolve address (simple implementation, relying on TcpStream::connect to resolve host:port)
        // If server_addr is "host:port" TcpStream::connect works.
        let stream = TcpStream::connect(&self.server_addr).await?;
        info!("Connected to {}", self.server_addr);

        let mut framed = Framed::new(stream, TunnelCodec::new());

        // 1. Send Handshake
        #[allow(clippy::cast_possible_truncation)]
        let _timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        debug!("Sending handshake...");
        framed
            .send(Frame::Handshake {
                version: 1,
                token: self.auth_token.clone(),
                capabilities: vec!["basic".to_string()],
            })
            .await?;

        // 2. Wait for Ack
        if let Some(result) = framed.next().await {
            match result? {
                Frame::HandshakeAck {
                    status,
                    session_id,
                    server_capabilities: _,
                } => match status {
                    HandshakeStatus::Success => {
                        self.session_id = Some(session_id);
                        info!("Handshake successful. Session ID: {}", session_id);
                    }
                    status => {
                        error!("Handshake failed: {:?}", status);
                        return Err(TunnelError::Authentication(format!(
                            "Handshake rejected: {status:?}"
                        )));
                    }
                },
                _ => return Err(TunnelError::Protocol("Expected HandshakeAck".into())),
            }
        } else {
            return Err(TunnelError::Connection("Connection closed".into()));
        }

        // 3. Heartbeat and Message Loop
        // We need to split the stream to handle sending (heartbeats) and receiving (messages) concurrently
        let (mut split_sink, mut split_stream) = framed.split();

        let mut heartbeat_interval = interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                // Heartbeat Loop
                _ = heartbeat_interval.tick() => {
                    #[allow(clippy::cast_possible_truncation)]
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    debug!("Sending heartbeat");
                    if let Err(e) = split_sink.send(Frame::Heartbeat { timestamp: ts }).await {
                        error!("Failed to send heartbeat: {}", e);
                        return Err(e.into());
                    }
                }

                // Incoming Message Loop
                result = split_stream.next() => {
                    match result {
                        Some(Ok(frame)) => {
                            match frame {
                                Frame::HeartbeatAck { timestamp: _ } => {
                                    debug!("Heartbeat ack received");
                                }
                                _ => {
                                    // Handle other frames
                                    debug!("Received frame: {:?}", frame);
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!("Protocol error: {}", e);
                            return Err(e.into());
                        }
                        None => {
                            info!("Connection closed by server");
                            return Err(TunnelError::Connection("Connection closed".into()));
                        }
                    }
                }
            }
        }
    }
}
