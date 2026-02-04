//! TCP implementation of [`FrameSender`] and [`FrameReceiver`].

use super::{FrameReceiver, FrameSender};
use crate::stream::PrioritizedFrame;
use ferrotunnel_common::Result;
use ferrotunnel_protocol::codec::TunnelCodec;
use ferrotunnel_protocol::frame::StreamPriority;
use ferrotunnel_protocol::Frame;
use futures::StreamExt;
use kanal::AsyncSender;
use std::future::Future;
use std::pin::Pin;
use tokio::io::AsyncRead;
use tokio_util::codec::FramedRead;

/// Sends frames over TCP by pushing to the channel consumed by the batched sender task.
#[derive(Clone)]
pub struct TcpFrameSender {
    tx: AsyncSender<PrioritizedFrame>,
}

impl TcpFrameSender {
    pub fn new(tx: AsyncSender<PrioritizedFrame>) -> Self {
        Self { tx }
    }
}

impl FrameSender for TcpFrameSender {
    fn send_frame(&self, frame: Frame) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        let tx = self.tx.clone();
        Box::pin(async move {
            // Trait API has no priority; use Normal when sending via trait.
            tx.send((StreamPriority::Normal, frame))
                .await
                .map_err(|e| ferrotunnel_common::TunnelError::Protocol(e.to_string()))?;
            Ok(())
        })
    }
}

/// Receives frames from TCP via a framed read stream.
pub struct TcpFrameReceiver<R> {
    stream: FramedRead<R, TunnelCodec>,
}

impl<R> TcpFrameReceiver<R>
where
    R: AsyncRead + Unpin + Send,
{
    pub fn new(stream: FramedRead<R, TunnelCodec>) -> Self {
        Self { stream }
    }
}

impl<R> FrameReceiver for TcpFrameReceiver<R>
where
    R: AsyncRead + Unpin + Send,
{
    fn recv_frame(&mut self) -> Pin<Box<dyn Future<Output = Result<Option<Frame>>> + Send + '_>> {
        let stream = &mut self.stream;
        Box::pin(async move {
            match stream.next().await {
                Some(Ok(frame)) => Ok(Some(frame)),
                Some(Err(e)) => Err(ferrotunnel_common::TunnelError::Io(e)),
                None => Ok(None),
            }
        })
    }
}
