//! Stream multiplexer using lock-free data structures
//!
//! Manages multiple virtual streams over a single connection.

use super::bytes_pool;
use super::pool::ObjectPool;
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use ferrotunnel_common::Result;
use ferrotunnel_protocol::frame::{Frame, OpenStreamFrame, Protocol};
use kanal::{bounded_async, AsyncReceiver, AsyncSender, ReceiveError, SendError};
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tracing::warn;

/// Pool for reusing read buffers in `VirtualStream`
pub type ReadBufferPool = ObjectPool<Vec<u8>>;

/// Manages multiple virtual streams over a single connection
///
/// Uses lock-free data structures for high-concurrency performance:
/// - `DashMap` for concurrent stream access without global locks
/// - `AtomicU32` for lock-free stream ID allocation
/// - `kanal` for fast async channel throughput
/// - `ObjectPool` for read buffer reuse
#[derive(Clone, Debug)]
pub struct Multiplexer {
    streams: Arc<DashMap<u32, AsyncSender<Result<Frame>>>>,
    next_stream_id: Arc<AtomicU32>,
    frame_tx: AsyncSender<Frame>,
    new_stream_tx: AsyncSender<VirtualStream>,
    buffer_pool: ReadBufferPool,
}

impl Multiplexer {
    pub fn new(
        frame_tx: AsyncSender<Frame>,
        is_client: bool,
    ) -> (Self, AsyncReceiver<VirtualStream>) {
        let (new_stream_tx, new_stream_rx) = bounded_async(10);
        let initial_stream_id = if is_client { 1 } else { 2 };
        (
            Self {
                streams: Arc::new(DashMap::new()),
                next_stream_id: Arc::new(AtomicU32::new(initial_stream_id)),
                frame_tx,
                new_stream_tx,
                buffer_pool: ReadBufferPool::with_default_capacity(),
            },
            new_stream_rx,
        )
    }

    /// Get the buffer pool for reusing read buffers
    pub fn buffer_pool(&self) -> &ReadBufferPool {
        &self.buffer_pool
    }

    /// Allocate a new stream ID atomically (lock-free)
    #[inline]
    fn allocate_stream_id(&self) -> u32 {
        self.next_stream_id.fetch_add(2, Ordering::Relaxed)
    }

    /// Send a frame directly to the wire
    pub async fn send_frame(&self, frame: Frame) -> Result<()> {
        self.frame_tx
            .send(frame)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()).into())
    }

    /// Process an incoming frame from the wire
    pub async fn process_frame(&self, frame: Frame) -> Result<()> {
        match &frame {
            Frame::OpenStream(open_stream) => {
                let stream_id = open_stream.stream_id;
                let (tx, rx) = bounded_async(10);

                match self.streams.entry(stream_id) {
                    Entry::Occupied(_) => {
                        warn!("Stream {} already exists", stream_id);
                        return Ok(());
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(tx);
                    }
                }

                let read_buffer = self.buffer_pool.try_acquire().unwrap_or_default();
                let stream = VirtualStream::new_with_buffer(
                    stream_id,
                    rx,
                    self.frame_tx.clone(),
                    read_buffer,
                    self.buffer_pool.clone(),
                    open_stream.protocol,
                );
                if self.new_stream_tx.send(stream).await.is_err() {
                    warn!("Failed to queue new stream {}", stream_id);
                }
            }
            Frame::Data { stream_id, .. } => {
                let stream_id = *stream_id;
                let tx = self.streams.get(&stream_id).map(|r| r.clone());

                if let Some(tx) = tx {
                    if tx.send(Ok(frame)).await.is_err() {
                        self.streams.remove(&stream_id);
                    }
                }
            }
            Frame::CloseStream { stream_id, .. } => {
                let stream_id = *stream_id;
                let tx = self.streams.get(&stream_id).map(|r| r.clone());

                if let Some(tx) = tx {
                    if tx.send(Ok(frame)).await.is_err() {
                        self.streams.remove(&stream_id);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Open a new outbound stream
    pub async fn open_stream(&self, protocol: Protocol) -> Result<VirtualStream> {
        let stream_id = self.allocate_stream_id();

        let (tx, rx) = bounded_async(10);
        self.streams.insert(stream_id, tx);

        self.frame_tx
            .send(Frame::OpenStream(Box::new(OpenStreamFrame {
                stream_id,
                protocol,
                headers: vec![],
                body_hint: None,
            })))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()))?;

        let read_buffer = self.buffer_pool.try_acquire().unwrap_or_default();
        Ok(VirtualStream::new_with_buffer(
            stream_id,
            rx,
            self.frame_tx.clone(),
            read_buffer,
            self.buffer_pool.clone(),
            protocol,
        ))
    }
}

/// Boxed future type for receiving frames
type RecvFuture = Pin<
    Box<dyn std::future::Future<Output = std::result::Result<Result<Frame>, ReceiveError>> + Send>,
>;

/// Boxed future type for sending frames
type SendFuture =
    Pin<Box<dyn std::future::Future<Output = std::result::Result<(), SendError>> + Send>>;

/// A virtual stream that implements `AsyncRead` + `AsyncWrite`
///
/// Uses kanal channels for async communication.
/// The polling implementation uses boxed futures to bridge kanal's
/// async API with tokio's poll-based traits.
///
/// Read buffers are pooled and returned when the stream is dropped.
pub struct VirtualStream {
    stream_id: u32,
    rx: AsyncReceiver<Result<Frame>>,
    tx: AsyncSender<Frame>,
    read_buffer: Vec<u8>,
    /// Pool to return `read_buffer` to when dropped
    buffer_pool: Option<ReadBufferPool>,
    /// Pending receive future for `poll_read`
    pending_recv: Option<RecvFuture>,
    /// Pending send future for `poll_write`
    pending_send: Option<SendFuture>,
    /// Buffered frame to send
    pending_send_frame: Option<Frame>,
    /// Protocol for this stream
    protocol: Protocol,
}

impl std::fmt::Debug for VirtualStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualStream")
            .field("stream_id", &self.stream_id)
            .field("read_buffer_len", &self.read_buffer.len())
            .finish_non_exhaustive()
    }
}

impl VirtualStream {
    /// Create a new `VirtualStream` without pooling (for backward compatibility)
    pub fn new(
        stream_id: u32,
        rx: AsyncReceiver<Result<Frame>>,
        tx: AsyncSender<Frame>,
        protocol: Protocol,
    ) -> Self {
        Self {
            stream_id,
            rx,
            tx,
            read_buffer: Vec::new(),
            buffer_pool: None,
            pending_recv: None,
            pending_send: None,
            pending_send_frame: None,
            protocol,
        }
    }

    /// Create a new `VirtualStream` with a pooled buffer
    pub fn new_with_buffer(
        stream_id: u32,
        rx: AsyncReceiver<Result<Frame>>,
        tx: AsyncSender<Frame>,
        read_buffer: Vec<u8>,
        buffer_pool: ReadBufferPool,
        protocol: Protocol,
    ) -> Self {
        Self {
            stream_id,
            rx,
            tx,
            read_buffer,
            buffer_pool: Some(buffer_pool),
            pending_recv: None,
            pending_send: None,
            pending_send_frame: None,
            protocol,
        }
    }

    pub fn id(&self) -> u32 {
        self.stream_id
    }

    pub fn protocol(&self) -> Protocol {
        self.protocol
    }
}

impl Drop for VirtualStream {
    fn drop(&mut self) {
        // Return the read buffer to the pool if available
        if let Some(pool) = self.buffer_pool.take() {
            let buffer = std::mem::take(&mut self.read_buffer);
            pool.release(buffer);
        }
    }
}

impl AsyncRead for VirtualStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // First, drain any buffered data
        if !self.read_buffer.is_empty() {
            let len = std::cmp::min(buf.remaining(), self.read_buffer.len());
            buf.put_slice(&self.read_buffer[..len]);
            self.read_buffer.drain(..len);
            return Poll::Ready(Ok(()));
        }

        // Check if we have a pending receive future
        if self.pending_recv.is_none() {
            let rx = self.rx.clone();
            self.pending_recv = Some(Box::pin(async move { rx.recv().await }));
        }

        // Poll the pending future - unwrap is safe, we just set it above
        #[allow(clippy::unwrap_used)]
        let fut = self.pending_recv.as_mut().unwrap();
        match fut.as_mut().poll(cx) {
            Poll::Ready(result) => {
                self.pending_recv = None;
                match result {
                    Ok(Ok(Frame::Data {
                        data: bytes,
                        end_of_stream: _,
                        ..
                    })) => {
                        let len = std::cmp::min(buf.remaining(), bytes.len());
                        buf.put_slice(&bytes[..len]);
                        if len < bytes.len() {
                            self.read_buffer.extend_from_slice(&bytes[len..]);
                        }
                        Poll::Ready(Ok(()))
                    }
                    Ok(Ok(Frame::CloseStream { .. })) | Err(ReceiveError::Closed) => {
                        Poll::Ready(Ok(())) // EOF
                    }
                    Ok(Ok(_)) => Poll::Pending, // Ignore other frame types
                    Ok(Err(e)) => Poll::Ready(Err(io::Error::other(e.to_string()))),
                    Err(ReceiveError::SendClosed) => Poll::Ready(Ok(())), // EOF
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for VirtualStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // If we have a pending send, poll it first
        if let Some(fut) = self.pending_send.as_mut() {
            match fut.as_mut().poll(cx) {
                Poll::Ready(result) => {
                    self.pending_send = None;
                    self.pending_send_frame = None;
                    if let Err(e) = result {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            e.to_string(),
                        )));
                    }
                    // Previous send completed, return its length
                    return Poll::Ready(Ok(buf.len()));
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        // Create new frame using pooled buffer (zero-copy optimization)
        // Instead of Bytes::copy_from_slice(), use pooled BytesMut
        let mut bytes_mut = bytes_pool::acquire_bytes(buf.len());
        bytes_mut.extend_from_slice(buf);
        let data = bytes_mut.freeze();

        let frame = Frame::Data {
            stream_id: self.stream_id,
            data,
            end_of_stream: false,
        };

        let tx = self.tx.clone();
        let frame_clone = frame.clone();
        self.pending_send_frame = Some(frame);
        self.pending_send = Some(Box::pin(async move { tx.send(frame_clone).await }));

        // Poll the new future - unwrap is safe, we just set it above
        #[allow(clippy::unwrap_used)]
        let fut = self.pending_send.as_mut().unwrap();
        match fut.as_mut().poll(cx) {
            Poll::Ready(result) => {
                self.pending_send = None;
                self.pending_send_frame = None;
                match result {
                    Ok(()) => Poll::Ready(Ok(buf.len())),
                    Err(e) => Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        e.to_string(),
                    ))),
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Kanal channels don't require explicit flushing
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // If we have a pending send, poll it first
        if let Some(fut) = self.pending_send.as_mut() {
            match fut.as_mut().poll(cx) {
                Poll::Ready(_) => {
                    self.pending_send = None;
                    self.pending_send_frame = None;
                }
                Poll::Pending => return Poll::Pending,
            }
        }

        // Send close frame
        let frame = Frame::CloseStream {
            stream_id: self.stream_id,
            reason: ferrotunnel_protocol::frame::CloseReason::Normal,
        };

        let tx = self.tx.clone();
        self.pending_send = Some(Box::pin(async move { tx.send(frame).await }));

        // unwrap is safe, we just set it above
        #[allow(clippy::unwrap_used)]
        let fut = self.pending_send.as_mut().unwrap();
        match fut.as_mut().poll(cx) {
            Poll::Ready(result) => {
                self.pending_send = None;
                match result {
                    Ok(()) => Poll::Ready(Ok(())),
                    Err(e) => Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        e.to_string(),
                    ))),
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stream_id_allocation() {
        let (tx, _rx) = bounded_async(100);

        // Client (Odd IDs)
        let (client_mux, _client_streams) = Multiplexer::new(tx.clone(), true);

        let s1 = client_mux.open_stream(Protocol::HTTP).await.unwrap();
        assert_eq!(s1.id(), 1);

        let s2 = client_mux.open_stream(Protocol::HTTP).await.unwrap();
        assert_eq!(s2.id(), 3);

        // Server (Even IDs)
        let (server_mux, _server_streams) = Multiplexer::new(tx, false);

        let s3 = server_mux.open_stream(Protocol::HTTP).await.unwrap();
        assert_eq!(s3.id(), 2);

        let s4 = server_mux.open_stream(Protocol::HTTP).await.unwrap();
        assert_eq!(s4.id(), 4);
    }
}
