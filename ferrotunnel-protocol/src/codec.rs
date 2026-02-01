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

const FRAME_TYPE_CONTROL: u8 = 0x00;
const FRAME_TYPE_DATA: u8 = 0x01;
const FLAG_EOS: u8 = 0x01;

/// Tunnel protocol codec
///
/// Frames are length-prefixed with a 4-byte big-endian length field,
/// followed by a 1-byte frame type.
///
/// Frame format:
/// ```text
/// ┌─────────────┬───────────┬──────────────┐
/// │ Length (u32)│ Type (u8) │ Payload      │
/// │ 4 bytes     │ 1 byte    │ N bytes      │
/// └─────────────┴───────────┴──────────────┘
/// ```
///
/// Payload format depends on Type:
/// - Control (0x00): `bincode(Frame)` (excluding `Frame::Data`)
/// - Data (0x01): `[StreamID(u32)][Flags(u8)][Raw Bytes...]`
///
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

    /// Encode just the header for a data frame into a buffer.
    /// Used for vectored I/O (writev) to avoid copying the payload.
    /// Buffer must be at least 10 bytes.
    pub fn encode_data_header(
        &self,
        dst: &mut [u8],
        stream_id: u32,
        payload_len: usize,
        end_of_stream: bool,
    ) -> io::Result<()> {
        if dst.len() < 10 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "Destination buffer too small for data header",
            ));
        }

        // Total Length = 1 (Type) + 4 (StreamID) + 1 (Flags) + payload_len
        let total_len = 1 + 4 + 1 + payload_len;

        if total_len > self.max_frame_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Frame too large: {total_len} bytes (max: {})",
                    self.max_frame_size
                ),
            ));
        }

        // 1. Length Prefix (u32 big endian)
        let len_bytes = u32::try_from(total_len)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Frame too large"))?
            .to_be_bytes();
        dst[0..4].copy_from_slice(&len_bytes);

        // 2. Type (u8)
        dst[4] = FRAME_TYPE_DATA;

        // 3. Stream ID (u32 big endian)
        let stream_id_bytes = stream_id.to_be_bytes();
        dst[5..9].copy_from_slice(&stream_id_bytes);

        // 4. Flags (u8)
        dst[9] = if end_of_stream { FLAG_EOS } else { 0 };

        Ok(())
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

        // Ensure we have at least 1 byte for Type
        if frame_length < 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Frame too short (missing type byte)",
            ));
        }

        let frame_type = src[0];
        src.advance(1); // Consume type byte
        let payload_len = frame_length - 1;

        match frame_type {
            FRAME_TYPE_DATA => {
                // Fast Path: Data Frame
                // Format: [StreamID: 4][Flags: 1][Data: N]
                if payload_len < 5 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Data frame too short header",
                    ));
                }

                let stream_id = src.get_u32();
                let flags = src.get_u8();
                let end_of_stream = (flags & FLAG_EOS) != 0;

                // The rest is data. We take a slice of it.
                // We consumed 5 bytes (4 stream_id + 1 flags) from payload
                let data_len = payload_len - 5;
                let data = src.split_to(data_len).freeze();

                Ok(Some(Frame::Data {
                    stream_id,
                    data,
                    end_of_stream,
                }))
            }
            FRAME_TYPE_CONTROL => {
                // Slow Path: Control Frame (Serde)
                let frame_bytes = src.split_to(payload_len); // Consume entire payload
                let config =
                    bincode_next::config::standard().with_limit::<{ MAX_FRAME_SIZE as usize }>();

                let (frame, _) = bincode_next::serde::decode_from_slice(&frame_bytes, config)
                    .map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("Decode error: {e}"))
                    })?;

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
                // Fast Path Encoding
                // Payload: [Type(1)][StreamID(4)][Flags(1)][Data(N)]
                // Total Length = 1 + 4 + 1 + data.len()
                let payload_len = 1 + 4 + 1 + data.len();

                if payload_len > self.max_frame_size {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "Frame too large: {payload_len} bytes (max: {})",
                            self.max_frame_size
                        ),
                    ));
                }

                dst.reserve(4 + payload_len);
                #[allow(clippy::cast_possible_truncation)]
                dst.put_u32(payload_len as u32);
                dst.put_u8(FRAME_TYPE_DATA); // Type
                dst.put_u32(stream_id);

                let flags = if end_of_stream { FLAG_EOS } else { 0 };
                dst.put_u8(flags);

                dst.put_slice(&data);
                Ok(())
            }
            control_frame => {
                // Slow Path Encoding (buffer reused)
                ENCODE_BUFFER.with(|buf| {
                    let mut buf = buf.borrow_mut();

                    if buf.capacity() == 0 {
                        buf.reserve(INITIAL_ENCODE_BUFFER_CAPACITY);
                    }
                    buf.clear();

                    // Serialize Frame into buffer (Payload)
                    let config = bincode_next::config::standard();
                    bincode_next::serde::encode_into_std_write(&control_frame, &mut *buf, config)
                        .map_err(|e| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("Encode error: {e}"))
                    })?;

                    // Length = 1 (Type) + Serialized Len
                    let serialized_len = buf.len();
                    let total_len = 1 + serialized_len;

                    if total_len > self.max_frame_size {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "Frame too large: {total_len} bytes (max: {})",
                                self.max_frame_size
                            ),
                        ));
                    }

                    dst.reserve(4 + total_len);
                    #[allow(clippy::cast_possible_truncation)]
                    dst.put_u32(total_len as u32);
                    dst.put_u8(FRAME_TYPE_CONTROL); // Type
                    dst.put_slice(&buf);

                    Ok(())
                })
            }
        }
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
        let frame = Frame::Data {
            stream_id: 1,
            data: Bytes::from(large_data),
            end_of_stream: false,
        };

        // Encoding should fail
        let result = codec.encode(frame, &mut buf);
        assert!(result.is_err());
    }
}
