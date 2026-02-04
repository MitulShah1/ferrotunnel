//! Benchmarks for FerroTunnel protocol codec

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::pedantic)]
#![allow(unused_imports, unused_variables)]

use bytes::{Bytes, BytesMut};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ferrotunnel_protocol::codec::TunnelCodec;
use ferrotunnel_protocol::frame::{
    Frame, HandshakeFrame, HandshakeStatus, OpenStreamFrame, Protocol, StreamPriority,
};
use std::collections::HashMap;
use tokio_util::codec::{Decoder, Encoder};
use uuid::Uuid;

fn create_test_frames() -> Vec<(&'static str, Frame)> {
    vec![
        (
            "handshake",
            Frame::Handshake(Box::new(HandshakeFrame {
                min_version: 1,
                max_version: 1,
                token: "test-token-12345678901234567890".to_string(),
                tunnel_id: Some("benchmark-tunnel".to_string()),
                capabilities: vec!["basic".to_string(), "tls".to_string()],
            })),
        ),
        (
            "handshake_ack",
            Frame::HandshakeAck {
                status: HandshakeStatus::Success,
                session_id: Uuid::new_v4(),
                version: 1,
                server_capabilities: vec!["basic".to_string()],
            },
        ),
        (
            "heartbeat",
            Frame::Heartbeat {
                timestamp: 1_234_567_890,
            },
        ),
        (
            "open_stream",
            Frame::OpenStream(Box::new(OpenStreamFrame {
                stream_id: 12345,
                protocol: Protocol::HTTP,
                headers: vec![
                    ("Host".to_string(), "example.com".to_string()),
                    ("Content-Type".to_string(), "application/json".to_string()),
                ],
                body_hint: Some(1024),
                priority: StreamPriority::default(),
            })),
        ),
        (
            "data_small",
            Frame::Data {
                stream_id: 12345,
                data: Bytes::from(vec![0u8; 64]),
                end_of_stream: false,
            },
        ),
        (
            "data_medium",
            Frame::Data {
                stream_id: 12345,
                data: Bytes::from(vec![0u8; 1024]),
                end_of_stream: false,
            },
        ),
        (
            "data_large",
            Frame::Data {
                stream_id: 12345,
                data: Bytes::from(vec![0u8; 65536]),
                end_of_stream: true,
            },
        ),
        (
            "register",
            Frame::Register {
                service_name: "my-web-service".to_string(),
                protocol: Protocol::HTTP,
                metadata: HashMap::from([
                    ("version".to_string(), "1.0".to_string()),
                    ("env".to_string(), "production".to_string()),
                ]),
            },
        ),
    ]
}

fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode");
    let frames = create_test_frames();

    for (name, frame) in &frames {
        let payload_size = match frame {
            Frame::Data { data, .. } => data.len(),
            _ => 0,
        };

        if payload_size > 0 {
            group.throughput(Throughput::Bytes(payload_size as u64));
        }

        group.bench_with_input(BenchmarkId::new("frame", name), frame, |b, frame| {
            let mut codec = TunnelCodec::new();
            let mut buf = BytesMut::with_capacity(65536);

            b.iter(|| {
                buf.clear();
                codec
                    .encode(black_box(frame.clone()), &mut buf)
                    .expect("encode failed");
                black_box(&buf);
            });
        });
    }

    group.finish();
}

fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");
    let frames = create_test_frames();

    for (name, frame) in &frames {
        let mut codec = TunnelCodec::new();
        let mut encoded = BytesMut::with_capacity(65536);
        codec
            .encode(frame.clone(), &mut encoded)
            .expect("encode failed");
        let encoded_bytes = encoded.freeze();

        let payload_size = match frame {
            Frame::Data { data, .. } => data.len(),
            _ => 0,
        };

        if payload_size > 0 {
            group.throughput(Throughput::Bytes(payload_size as u64));
        }

        group.bench_with_input(
            BenchmarkId::new("frame", name),
            &encoded_bytes,
            |b, encoded| {
                let mut codec = TunnelCodec::new();

                b.iter(|| {
                    let mut buf = BytesMut::from(&encoded[..]);
                    let result = codec.decode(&mut buf).expect("decode failed");
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    let payload_sizes = [64, 256, 1024, 4096, 16384, 65536];

    for size in payload_sizes {
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("data_frame", size), &size, |b, &size| {
            let frame = Frame::Data {
                stream_id: 1,
                data: Bytes::from(vec![0u8; size]),
                end_of_stream: false,
            };
            let mut codec = TunnelCodec::new();
            let mut buf = BytesMut::with_capacity(size + 128);

            b.iter(|| {
                buf.clear();
                codec
                    .encode(black_box(frame.clone()), &mut buf)
                    .expect("encode failed");
                let decoded = codec.decode(&mut buf).expect("decode failed");
                black_box(decoded);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_encode, bench_decode, bench_roundtrip);
criterion_main!(benches);
