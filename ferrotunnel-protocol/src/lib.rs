//! `FerroTunnel` wire protocol
//!
//! This crate defines the binary protocol used for communication between
//! `FerroTunnel` clients and servers.

pub mod codec;
pub mod constants;
pub mod frame;

pub use codec::TunnelCodec;
pub use frame::{CloseReason, Frame, HandshakeStatus, Protocol, RegisterStatus, StreamStatus};
