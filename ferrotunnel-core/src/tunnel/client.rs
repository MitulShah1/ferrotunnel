use crate::auth::validate_token_format;
use crate::stream::multiplexer::{Multiplexer, VirtualStream};
use crate::transport::{self, TransportConfig};
use ferrotunnel_common::{Result, TunnelError};
use ferrotunnel_protocol::codec::TunnelCodec;
use ferrotunnel_protocol::frame::{Frame, HandshakeStatus};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use std::future::Future;
use std::time::Duration;
use tokio::time::interval;
use tokio_util::codec::Framed;
use tracing::{debug, error, info};
use uuid::Uuid;

pub struct TunnelClient {
    server_addr: String,
    auth_token: String,
    session_id: Option<Uuid>,
    transport_config: TransportConfig,
}

impl TunnelClient {
    pub fn new(server_addr: String, auth_token: String) -> Self {
        Self {
            server_addr,
            auth_token,
            session_id: None,
            transport_config: TransportConfig::default(),
        }
    }

    #[must_use]
    pub fn with_transport(mut self, config: TransportConfig) -> Self {
        self.transport_config = config;
        self
    }

    /// Connect to the server and start the session
    #[allow(clippy::too_many_lines)]
    pub async fn connect_and_run<F, Fut>(&mut self, stream_handler: F) -> Result<()>
    where
        F: Fn(VirtualStream) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        validate_token_format(&self.auth_token, 256)
            .map_err(|e| TunnelError::Authentication(format!("Invalid token: {e}")))?;

        info!("Connecting to {}", self.server_addr);

        let stream = transport::connect(&self.transport_config, &self.server_addr).await?;
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

        // 3. Setup Multiplexer
        let (mut sink, mut split_stream) = framed.split();
        let (frame_tx, mut frame_rx) = mpsc::channel::<Frame>(100);

        // Spawn sender task: Rx -> Sink
        tokio::spawn(async move {
            while let Some(frame) = frame_rx.next().await {
                if let Err(e) = sink.send(frame).await {
                    error!("Failed to send frame: {}", e);
                    break;
                }
            }
        });

        let (multiplexer, mut new_stream_rx) = Multiplexer::new(frame_tx);

        // Spawn stream handler
        tokio::spawn(async move {
            while let Some(stream) = new_stream_rx.next().await {
                stream_handler(stream).await;
            }
        });

        // 4. Heartbeat and Message Loop
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
                    if let Err(e) = multiplexer.send_frame(Frame::Heartbeat { timestamp: ts }).await {
                        error!("Failed to send heartbeat: {}", e);
                        return Err(e);
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
                                    // Handle other frames via multiplexer
                                    if let Err(e) = multiplexer.process_frame(frame).await {
                                        error!("Multiplexer error: {}", e);
                                    }
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
