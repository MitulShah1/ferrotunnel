#![no_main]

use bytes::BytesMut;
use ferrotunnel_protocol::codec::TunnelCodec;
use libfuzzer_sys::fuzz_target;
use tokio_util::codec::Decoder;

fuzz_target!(|data: &[u8]| {
    // Create codec with reasonable limits
    let mut codec = TunnelCodec::with_max_frame_size(1024 * 1024);
    let mut buf = BytesMut::from(data);

    // Decode should never panic, regardless of input
    // It may return Ok(None), Ok(Some(frame)), or Err(_)
    let _ = codec.decode(&mut buf);
});
