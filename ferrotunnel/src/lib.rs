//! # `FerroTunnel`
//!
//! A production-ready, secure reverse tunnel system in Rust.
//!
//! ## Overview
//!
//! `FerroTunnel` provides a complete reverse tunneling solution, allowing you to expose
//! local services through a public server. It's built with security, performance, and
//! reliability in mind.
//!
//! ## Features
//!
//! - 🔒 **Secure** - TLS encryption, token-based authentication
//! - ⚡ **Fast** - Built on Tokio for high-performance async I/O
//! - 🔌 **Protocol Support** - HTTP, HTTPS, WebSocket, gRPC, TCP
//! - 📊 **Observable** - Comprehensive logging and metrics
//! - 🛡️ **Resilient** - Automatic reconnection, heartbeat monitoring
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ferrotunnel::prelude::*;
//!
//! // Example usage (will be implemented with client/server)
//! ```
//!
//! ## Architecture
//!
//! `FerroTunnel` consists of several crates:
//!
//! - [`ferrotunnel-common`] - Shared types, errors, and utilities
//! - [`ferrotunnel-protocol`] - Wire protocol definitions and codec
//! - (Future) `ferrotunnel-client` - Client implementation
//! - (Future) `ferrotunnel-server` - Server implementation
//!
//! ## Re-exports
//!
//! This crate re-exports the most commonly used items from the subcrates
//! for convenience.

// Re-export subcrates
pub use ferrotunnel_common as common;
pub use ferrotunnel_protocol as protocol;

/// Prelude module for convenient imports
pub mod prelude {
    // Common types
    pub use crate::common::{Result, TunnelError};

    // Protocol types
    pub use crate::protocol::{
        CloseReason, Frame, HandshakeStatus, Protocol, RegisterStatus, StreamStatus, TunnelCodec,
    };
}

// Convenience re-exports at crate root
pub use common::{Result, TunnelError};
pub use protocol::{
    CloseReason, Frame, HandshakeStatus, Protocol, RegisterStatus, StreamStatus, TunnelCodec,
};
