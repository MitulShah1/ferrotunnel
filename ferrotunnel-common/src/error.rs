//! Error types for `FerroTunnel`

use thiserror::Error;
use uuid::Uuid;

/// Main error type for `FerroTunnel` operations
#[derive(Error, Debug)]
pub enum TunnelError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    /// Stream not found
    #[error("Stream {0} not found")]
    StreamNotFound(u32),

    /// Timeout error
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// TLS error
    #[error("TLS error: {0}")]
    Tls(String),

    /// Connection error
    #[error("Connection failed: {0}")]
    Connection(String),

    /// Service unavailable
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Result type alias
pub type Result<T> = std::result::Result<T, TunnelError>;

impl From<bincode_next::error::EncodeError> for TunnelError {
    fn from(err: bincode_next::error::EncodeError) -> Self {
        TunnelError::Serialization(err.to_string())
    }
}

impl From<bincode_next::error::DecodeError> for TunnelError {
    fn from(err: bincode_next::error::DecodeError) -> Self {
        TunnelError::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TunnelError::Protocol("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::other("test");
        let tunnel_err: TunnelError = io_err.into();
        assert!(matches!(tunnel_err, TunnelError::Io(_)));
    }
}
