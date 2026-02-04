//! Transport-agnostic frame send/recv abstraction.
//!
//! Enables the same protocol ([`Frame`]) to run over TCP (current), QUIC (v0.4), or HTTP/2 (v0.2)
//! without changing tunnel or multiplexer logic. See [extra/quic-like-framing-and-transport.md]
//! for the full design.

use ferrotunnel_common::Result;
use ferrotunnel_protocol::Frame;
use std::future::Future;
use std::pin::Pin;

/// Sends protocol frames over a connection.
///
/// Implemented by the current TCP path (batched sender) and future QUIC/HTTP2 transports.
/// Allows tunnel and multiplexer code to be transport-agnostic.
pub trait FrameSender: Send + Sync {
    /// Send a single frame. May buffer and flush internally.
    fn send_frame(&self, frame: Frame) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
}

/// Receives protocol frames from a connection.
///
/// Implemented by the current TCP path (FramedRead + codec) and future QUIC/HTTP2 transports.
pub trait FrameReceiver: Send {
    /// Receive the next frame. Returns `Ok(None)` on EOF.
    fn recv_frame(&mut self) -> Pin<Box<dyn Future<Output = Result<Option<Frame>>> + Send + '_>>;
}

/// Pair of frame send and receive capabilities.
///
/// A transport connection (TCP, QUIC, etc.) can be split into this pair so that
/// one task runs the sender and another the receiver, both using the same [`Frame`] type.
pub struct FrameConnectionSplit<S, R> {
    pub sender: S,
    pub receiver: R,
}

impl<S, R> FrameConnectionSplit<S, R> {
    pub const fn new(sender: S, receiver: R) -> Self {
        Self { sender, receiver }
    }
}
