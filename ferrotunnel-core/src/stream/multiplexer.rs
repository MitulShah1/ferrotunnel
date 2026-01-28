use bytes::Bytes;
use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use ferrotunnel_common::Result;
use ferrotunnel_protocol::frame::{DataFrame, Frame, OpenStreamFrame, Protocol};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tracing::{debug, warn};

/// Manages multiple virtual streams over a single connection
///
/// Uses lock-free data structures for high-concurrency performance:
/// - `DashMap` for concurrent stream access without global locks
/// - `AtomicU32` for lock-free stream ID allocation
#[derive(Clone, Debug)]
pub struct Multiplexer {
    streams: Arc<DashMap<u32, mpsc::Sender<Result<Frame>>>>,
    next_stream_id: Arc<AtomicU32>,
    frame_tx: mpsc::Sender<Frame>,
    new_stream_tx: mpsc::Sender<VirtualStream>,
}

impl Multiplexer {
    pub fn new(
        frame_tx: mpsc::Sender<Frame>,
        is_client: bool,
    ) -> (Self, mpsc::Receiver<VirtualStream>) {
        let (new_stream_tx, new_stream_rx) = mpsc::channel(10);
        let initial_stream_id = if is_client { 1 } else { 2 };
        (
            Self {
                streams: Arc::new(DashMap::new()),
                next_stream_id: Arc::new(AtomicU32::new(initial_stream_id)),
                frame_tx,
                new_stream_tx,
            },
            new_stream_rx,
        )
    }

    /// Allocate a new stream ID atomically (lock-free)
    #[inline]
    fn allocate_stream_id(&self) -> u32 {
        self.next_stream_id.fetch_add(2, Ordering::Relaxed)
    }

    /// Send a frame directly to the wire
    pub async fn send_frame(&self, frame: Frame) -> Result<()> {
        self.frame_tx
            .clone()
            .send(frame)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()).into())
    }

    /// Process an incoming frame from the wire
    pub async fn process_frame(&self, frame: Frame) -> Result<()> {
        match &frame {
            Frame::OpenStream(open_stream) => {
                let stream_id = open_stream.stream_id;
                debug!(
                    "Accepting new stream {} ({:?})",
                    stream_id, open_stream.protocol
                );
                let (tx, rx) = mpsc::channel(10);

                // Lock-free insertion using DashMap's entry API
                match self.streams.entry(stream_id) {
                    Entry::Occupied(_) => {
                        warn!("Stream {} already exists", stream_id);
                        return Ok(());
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(tx);
                    }
                }

                let stream = VirtualStream::new(stream_id, rx, self.frame_tx.clone());
                if self.new_stream_tx.clone().send(stream).await.is_err() {
                    warn!("Failed to queue new stream {}", stream_id);
                }
            }
            Frame::Data(data_frame) => {
                let stream_id = data_frame.stream_id;
                let tx = self.streams.get(&stream_id).map(|r| r.clone());

                if let Some(mut tx) = tx {
                    if tx.send(Ok(frame)).await.is_err() {
                        self.streams.remove(&stream_id);
                    }
                } else {
                    debug!("Received frame for unknown stream {}", stream_id);
                }
            }
            Frame::CloseStream { stream_id, .. } => {
                let stream_id = *stream_id;
                let tx = self.streams.get(&stream_id).map(|r| r.clone());

                if let Some(mut tx) = tx {
                    if tx.send(Ok(frame)).await.is_err() {
                        self.streams.remove(&stream_id);
                    }
                } else {
                    debug!("Received frame for unknown stream {}", stream_id);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Open a new outbound stream
    pub async fn open_stream(&self, protocol: Protocol) -> Result<VirtualStream> {
        // Lock-free stream ID allocation
        let stream_id = self.allocate_stream_id();

        let (tx, rx) = mpsc::channel(10);
        // Lock-free insertion
        self.streams.insert(stream_id, tx);

        let mut frame_tx = self.frame_tx.clone();
        frame_tx
            .send(Frame::OpenStream(Box::new(OpenStreamFrame {
                stream_id,
                protocol,
                headers: vec![],
                body_hint: None,
            })))
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e.to_string()))?;

        Ok(VirtualStream::new(stream_id, rx, frame_tx))
    }
}

/// A virtual stream that implements `AsyncRead` + `AsyncWrite`
#[derive(Debug)]
pub struct VirtualStream {
    stream_id: u32,
    rx: mpsc::Receiver<Result<Frame>>,
    tx: mpsc::Sender<Frame>,
    read_buffer: Vec<u8>,
}

impl VirtualStream {
    pub fn new(stream_id: u32, rx: mpsc::Receiver<Result<Frame>>, tx: mpsc::Sender<Frame>) -> Self {
        Self {
            stream_id,
            rx,
            tx,
            read_buffer: Vec::new(),
        }
    }

    pub fn id(&self) -> u32 {
        self.stream_id
    }
}

impl AsyncRead for VirtualStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if !self.read_buffer.is_empty() {
            let len = std::cmp::min(buf.remaining(), self.read_buffer.len());
            buf.put_slice(&self.read_buffer[..len]);
            self.read_buffer.drain(..len);
            return Poll::Ready(Ok(()));
        }

        match self.rx.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(Frame::Data(data_frame)))) => {
                let bytes = data_frame.data.to_vec();
                let len = std::cmp::min(buf.remaining(), bytes.len());
                buf.put_slice(&bytes[..len]);
                if len < bytes.len() {
                    self.read_buffer.extend_from_slice(&bytes[len..]);
                }
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Some(Ok(Frame::CloseStream { .. })) | None) => Poll::Ready(Ok(())),
            Poll::Ready(Some(Ok(_))) | Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Err(e))) => Poll::Ready(Err(io::Error::other(e.to_string()))),
        }
    }
}

impl AsyncWrite for VirtualStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // We need to clone to send, but poll_write takes &mut Self
        // For simplicity/performance in this MVP, we assume the channel has capacity.
        // In a real impl, we should use poll_ready.

        let frame = Frame::Data(Box::new(DataFrame {
            stream_id: self.stream_id,
            data: Bytes::copy_from_slice(buf),
            end_of_stream: false,
        }));

        match self.tx.poll_ready(cx) {
            Poll::Ready(Ok(())) => {
                if let Err(e) = self.tx.start_send(frame) {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        e.to_string(),
                    )));
                }
                Poll::Ready(Ok(buf.len()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                e.to_string(),
            ))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.tx.poll_flush_unpin(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                e.to_string(),
            ))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let frame = Frame::CloseStream {
            stream_id: self.stream_id,
            reason: ferrotunnel_protocol::frame::CloseReason::Normal,
        };

        match self.tx.poll_ready(cx) {
            Poll::Ready(Ok(())) => {
                if let Err(e) = self.tx.start_send(frame) {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        e.to_string(),
                    )));
                }
                let _ = self.tx.poll_flush_unpin(cx); // Try to flush
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                e.to_string(),
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stream_id_allocation() {
        let (tx, _rx) = mpsc::channel(100);

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
