//! Batched frame sender for reduced syscall overhead
//!
//! Collects multiple frames and flushes them together in a single operation.

use ferrotunnel_protocol::Frame;
use futures::sink::Sink;
use futures::SinkExt;
use kanal::AsyncReceiver;
use std::time::Duration;
use tokio::time::timeout;
use tracing::warn;

const MAX_BATCH_SIZE: usize = 32;
const BATCH_TIMEOUT_MICROS: u64 = 100;

/// Spawns a batched sender task that collects frames and flushes them together
///
/// This reduces syscall overhead by batching multiple small frames into a single
/// flush operation. Uses `feed` to queue frames without flushing, then flushes once.
pub async fn run_batched_sender<S, E>(frame_rx: AsyncReceiver<Frame>, mut sink: S)
where
    S: Sink<Frame, Error = E> + Unpin + Send + 'static,
    E: std::fmt::Display,
{
    loop {
        // Wait for first frame
        let Ok(first) = frame_rx.recv().await else {
            break;
        };

        // Feed first frame without flushing
        if let Err(e) = sink.feed(first).await {
            warn!("Failed to feed frame: {}", e);
            break;
        }

        // Try to collect more frames with a short timeout
        let deadline = Duration::from_micros(BATCH_TIMEOUT_MICROS);
        let mut batch_count = 1;

        while batch_count < MAX_BATCH_SIZE {
            match timeout(deadline, frame_rx.recv()).await {
                Ok(Ok(frame)) => {
                    if let Err(e) = sink.feed(frame).await {
                        warn!("Failed to feed frame: {}", e);
                        return;
                    }
                    batch_count += 1;
                }
                Ok(Err(_)) | Err(_) => break,
            }
        }

        // Single flush for all batched frames
        if let Err(e) = sink.flush().await {
            warn!("Failed to flush frames: {}", e);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use ferrotunnel_protocol::frame::DataFrame;
    use ferrotunnel_protocol::TunnelCodec;
    use kanal::bounded_async;
    use tokio::io::duplex;
    use tokio_util::codec::FramedWrite;

    #[tokio::test]
    async fn test_batched_sender_single_frame() {
        let (tx, rx) = bounded_async::<Frame>(10);
        let (writer, _reader) = duplex(8192);
        let framed = FramedWrite::new(writer, TunnelCodec::new());

        tx.send(Frame::Heartbeat { timestamp: 123 }).await.unwrap();
        drop(tx);

        run_batched_sender(rx, framed).await;
    }

    #[tokio::test]
    async fn test_batched_sender_multiple_frames() {
        let (tx, rx) = bounded_async::<Frame>(10);
        let (writer, _reader) = duplex(8192);
        let framed = FramedWrite::new(writer, TunnelCodec::new());

        for i in 0..5 {
            tx.send(Frame::Heartbeat { timestamp: i }).await.unwrap();
        }
        drop(tx);

        run_batched_sender(rx, framed).await;
    }

    #[tokio::test]
    async fn test_batched_sender_data_frames() {
        let (tx, rx) = bounded_async::<Frame>(10);
        let (writer, _reader) = duplex(65536);
        let framed = FramedWrite::new(writer, TunnelCodec::new());

        for i in 0..3 {
            tx.send(Frame::Data(Box::new(DataFrame {
                stream_id: i,
                data: Bytes::from("test data"),
                end_of_stream: false,
            })))
            .await
            .unwrap();
        }
        drop(tx);

        run_batched_sender(rx, framed).await;
    }
}
