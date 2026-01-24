use crate::stream::multiplexer::Multiplexer;
use crate::tunnel::session::Session;
use crate::tunnel::session::SessionStore;
use ferrotunnel_common::{Result, TunnelError};
use ferrotunnel_protocol::codec::TunnelCodec;
use ferrotunnel_protocol::frame::{Frame, HandshakeStatus};
use futures::channel::mpsc;
use futures::stream::SplitStream;
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub struct TunnelServer {
    addr: SocketAddr,
    auth_token: String,
    sessions: SessionStore,
    session_timeout: Duration,
}

impl TunnelServer {
    pub fn new(addr: SocketAddr, auth_token: String) -> Self {
        Self {
            addr,
            auth_token,
            sessions: SessionStore::new(),
            session_timeout: Duration::from_secs(90),
        }
    }

    pub fn sessions(&self) -> SessionStore {
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
            match listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("New connection from {}", addr);
                    let sessions = sessions.clone();
                    let token = self.auth_token.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, addr, sessions, token).await
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

    async fn handle_connection(
        stream: TcpStream,
        addr: SocketAddr,
        sessions: SessionStore,
        expected_token: String,
    ) -> Result<()> {
        let mut framed = Framed::new(stream, TunnelCodec::new());

        // 1. Handshake
        if let Some(result) = framed.next().await {
            let frame = result?;
            match frame {
                Frame::Handshake {
                    version,
                    token,
                    capabilities,
                } => {
                    if token != expected_token {
                        warn!("Invalid token from {}", addr);
                        framed
                            .send(Frame::HandshakeAck {
                                status: HandshakeStatus::InvalidToken,
                                session_id: Uuid::nil(),
                                server_capabilities: vec![],
                            })
                            .await?;
                        return Ok(());
                    }

                    if version != 1 {
                        warn!("Unsupported protocol version {} from {}", version, addr);
                        framed
                            .send(Frame::HandshakeAck {
                                status: HandshakeStatus::UnsupportedVersion,
                                session_id: Uuid::nil(),
                                server_capabilities: vec![],
                            })
                            .await?;
                        return Ok(());
                    }

                    // Success
                    let session_id = Uuid::new_v4();

                    // Setup multiplexer
                    let (mut sink, stream) = framed.split();
                    let (frame_tx, mut frame_rx) = mpsc::channel::<Frame>(100);

                    // Spawn sender task: Rx -> Sink
                    tokio::spawn(async move {
                        while let Some(frame) = frame_rx.next().await {
                            if let Err(e) = sink.send(frame).await {
                                warn!("Failed to send frame: {}", e);
                                break;
                            }
                        }
                    });

                    let (multiplexer, mut new_stream_rx) = Multiplexer::new(frame_tx);

                    // Log unexpected streams from client (for now)
                    tokio::spawn(async move {
                        while let Some(_stream) = new_stream_rx.next().await {
                            warn!("Client tried to open stream (not supported in MVP)");
                        }
                    });

                    let session = Session::new(
                        session_id,
                        addr,
                        token,
                        capabilities,
                        Some(multiplexer.clone()),
                    );
                    sessions.add(session);

                    info!("Session established: {}", session_id);
                    multiplexer
                        .send_frame(Frame::HandshakeAck {
                            status: HandshakeStatus::Success,
                            session_id,
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
        mut stream: SplitStream<Framed<TcpStream, TunnelCodec>>,
        session_id: Uuid,
        sessions: SessionStore,
        multiplexer: Multiplexer,
    ) -> Result<()> {
        while let Some(result) = stream.next().await {
            let frame = result?;

            // Update heartbeat for any activity
            if let Some(mut session) = sessions.get_mut(&session_id) {
                session.update_heartbeat();
            } else {
                // Session removed (shutdown/timeout)
                return Err(TunnelError::Protocol("Session not found".into()));
            }

            match frame {
                Frame::Heartbeat { .. } => {
                    debug!("Heartbeat from {}", session_id);
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
