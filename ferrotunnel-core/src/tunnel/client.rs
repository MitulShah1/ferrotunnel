use crate::auth::validate_token_format;
use crate::stream::multiplexer::{Multiplexer, VirtualStream};
use crate::transport::batched_sender::run_batched_sender;
use crate::transport::{self, TransportConfig};
use ferrotunnel_common::{Result, TunnelError};
use ferrotunnel_protocol::codec::TunnelCodec;
use ferrotunnel_protocol::constants::{MAX_PROTOCOL_VERSION, MIN_PROTOCOL_VERSION};
use ferrotunnel_protocol::frame::{Frame, HandshakeFrame, HandshakeStatus};
use futures::{SinkExt, StreamExt};
use kanal::bounded_async;
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
    tunnel_id: Option<String>,
    transport_config: TransportConfig,
}

impl TunnelClient {
    pub fn new(server_addr: String, auth_token: String) -> Self {
        Self {
            server_addr,
            auth_token,
            session_id: None,
            tunnel_id: None,
            transport_config: TransportConfig::default(),
        }
    }

    #[must_use]
    pub fn with_transport(mut self, config: TransportConfig) -> Self {
        self.transport_config = config;
        self
    }

    #[must_use]
    pub fn with_tunnel_id(mut self, tunnel_id: impl Into<String>) -> Self {
        self.tunnel_id = Some(tunnel_id.into());
        self
    }

    /// Enable TLS for the connection with certificate verification skipped.
    ///
    /// This is insecure and should only be used for self-signed certificates.
    #[must_use]
    pub fn with_tls_skip_verify(mut self) -> Self {
        if let TransportConfig::Tls(ref mut tls) = self.transport_config {
            tls.skip_verify = true;
        } else {
            self.transport_config = TransportConfig::Tls(transport::tls::TlsTransportConfig {
                skip_verify: true,
                ..Default::default()
            });
        }
        self
    }

    /// Enable TLS for the connection with a custom CA certificate.
    #[must_use]
    pub fn with_tls_ca(mut self, ca_cert_path: impl Into<std::path::PathBuf>) -> Self {
        let ca = Some(ca_cert_path.into().to_string_lossy().to_string());
        if let TransportConfig::Tls(ref mut tls) = self.transport_config {
            tls.ca_cert_path = ca;
        } else {
            self.transport_config = TransportConfig::Tls(transport::tls::TlsTransportConfig {
                ca_cert_path: ca,
                ..Default::default()
            });
        }
        self
    }

    /// Enable mutual TLS by providing a client certificate and private key.
    #[must_use]
    pub fn with_tls(
        mut self,
        cert_path: impl Into<std::path::PathBuf>,
        key_path: impl Into<std::path::PathBuf>,
    ) -> Self {
        let (cert, key) = (
            cert_path.into().to_string_lossy().to_string(),
            key_path.into().to_string_lossy().to_string(),
        );
        if let TransportConfig::Tls(ref mut tls) = self.transport_config {
            tls.cert_path = cert;
            tls.key_path = key;
        } else {
            self.transport_config = TransportConfig::Tls(transport::tls::TlsTransportConfig {
                cert_path: cert,
                key_path: key,
                ..Default::default()
            });
        }
        self
    }

    /// Set the server name (SNI) for TLS verification.
    #[must_use]
    pub fn with_server_name(mut self, server_name: impl Into<String>) -> Self {
        let name = Some(server_name.into());
        if let TransportConfig::Tls(ref mut tls) = self.transport_config {
            tls.server_name = name;
        } else {
            self.transport_config = TransportConfig::Tls(transport::tls::TlsTransportConfig {
                server_name: name,
                ..Default::default()
            });
        }
        self
    }

    /// Connect to the server and start the session
    #[allow(clippy::too_many_lines)]
    pub async fn connect_and_run<F, Fut>(&mut self, stream_handler: F) -> Result<()>
    where
        F: Fn(VirtualStream) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.connect_and_run_with_callback(stream_handler, |_| {})
            .await
    }

    /// Connect to the server, call the callback on successful handshake, and run the session
    #[allow(clippy::too_many_lines)]
    pub async fn connect_and_run_with_callback<F, Fut, C>(
        &mut self,
        stream_handler: F,
        on_connected: C,
    ) -> Result<()>
    where
        F: Fn(VirtualStream) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
        C: FnOnce(Uuid) + Send + 'static,
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

        debug!(
            "Sending handshake (v{}-{})",
            MIN_PROTOCOL_VERSION, MAX_PROTOCOL_VERSION
        );
        framed
            .send(Frame::Handshake(Box::new(HandshakeFrame {
                min_version: MIN_PROTOCOL_VERSION,
                max_version: MAX_PROTOCOL_VERSION,
                token: self.auth_token.clone(),
                tunnel_id: self.tunnel_id.clone(),
                capabilities: vec!["basic".to_string()],
            })))
            .await?;

        // 2. Wait for Ack
        if let Some(result) = framed.next().await {
            match result? {
                Frame::HandshakeAck {
                    status,
                    session_id,
                    version,
                    server_capabilities: _,
                } => match status {
                    HandshakeStatus::Success => {
                        self.session_id = Some(session_id);
                        info!(
                            "Handshake successful. Session ID: {}, Protocol v{}",
                            session_id, version
                        );
                        // Notify callback
                        on_connected(session_id);
                    }
                    HandshakeStatus::VersionMismatch => {
                        error!("Protocol version mismatch. Server requires different version.");
                        return Err(TunnelError::Protocol(
                            "No compatible protocol version found".into(),
                        ));
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

        // 3. Setup Multiplexer with kanal channels
        let (sink, mut split_stream) = framed.split();
        let (frame_tx, frame_rx) = bounded_async::<Frame>(100);

        // Spawn batched sender task for vectored I/O performance
        tokio::spawn(run_batched_sender(frame_rx, sink));

        let (multiplexer, new_stream_rx) = Multiplexer::new(frame_tx, true);

        // Spawn stream handler
        tokio::spawn(async move {
            while let Ok(stream) = new_stream_rx.recv().await {
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
