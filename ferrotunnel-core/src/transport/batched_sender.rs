//! Batched frame sender for reduced syscall overhead
//!
//! Collects multiple frames and flushes them together in a single operation.

use ferrotunnel_protocol::codec::TunnelCodec;
use ferrotunnel_protocol::Frame;
use kanal::AsyncReceiver;
use std::io::IoSlice;
use std::time::Duration;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::time::timeout;
use tokio_util::codec::Encoder; // Import Encoder trait
use tracing::warn;

const MAX_BATCH_SIZE: usize = 32;
const BATCH_TIMEOUT_MICROS: u64 = 100;

/// Spawns a batched sender task that collects frames and flushes them together using vectored I/O.
///
/// This reduces syscall overhead and eliminates data checking by writing headers and payloads
/// directly to the socket gathering write.
pub async fn run_batched_sender<W>(
    frame_rx: AsyncReceiver<Frame>,
    mut writer: W,
    mut codec: TunnelCodec,
) where
    W: AsyncWrite + Unpin + Send + 'static,
{
    // Buffer for holding frames to keep them alive during write
    let mut frames = Vec::with_capacity(MAX_BATCH_SIZE);

    // Buffer for holding encoded headers and control frames.
    // Each data frame needs 10 bytes of header.
    let mut header_buffer = Vec::with_capacity(MAX_BATCH_SIZE * 64);

    loop {
        // Clear buffers from previous iteration
        frames.clear();
        header_buffer.clear();

        // Vector of IO slices for writev
        // Defined inside loop to avoid self-referential borrow issues across iterations
        let mut iovecs = Vec::with_capacity(MAX_BATCH_SIZE * 2);

        // 1. Wait for first frame
        if let Ok(frame) = frame_rx.recv().await {
            frames.push(frame);
        } else {
            break; // Channel closed
        }

        // 2. Collect more frames with a short timeout
        let deadline = Duration::from_micros(BATCH_TIMEOUT_MICROS);
        let start = std::time::Instant::now();

        while frames.len() < MAX_BATCH_SIZE {
            let elapsed = start.elapsed();
            if elapsed >= deadline {
                break;
            }
            let remaining = deadline.checked_sub(elapsed).unwrap_or(Duration::ZERO);

            match timeout(remaining, frame_rx.recv()).await {
                Ok(Ok(frame)) => frames.push(frame),
                _ => break, // Timeout or empty
            }
        }

        // 3. First Pass: Encode headers and control frames into header_buffer
        // We accumulate (Range, Option<DataSlice>) to construct IoSlices later.
        // This is needed because extending header_buffer invalidates previous slices into it.
        let mut parts = Vec::with_capacity(frames.len());

        for frame in &frames {
            match frame {
                Frame::Data {
                    stream_id,
                    data,
                    end_of_stream,
                    ..
                } => {
                    let start = header_buffer.len();
                    header_buffer.extend_from_slice(&[0u8; 10]); // Reserve 10 bytes

                    if let Err(e) = codec.encode_data_header(
                        &mut header_buffer[start..],
                        *stream_id,
                        data.len(),
                        *end_of_stream,
                    ) {
                        warn!("Skipping invalid data frame header: {}", e);
                        header_buffer.truncate(start);
                        continue;
                    }
                    parts.push((start..header_buffer.len(), Some(data)));
                }
                control_frame => {
                    let start = header_buffer.len();
                    // Encode control frame (slow path)
                    let mut buf = bytes::BytesMut::new();
                    if let Err(e) = codec.encode(control_frame.clone(), &mut buf) {
                        warn!("Skipping invalid control frame: {}", e);
                        continue;
                    }
                    header_buffer.extend_from_slice(&buf);
                    parts.push((start..header_buffer.len(), None));
                }
            }
        }

        // 4. Second Pass: Construct iovecs
        // Now that header_buffer is stable, we can create slices into it.
        for (range, data_opt) in parts {
            iovecs.push(IoSlice::new(&header_buffer[range]));
            if let Some(data) = data_opt {
                iovecs.push(IoSlice::new(data));
            }
        }

        // 5. Flush using vectored write
        if !iovecs.is_empty() {
            if let Err(e) = writer.write_vectored(&iovecs).await {
                warn!("Failed to write batched frames: {}", e);
                break;
            }

            if let Err(e) = writer.flush().await {
                warn!("Failed to flush batched writer: {}", e);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use ferrotunnel_protocol::codec::TunnelCodec;
    use kanal::bounded_async;
    use tokio::io::duplex;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn test_batched_sender_single_frame() {
        let (tx, rx) = bounded_async::<Frame>(10);
        let (writer, mut reader) = duplex(8192);

        tokio::spawn(async move {
            run_batched_sender(rx, writer, TunnelCodec::new()).await;
        });

        tx.send(Frame::Heartbeat { timestamp: 123 }).await.unwrap();
        drop(tx);

        // Verify we can read something (basic check)
        let mut buf = [0u8; 100];
        let n = reader.read(&mut buf).await.unwrap();
        assert!(n > 0);
    }

    #[tokio::test]
    async fn test_batched_sender_multiple_frames() {
        let (tx, rx) = bounded_async::<Frame>(10);
        let (writer, mut reader) = duplex(8192);

        tokio::spawn(async move {
            run_batched_sender(rx, writer, TunnelCodec::new()).await;
        });

        for i in 0..5 {
            tx.send(Frame::Heartbeat { timestamp: i }).await.unwrap();
        }
        drop(tx);

        // Just verify connection doesn't drop immediately and we get data
        let mut buf = [0u8; 1024];
        let n = reader.read(&mut buf).await.unwrap();
        assert!(n > 0);
    }

    #[tokio::test]
    async fn test_batched_sender_data_frames() {
        let (tx, rx) = bounded_async::<Frame>(10);
        let (writer, mut reader) = duplex(65536);

        tokio::spawn(async move {
            run_batched_sender(rx, writer, TunnelCodec::new()).await;
        });

        for i in 0..3 {
            tx.send(Frame::Data {
                stream_id: i,
                data: Bytes::from("test data"),
                end_of_stream: false,
            })
            .await
            .unwrap();
        }
        drop(tx);

        let mut buf = [0u8; 1024];
        let n = reader.read(&mut buf).await.unwrap();
        assert!(n > 0);
    }
}
