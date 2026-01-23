//! Protocol frame definitions

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Wire protocol frame
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Frame {
    // Control frames
    /// Initial handshake from client to server
    Handshake {
        token: String,
        version: u8,
        capabilities: Vec<String>,
    },

    /// Handshake acknowledgment from server
    HandshakeAck {
        session_id: Uuid,
        status: HandshakeStatus,
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

    // Stream frames
    /// Open a new stream
    OpenStream {
        stream_id: u32,
        protocol: Protocol,
        headers: Vec<(String, String)>,
        body_hint: Option<u64>,
    },

    /// Stream acknowledgment
    StreamAck {
        stream_id: u32,
        status: StreamStatus,
    },

    Data {
        stream_id: u32,
        data: Bytes,
        end_of_stream: bool,
    },

    /// Close a stream
    CloseStream {
        stream_id: u32,
        reason: CloseReason,
    },

    // Keepalive
    /// Heartbeat ping
    Heartbeat {
        timestamp: u64,
    },

    /// Heartbeat acknowledgment
    HeartbeatAck {
        timestamp: u64,
    },

    // Error handling
    /// Error frame
    Error {
        stream_id: Option<u32>,
        code: ErrorCode,
        message: String,
    },

    // Plugin support (for future use)
    PluginData {
        plugin_id: String,
        data: Bytes,
    },
}

/// Handshake status codes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HandshakeStatus {
    Success,
    InvalidToken,
    UnsupportedVersion,
    RateLimited,
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
        let frame = Frame::Handshake {
            token: "test-token".to_string(),
            version: 1,
            capabilities: vec!["http".to_string()],
        };

        // Serialize
        let encoded = bincode::serialize(&frame).unwrap();

        // Deserialize
        let decoded: Frame = bincode::deserialize(&encoded).unwrap();

        assert_eq!(frame, decoded);
    }

    #[test]
    fn test_data_frame_with_bytes() {
        let data = Bytes::from("hello world");
        let frame = Frame::Data {
            stream_id: 42,
            data: data.clone(),
            end_of_stream: false,
        };

        let encoded = bincode::serialize(&frame).unwrap();
        let decoded: Frame = bincode::deserialize(&encoded).unwrap();

        if let Frame::Data {
            data: decoded_data, ..
        } = decoded
        {
            assert_eq!(data, decoded_data);
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
            let encoded = bincode::serialize(&frame).unwrap();
            let _decoded: Frame = bincode::deserialize(&encoded).unwrap();
        }
    }
}
