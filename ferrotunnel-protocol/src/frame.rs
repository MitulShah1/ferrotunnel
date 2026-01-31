//! Protocol frame definitions

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Data frame payload - boxed to reduce Frame enum size
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataFrame {
    pub stream_id: u32,
    pub data: Bytes,
    pub end_of_stream: bool,
}

/// Stream open request payload - boxed to reduce Frame enum size
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenStreamFrame {
    pub stream_id: u32,
    pub protocol: Protocol,
    pub headers: Vec<(String, String)>,
    pub body_hint: Option<u64>,
}

/// Handshake payload - boxed to reduce Frame enum size
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandshakeFrame {
    pub token: String,
    pub tunnel_id: Option<String>,
    /// Minimum protocol version supported by this peer
    pub min_version: u8,
    /// Maximum protocol version supported by this peer
    pub max_version: u8,
    pub capabilities: Vec<String>,
}

/// Wire protocol frame
///
/// Large variants are boxed to keep stack size small for control frames.
/// This provides ~60% stack reduction for small frames like Heartbeat.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Frame {
    /// Initial handshake from client to server
    Handshake(Box<HandshakeFrame>),

    /// Handshake acknowledgment from server
    HandshakeAck {
        session_id: Uuid,
        status: HandshakeStatus,
        /// Negotiated protocol version
        version: u8,
        server_capabilities: Vec<String>,
    },

    /// Register a service
    Register {
        service_name: String,
        protocol: Protocol,
        metadata: HashMap<String, String>,
    },

    /// Registration response
    RegisterAck {
        public_url: String,
        status: RegisterStatus,
    },

    /// Open a new stream
    OpenStream(Box<OpenStreamFrame>),

    /// Stream acknowledgment
    StreamAck {
        stream_id: u32,
        status: StreamStatus,
    },

    /// Data frame
    Data(Box<DataFrame>),

    /// Close a stream
    CloseStream { stream_id: u32, reason: CloseReason },

    /// Heartbeat ping
    Heartbeat { timestamp: u64 },

    /// Heartbeat acknowledgment
    HeartbeatAck { timestamp: u64 },

    /// Error frame
    Error {
        stream_id: Option<u32>,
        code: ErrorCode,
        message: String,
    },

    /// Plugin data (for future use)
    PluginData { plugin_id: String, data: Bytes },
}

/// Handshake status codes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HandshakeStatus {
    Success,
    InvalidToken,
    UnsupportedVersion,
    VersionMismatch,
    RateLimited,
    TunnelIdTaken,
}

/// Registration status codes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegisterStatus {
    Success,
    ServiceNameTaken,
    InvalidServiceName,
    QuotaExceeded,
}

/// Stream status codes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StreamStatus {
    Accepted,
    Rejected,
    BackpressureApplied,
}

/// Supported protocols
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Protocol {
    HTTP,
    HTTPS,
    WebSocket,
    GRPC,
    TCP,
}

/// Stream close reasons
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CloseReason {
    Normal,
    Timeout,
    Error(String),
    LocalServiceUnreachable,
    ProtocolViolation,
}

/// Error codes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCode {
    ProtocolError = 1,
    AuthenticationFailed = 2,
    SessionNotFound = 3,
    StreamNotFound = 4,
    Timeout = 5,
    InternalServerError = 6,
    ServiceUnavailable = 7,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_serialization() {
        let frame = Frame::Handshake(Box::new(HandshakeFrame {
            token: "test-token".to_string(),
            tunnel_id: Some("test-tunnel".to_string()),
            min_version: 1,
            max_version: 1,
            capabilities: vec!["http".to_string()],
        }));

        let config = bincode_next::config::standard();
        let encoded = bincode_next::serde::encode_to_vec(&frame, config).unwrap();
        let (decoded, _): (Frame, usize) =
            bincode_next::serde::decode_from_slice(&encoded, config).unwrap();

        assert_eq!(frame, decoded);
    }

    #[test]
    fn test_data_frame_with_bytes() {
        let data = Bytes::from("hello world");
        let frame = Frame::Data(Box::new(DataFrame {
            stream_id: 42,
            data: data.clone(),
            end_of_stream: false,
        }));

        let config = bincode_next::config::standard();
        let encoded = bincode_next::serde::encode_to_vec(&frame, config).unwrap();
        let (decoded, _): (Frame, usize) =
            bincode_next::serde::decode_from_slice(&encoded, config).unwrap();

        if let Frame::Data(data_frame) = decoded {
            assert_eq!(data, data_frame.data);
        } else {
            panic!("Expected Data frame");
        }
    }

    #[test]
    fn test_all_frame_types() {
        let frames = vec![
            Frame::Heartbeat { timestamp: 123_456 },
            Frame::HeartbeatAck { timestamp: 123_456 },
            Frame::CloseStream {
                stream_id: 1,
                reason: CloseReason::Normal,
            },
            Frame::Error {
                stream_id: Some(1),
                code: ErrorCode::ProtocolError,
                message: "test error".to_string(),
            },
        ];

        for frame in frames {
            let config = bincode_next::config::standard();
            let encoded = bincode_next::serde::encode_to_vec(&frame, config).unwrap();
            let (_decoded, _): (Frame, usize) =
                bincode_next::serde::decode_from_slice(&encoded, config).unwrap();
        }
    }
}
