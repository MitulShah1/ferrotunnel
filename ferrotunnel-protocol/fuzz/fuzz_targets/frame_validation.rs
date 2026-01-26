#![no_main]

use bytes::BytesMut;
use ferrotunnel_protocol::codec::TunnelCodec;
use ferrotunnel_protocol::validation::{validate_frame, ValidationLimits};
use libfuzzer_sys::fuzz_target;
use tokio_util::codec::Decoder;

fuzz_target!(|data: &[u8]| {
    let mut codec = TunnelCodec::with_max_frame_size(1024 * 1024);
    let mut buf = BytesMut::from(data);

    // If we can decode a frame, validation should not panic
    if let Ok(Some(frame)) = codec.decode(&mut buf) {
        let limits = ValidationLimits::default();
        let _ = validate_frame(&frame, &limits);
    }
});
