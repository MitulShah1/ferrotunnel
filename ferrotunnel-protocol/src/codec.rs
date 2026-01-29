//! Codec for encoding and decoding protocol frames
//!
//! Uses zero-copy techniques and thread-local buffers for efficiency.

use crate::constants::MAX_FRAME_SIZE;
use crate::frame::Frame;
use bytes::{Buf, BufMut, BytesMut};
use std::cell::RefCell;
use std::io;
use tokio_util::codec::{Decoder, Encoder};

thread_local! {
    static ENCODE_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

const INITIAL_ENCODE_BUFFER_CAPACITY: usize = 8192;

/// Tunnel protocol codec
///
/// Frames are length-prefixed with a 4-byte big-endian length field,
/// followed by bincode-encoded Frame data.
///
/// Frame format:
/// ```text
/// ┌─────────────┬──────────────┐
/// │ Length (u32)│ Frame Data   │
/// │ 4 bytes     │ N bytes      │
/// └─────────────┴──────────────┘
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TunnelCodec {
    max_frame_size: usize,
}

impl Default for TunnelCodec {
    fn default() -> Self {
        Self {
            max_frame_size: MAX_FRAME_SIZE as usize,
        }
    }
}

impl TunnelCodec {
    /// Create a new codec instance with default max frame size
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new codec instance with a custom max frame size
    pub fn with_max_frame_size(max_frame_size: usize) -> Self {
        Self { max_frame_size }
    }

    /// Get the configured max frame size
    pub fn max_frame_size(&self) -> usize {
        self.max_frame_size
    }
}

impl Decoder for TunnelCodec {
    type Item = Frame;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least 4 bytes for length prefix
        if src.len() < 4 {
            return Ok(None);
        }

        // Read length prefix (don't consume yet)
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&src[..4]);
        let frame_length = u32::from_be_bytes(length_bytes) as usize;

        // Validate frame size
        if frame_length > self.max_frame_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Frame too large: {frame_length} bytes (max: {})",
                    self.max_frame_size
                ),
            ));
        }

        // Check if we have the full frame
        if src.len() < 4 + frame_length {
            // Reserve space for the full frame
            src.reserve(4 + frame_length - src.len());
            return Ok(None);
        }

        // Skip the length prefix
        src.advance(4);

        let frame_bytes = src.split_to(frame_length).freeze();

        let frame = bincode::deserialize(&frame_bytes).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Decode error: {e}"))
        })?;

        Ok(Some(frame))
    }
}

impl Encoder<Frame> for TunnelCodec {
    type Error = io::Error;

    fn encode(&mut self, frame: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        ENCODE_BUFFER.with(|buf| {
            let mut buf = buf.borrow_mut();

            if buf.capacity() == 0 {
                buf.reserve(INITIAL_ENCODE_BUFFER_CAPACITY);
            }
            buf.clear();

            // Serialize into reusable buffer
            bincode::serialize_into(&mut *buf, &frame).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("Encode error: {e}"))
            })?;

            let frame_length = buf.len();

            // Validate frame size
            if frame_length > self.max_frame_size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Frame too large: {frame_length} bytes (max: {})",
                        self.max_frame_size
                    ),
                ));
            }

            // Reserve space for length prefix + data
            dst.reserve(4 + frame_length);

            // Write length prefix
            #[allow(clippy::cast_possible_truncation, clippy::expect_used)]
            dst.put_u32(u32::try_from(frame_length).expect("Frame length validated"));

            // Write frame data
            dst.put_slice(&buf);

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::DataFrame;
    use bytes::Bytes;

    #[test]
    fn test_codec_round_trip() {
        let mut codec = TunnelCodec::new();
        let mut buf = BytesMut::new();

        let frame = Frame::Heartbeat { timestamp: 12345 };

        // Encode
        codec.encode(frame.clone(), &mut buf).unwrap();

        // Decode
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        assert_eq!(frame, decoded);
    }

    #[test]
    fn test_partial_frame() {
        let mut codec = TunnelCodec::new();
        let mut buf = BytesMut::new();

        let frame = Frame::Data(Box::new(DataFrame {
            stream_id: 1,
            data: Bytes::from("hello world"),
            end_of_stream: false,
        }));

        // Encode
        codec.encode(frame, &mut buf).unwrap();

        // Split the buffer
        let full_len = buf.len();
        let mut partial = buf.split_to(full_len / 2);

        // Try to decode partial frame
        let result = codec.decode(&mut partial);
        assert!(result.unwrap().is_none());

        // Unsplit and decode
        partial.unsplit(buf);
        let decoded = codec.decode(&mut partial).unwrap();
        assert!(decoded.is_some());
    }

    #[test]
    fn test_multiple_frames() {
        let mut codec = TunnelCodec::new();
        let mut buf = BytesMut::new();

        let frames = vec![
            Frame::Heartbeat { timestamp: 1 },
            Frame::Heartbeat { timestamp: 2 },
            Frame::Heartbeat { timestamp: 3 },
        ];

        // Encode all frames
        for frame in &frames {
            codec.encode(frame.clone(), &mut buf).unwrap();
        }

        // Decode all frames
        for expected in &frames {
            let decoded = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(*expected, decoded);
        }

        // Buffer should be empty
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_max_frame_size() {
        let mut codec = TunnelCodec::new();
        let mut buf = BytesMut::new();

        // Create a frame that's too large
        let large_data = vec![0u8; (MAX_FRAME_SIZE + 1) as usize];
        let frame = Frame::Data(Box::new(DataFrame {
            stream_id: 1,
            data: Bytes::from(large_data),
            end_of_stream: false,
        }));

        // Encoding should fail
        let result = codec.encode(frame, &mut buf);
        assert!(result.is_err());
    }
}
