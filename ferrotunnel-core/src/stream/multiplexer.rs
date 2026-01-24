use bytes::Bytes;
use ferrotunnel_common::Result;
use ferrotunnel_protocol::frame::{Frame, Protocol};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tracing::{debug, warn};

/// Manages multiple virtual streams over a single connection
#[derive(Clone, Debug)]
pub struct Multiplexer {
    inner: Arc<Mutex<Inner>>,
    frame_tx: mpsc::Sender<Frame>,
    new_stream_tx: mpsc::Sender<VirtualStream>,
}

#[derive(Debug)]
struct Inner {
    streams: HashMap<u32, mpsc::Sender<Result<Frame>>>,
    next_stream_id: u32,
}

#[allow(clippy::expect_used)]
#[allow(clippy::unwrap_used)]
impl Multiplexer {
    pub fn new(frame_tx: mpsc::Sender<Frame>) -> (Self, mpsc::Receiver<VirtualStream>) {
        let (new_stream_tx, new_stream_rx) = mpsc::channel(10);
        let inner = Inner {
            streams: HashMap::new(),
            next_stream_id: 1,
        };
        (
            Self {
                inner: Arc::new(Mutex::new(inner)),
                frame_tx,
                new_stream_tx,
            },
            new_stream_rx,
        )
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
        match frame {
            Frame::OpenStream {
                stream_id,
                protocol,
                ..
            } => {
                // Incoming new stream request
                debug!("Accepting new stream {} ({:?})", stream_id, protocol);
                let (tx, rx) = mpsc::channel(10);

                {
                    let mut inner = self.inner.lock().unwrap();
                    if inner.streams.contains_key(&stream_id) {
                        warn!("Stream {} already exists", stream_id);
                        return Ok(());
                    }
                    inner.streams.insert(stream_id, tx);
                }

                let stream = VirtualStream::new(stream_id, rx, self.frame_tx.clone());
                if self.new_stream_tx.clone().send(stream).await.is_err() {
                    warn!("Failed to queue new stream {}", stream_id);
                }
            }
            Frame::Data { stream_id, .. } | Frame::CloseStream { stream_id, .. } => {
                let mut tx = {
                    let inner = self.inner.lock().unwrap();
                    inner.streams.get(&stream_id).cloned()
                };

                if let Some(tx) = &mut tx {
                    if let Err(_e) = tx.send(Ok(frame)).await {
                        // Stream receiver dropped, cleanup
                        let mut inner = self.inner.lock().unwrap();
                        inner.streams.remove(&stream_id);
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
        let stream_id = {
            let mut inner = self.inner.lock().unwrap();
            let id = inner.next_stream_id;
            inner.next_stream_id += 2; // Increment by 2 to avoid collisions (client odd, server even)
                                       // TODO: properly handle initiator ID parity
            id
        };

        let (tx, rx) = mpsc::channel(10);
        {
            let mut inner = self.inner.lock().unwrap();
            inner.streams.insert(stream_id, tx);
        }

        let mut frame_tx = self.frame_tx.clone();
        frame_tx
            .send(Frame::OpenStream {
                stream_id,
                protocol,
                headers: vec![],
                body_hint: None,
            })
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
            Poll::Ready(Some(Ok(Frame::Data { data, .. }))) => {
                let bytes = data.to_vec();
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

        let frame = Frame::Data {
            stream_id: self.stream_id,
            data: Bytes::copy_from_slice(buf),
            end_of_stream: false,
        };

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
