//! Codec for encoding and decoding protocol frames
//!
//! Uses simple length-delimited framing for maximum performance:
//! - 4-byte length prefix (u32 big-endian)
//! - 1-byte type discriminator
//! - Variable payload
//!
//! This is similar to Rathole's approach and avoids COBS overhead.

use crate::constants::MAX_FRAME_SIZE;
use crate::frame::Frame;
use bytes::{Buf, BufMut, BytesMut};
use std::io;
use tokio_util::codec::{Decoder, Encoder};

/// Frame header size: 4 bytes length + 1 byte type
const HEADER_SIZE: usize = 5;

const FRAME_TYPE_CONTROL: u8 = 0x00;
const FRAME_TYPE_DATA: u8 = 0x01;
const FLAG_EOS: u8 = 0x01;

/// Tunnel protocol codec using length-delimited framing
///
/// Wire format:
/// ```text
/// ┌─────────────┬───────────┬──────────────┐
/// │ Length (u32)│ Type (u8) │ Payload      │
/// │ 4 bytes BE  │ 1 byte    │ N bytes      │
/// └─────────────┴───────────┴──────────────┘
/// ```
///
/// Length includes Type + Payload (not the length field itself).
///
/// Payload format depends on Type:
/// - Control (0x00): `bincode(Frame)` (excluding `Frame::Data`)
/// - Data (0x01): `[StreamID(u32)][Flags(u8)][Raw Bytes...]`
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
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new codec instance with a custom max frame size
    #[inline]
    pub fn with_max_frame_size(max_frame_size: usize) -> Self {
        Self { max_frame_size }
    }

    /// Get the configured max frame size
    #[inline]
    pub fn max_frame_size(&self) -> usize {
        self.max_frame_size
    }
}

impl Decoder for TunnelCodec {
    type Item = Frame;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least header (4 bytes length + 1 byte type)
        if src.len() < HEADER_SIZE {
            src.reserve(HEADER_SIZE - src.len());
            return Ok(None);
        }

        // Peek at length (don't consume yet)
        let length = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        // Validate frame size
        if length == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Frame length must be at least 1 byte",
            ));
        }
        if length > self.max_frame_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Frame too large: {} bytes (max: {})",
                    length, self.max_frame_size
                ),
            ));
        }

        // Total frame size = 4 (length field) + length (type + payload)
        let total_size = 4 + length;
        if src.len() < total_size {
            // Not enough data yet, reserve space
            src.reserve(total_size - src.len());
            return Ok(None);
        }

        // Consume the frame
        let mut frame_bytes = src.split_to(total_size).freeze();
        frame_bytes.advance(4);

        // Parse type and payload
        let frame_type = frame_bytes.get_u8();

        match frame_type {
            FRAME_TYPE_DATA => {
                // Data frame: [StreamID(u32)][Flags(u8)][Raw Bytes...]
                if frame_bytes.remaining() < 5 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Data frame payload too short",
                    ));
                }
                let stream_id = frame_bytes.get_u32();
                let flags = frame_bytes.get_u8();
                let end_of_stream = (flags & FLAG_EOS) != 0;
                // Zero-copy slice of the remaining payload.
                let data = frame_bytes.split_to(frame_bytes.remaining());

                Ok(Some(Frame::Data {
                    stream_id,
                    data,
                    end_of_stream,
                }))
            }
            FRAME_TYPE_CONTROL => {
                // Control frame: bincode-encoded Frame
                let config =
                    bincode_next::config::standard().with_limit::<{ MAX_FRAME_SIZE as usize }>();
                let (frame, _) =
                    bincode_next::serde::decode_from_slice(frame_bytes.as_ref(), config).map_err(
                        |e| {
                            io::Error::new(io::ErrorKind::InvalidData, format!("Decode error: {e}"))
                        },
                    )?;
                Ok(Some(frame))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown frame type: {frame_type}"),
            )),
        }
    }
}

impl Encoder<Frame> for TunnelCodec {
    type Error = io::Error;

    fn encode(&mut self, frame: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match frame {
            Frame::Data {
                stream_id,
                data,
                end_of_stream,
            } => {
                // Data frame: [Length][Type][StreamID][Flags][Data]
                // Payload = type(1) + stream_id(4) + flags(1) + data.len()
                let payload_len = 1 + 4 + 1 + data.len();
                if payload_len > self.max_frame_size {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Frame too large: {} bytes (max: {})",
                            payload_len, self.max_frame_size
                        ),
                    ));
                }

                // Reserve space for entire frame
                dst.reserve(4 + payload_len);

                // Write length (type + payload, not including length field)
                dst.put_u32(payload_len as u32);
                // Write type
                dst.put_u8(FRAME_TYPE_DATA);
                // Write stream_id
                dst.put_u32(stream_id);
                // Write flags
                dst.put_u8(if end_of_stream { FLAG_EOS } else { 0 });
                // Write data directly - no copy needed if data is contiguous
                dst.extend_from_slice(&data);
            }
            control_frame => {
                // Control frame: [Length][Type][bincode payload]
                // First serialize the frame
                let config = bincode_next::config::standard();
                let serialized = bincode_next::serde::encode_to_vec(&control_frame, config)
                    .map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("Encode error: {e}"))
                    })?;

                let payload_len = 1 + serialized.len(); // type + serialized
                if payload_len > self.max_frame_size {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Frame too large: {} bytes (max: {})",
                            payload_len, self.max_frame_size
                        ),
                    ));
                }

                // Reserve and write
                dst.reserve(4 + payload_len);
                dst.put_u32(payload_len as u32);
                dst.put_u8(FRAME_TYPE_CONTROL);
                dst.extend_from_slice(&serialized);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_data_frame_round_trip() {
        let mut codec = TunnelCodec::new();
        let mut buf = BytesMut::new();

        let frame = Frame::Data {
            stream_id: 42,
            data: Bytes::from("hello world"),
            end_of_stream: true,
        };

        codec.encode(frame.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        assert_eq!(frame, decoded);
    }

    #[test]
    fn test_partial_frame() {
        let mut codec = TunnelCodec::new();
        let mut buf = BytesMut::new();

        let frame = Frame::Data {
            stream_id: 1,
            data: Bytes::from("hello world"),
            end_of_stream: false,
        };

        // Encode
        codec.encode(frame, &mut buf).unwrap();

        // Split the buffer
        let full_len = buf.len();
        let mut partial = buf.split_to(full_len / 2);

        // Try to decode partial frame - should return None
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
        let frame = Frame::Data {
            stream_id: 1,
            data: Bytes::from(large_data),
            end_of_stream: false,
        };

        // Encoding should fail
        let result = codec.encode(frame, &mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_large_data_frame() {
        let mut codec = TunnelCodec::new();
        let mut buf = BytesMut::new();

        // Test with a reasonably large frame (16KB)
        let data = vec![0xAB; 16 * 1024];
        let frame = Frame::Data {
            stream_id: 999,
            data: Bytes::from(data.clone()),
            end_of_stream: false,
        };

        codec.encode(frame.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        if let Frame::Data {
            stream_id,
            data: decoded_data,
            end_of_stream,
        } = decoded
        {
            assert_eq!(stream_id, 999);
            assert_eq!(decoded_data.as_ref(), data.as_slice());
            assert!(!end_of_stream);
        } else {
            panic!("Expected Data frame");
        }
    }

    #[test]
    fn test_frame_size_validation_on_decode() {
        let mut codec = TunnelCodec::with_max_frame_size(100);
        let mut buf = BytesMut::new();

        // Craft a frame with invalid large length
        buf.put_u32(1000); // Length claims 1000 bytes
        buf.put_u8(FRAME_TYPE_DATA);
        buf.extend_from_slice(&[0u8; 10]); // Only 10 bytes of payload

        // Should fail validation before trying to read more
        let result = codec.decode(&mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_length_frame_rejected() {
        let mut codec = TunnelCodec::new();
        let mut buf = BytesMut::new();

        buf.put_u32(0); // Invalid length
        buf.put_u8(FRAME_TYPE_CONTROL);
        let result = codec.decode(&mut buf);
        assert!(result.is_err());
    }
}
