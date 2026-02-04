use crate::auth::{constant_time_eq, validate_token_format};
use crate::resource_limits::{ServerResourceLimits, SessionPermit};
use crate::stream::{Multiplexer, PrioritizedFrame};
use crate::transport::batched_sender::run_batched_sender;
use crate::transport::{self, BoxedStream, TransportConfig};
use crate::tunnel::session::{Session, SessionStoreBackend, ShardedSessionStore};
use ferrotunnel_common::{Result, TunnelError};
use ferrotunnel_protocol::codec::TunnelCodec;
use ferrotunnel_protocol::constants::{MAX_PROTOCOL_VERSION, MIN_PROTOCOL_VERSION};
use ferrotunnel_protocol::frame::{Frame, HandshakeFrame, HandshakeStatus};
use futures::{SinkExt, StreamExt};
use kanal::bounded_async;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio_util::codec::Framed;
use tracing::{error, info, warn};
use uuid::Uuid;

pub struct TunnelServer {
    addr: SocketAddr,
    auth_token: String,
    sessions: SessionStoreBackend,
    session_timeout: Duration,
    resource_limits: ServerResourceLimits,
    transport_config: TransportConfig,
}

impl TunnelServer {
    pub fn new(addr: SocketAddr, auth_token: String) -> Self {
        Self {
            addr,
            auth_token,
            sessions: SessionStoreBackend::default(),
            session_timeout: Duration::from_secs(90),
            resource_limits: ServerResourceLimits::default(),
            transport_config: TransportConfig::default(),
        }
    }

    /// Use a sharded session store for lower contention under many concurrent tunnel_id lookups.
    #[must_use]
    pub fn with_sharded_sessions(mut self, n_shards: usize) -> Self {
        self.sessions = SessionStoreBackend::Sharded(ShardedSessionStore::with_shards(n_shards));
        self
    }

    #[must_use]
    pub fn with_resource_limits(mut self, limits: ServerResourceLimits) -> Self {
        self.resource_limits = limits;
        self
    }

    #[must_use]
    pub fn with_transport(mut self, config: TransportConfig) -> Self {
        self.transport_config = config;
        self
    }

    /// Configure TLS for the server using certificate and key files.
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

    /// Enable client certificate authentication (Mutual TLS) using the provided CA.
    #[must_use]
    pub fn with_client_auth(mut self, ca_path: impl Into<std::path::PathBuf>) -> Self {
        let ca = Some(ca_path.into().to_string_lossy().to_string());
        if let TransportConfig::Tls(ref mut tls) = self.transport_config {
            tls.ca_cert_path = ca;
            tls.client_auth = true;
        } else {
            self.transport_config = TransportConfig::Tls(transport::tls::TlsTransportConfig {
                ca_cert_path: ca,
                client_auth: true,
                ..Default::default()
            });
        }
        self
    }

    pub fn sessions(&self) -> SessionStoreBackend {
        self.sessions.clone()
    }

    pub async fn run(self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        info!("Server listening on {}", self.addr);

        let sessions = self.sessions.clone();
        let timeout = self.session_timeout;

        // Spawn session cleanup task
        let cleanup_sessions = sessions.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let count = cleanup_sessions.cleanup_stale_sessions(timeout);
                if count > 0 {
                    info!("Cleaned up {} stale sessions", count);
                }
            }
        });

        loop {
            match transport::accept(&self.transport_config, &listener).await {
                Ok((stream, addr)) => {
                    let session_permit = match self.resource_limits.try_acquire_session() {
                        Ok(permit) => permit,
                        Err(e) => {
                            warn!("Rejecting connection from {}: {}", addr, e);
                            continue;
                        }
                    };

                    let sessions = sessions.clone();
                    let token = self.auth_token.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_connection(stream, addr, sessions, token, session_permit)
                                .await
                        {
                            warn!("Connection error for {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn handle_connection(
        stream: BoxedStream,
        addr: SocketAddr,
        sessions: SessionStoreBackend,
        expected_token: String,
        _session_permit: SessionPermit,
    ) -> Result<()> {
        let mut framed = Framed::new(stream, TunnelCodec::new());

        // 1. Handshake
        if let Some(result) = framed.next().await {
            let frame = result?;
            match frame {
                Frame::Handshake(handshake) => {
                    let HandshakeFrame {
                        min_version,
                        max_version,
                        token,
                        tunnel_id,
                        capabilities,
                    } = *handshake;
                    if let Err(e) = validate_token_format(&token, 256) {
                        warn!("Invalid token format from {}: {}", addr, e);
                        framed
                            .send(Frame::HandshakeAck {
                                status: HandshakeStatus::InvalidToken,
                                session_id: Uuid::nil(),
                                version: 0,
                                server_capabilities: vec![],
                            })
                            .await?;
                        return Ok(());
                    }

                    if !constant_time_eq(token.as_bytes(), expected_token.as_bytes()) {
                        warn!("Invalid token from {}", addr);
                        framed
                            .send(Frame::HandshakeAck {
                                status: HandshakeStatus::InvalidToken,
                                session_id: Uuid::nil(),
                                version: 0,
                                server_capabilities: vec![],
                            })
                            .await?;
                        return Ok(());
                    }

                    // Version negotiation
                    let negotiated_version = match negotiate_version(min_version, max_version) {
                        Ok(v) => v,
                        Err(e) => {
                            warn!("Version negotiation failed for {}: {}", addr, e);
                            framed
                                .send(Frame::HandshakeAck {
                                    status: HandshakeStatus::VersionMismatch,
                                    session_id: Uuid::nil(),
                                    version: 0,
                                    server_capabilities: vec![],
                                })
                                .await?;
                            return Ok(());
                        }
                    };

                    info!(
                        "Client {} supports v{}-{}, negotiated v{}",
                        addr, min_version, max_version, negotiated_version
                    );

                    // Success
                    let session_id = Uuid::new_v4();

                    // Determine tunnel ID: prefer requested, fallback to random session ID
                    let tunnel_id = tunnel_id.unwrap_or_else(|| session_id.to_string());

                    // Setup multiplexer with kanal channels
                    let parts = framed.into_parts();
                    let (read_half, write_half) = tokio::io::split(parts.io);

                    // Reconstruct FramedRead for reading, preserving any buffered data.
                    // Dropping read_buf causes decoder desync ("Frame too large: 2021161080").
                    let mut stream = tokio_util::codec::FramedRead::new(read_half, parts.codec);
                    if !parts.read_buf.is_empty() {
                        stream.read_buffer_mut().extend_from_slice(&parts.read_buf);
                    }

                    let (frame_tx, frame_rx) = bounded_async::<PrioritizedFrame>(1024);

                    // Spawn batched sender task for vectored I/O performance
                    tokio::spawn(run_batched_sender(frame_rx, write_half, parts.codec));

                    let (multiplexer, new_stream_rx) = Multiplexer::new(frame_tx, false);

                    // Log unexpected streams from client (for now)
                    tokio::spawn(async move {
                        while let Ok(_stream) = new_stream_rx.recv().await {
                            warn!("Client tried to open stream (not supported in MVP)");
                        }
                    });

                    let session = Session::new(
                        session_id,
                        tunnel_id.clone(),
                        addr,
                        token,
                        capabilities,
                        Some(multiplexer.clone()),
                    );

                    if let Err(e) = sessions.add(session) {
                        warn!("Failed to register session: {}", e);
                        multiplexer
                            .send_frame(Frame::HandshakeAck {
                                status: HandshakeStatus::TunnelIdTaken,
                                session_id,
                                version: 0,
                                server_capabilities: vec![],
                            })
                            .await?;
                        return Err(TunnelError::Protocol(format!(
                            "Tunnel ID '{tunnel_id}' already in use"
                        )));
                    }

                    info!("Session established: {}", session_id);
                    multiplexer
                        .send_frame(Frame::HandshakeAck {
                            status: HandshakeStatus::Success,
                            session_id,
                            version: negotiated_version,
                            server_capabilities: vec!["basic".to_string()],
                        })
                        .await?;

                    // Enter message loop
                    Self::process_messages(stream, session_id, sessions, multiplexer).await?;
                }
                _ => {
                    return Err(TunnelError::Protocol("Expected handshake".into()));
                }
            }
        } else {
            return Err(TunnelError::Connection("Connection closed".into()));
        }

        Ok(())
    }

    async fn process_messages(
        mut stream: tokio_util::codec::FramedRead<tokio::io::ReadHalf<BoxedStream>, TunnelCodec>,
        session_id: Uuid,
        sessions: SessionStoreBackend,
        multiplexer: Multiplexer,
    ) -> Result<()> {
        loop {
            #[cfg_attr(not(feature = "metrics"), allow(unused_variables))]
            let decode_start = Instant::now();
            let result = stream.next().await;
            let Some(frame_result) = result else { break };
            let frame = frame_result?;

            #[cfg(feature = "metrics")]
            if let Some(m) = ferrotunnel_observability::tunnel_metrics() {
                let bytes = match &frame {
                    Frame::Data { data, .. } => data.len(),
                    _ => 0,
                };
                m.record_decode(1, bytes, decode_start.elapsed());
            }

            // Update heartbeat for any activity
            if let Some(mut session) = sessions.get_mut(&session_id) {
                session.update_heartbeat();
            } else {
                // Session removed (shutdown/timeout)
                return Err(TunnelError::Protocol("Session not found".into()));
            }

            match frame {
                Frame::Heartbeat { .. } => {
                    multiplexer
                        .send_frame(Frame::HeartbeatAck {
                            #[allow(clippy::cast_possible_truncation)]
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                        })
                        .await?;
                }
                _ => {
                    multiplexer.process_frame(frame).await?;
                }
            }
        }

        info!("Client disconnected: {}", session_id);
        sessions.remove(&session_id);
        Ok(())
    }
}

/// Negotiate protocol version between client and server
fn negotiate_version(client_min: u8, client_max: u8) -> Result<u8> {
    // Find highest common version
    let server_min = MIN_PROTOCOL_VERSION;
    let server_max = MAX_PROTOCOL_VERSION;

    if client_max < server_min || client_min > server_max {
        // No overlap
        return Err(TunnelError::Protocol(format!(
            "No compatible protocol version. Server: {server_min}-{server_max}, Client: {client_min}-{client_max}"
        )));
    }

    // Use highest common version
    let negotiated = std::cmp::min(client_max, server_max);
    Ok(negotiated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_negotiation_success() {
        // Client supports 1-2, Server supports 1-1 → v1
        assert_eq!(negotiate_version(1, 2).unwrap(), 1);

        // Client supports 1-1, Server supports 1-2 → v1
        assert_eq!(negotiate_version(1, 1).unwrap(), 1);
    }

    #[test]
    fn test_version_negotiation_failure() {
        // Client requires 3+, Server only has 1
        assert!(negotiate_version(3, 5).is_err());
    }
}
