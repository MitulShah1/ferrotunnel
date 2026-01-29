//! Throughput Benchmarks
//!
//! Measures raw data throughput for various payload sizes.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ferrotunnel_protocol::frame::DataFrame;
use ferrotunnel_protocol::Frame;

fn encode_frame(frame: &Frame) -> Vec<u8> {
    let config = bincode_next::config::standard();
    bincode_next::serde::encode_to_vec(frame, config).unwrap()
}

fn decode_frame(data: &[u8]) -> Frame {
    let config = bincode_next::config::standard();
    let (decoded, _): (Frame, usize) =
        bincode_next::serde::decode_from_slice(data, config).unwrap();
    decoded
}

/// Benchmark encoding throughput for various payload sizes
fn bench_encode_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_throughput");

    // Test various payload sizes
    for size in [64, 256, 1024, 4096, 16384, 65536].iter() {
        let payload = vec![0xABu8; *size];

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let frame = Frame::Data(Box::new(DataFrame {
                stream_id: 1,
                data: payload.clone().into(),
                end_of_stream: false,
            }));

            b.iter(|| encode_frame(&frame));
        });
    }

    group.finish();
}

/// Benchmark decoding throughput for various payload sizes
fn bench_decode_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_throughput");

    for size in [64, 256, 1024, 4096, 16384, 65536].iter() {
        let payload = vec![0xABu8; *size];
        let frame = Frame::Data(Box::new(DataFrame {
            stream_id: 1,
            data: payload.into(),
            end_of_stream: false,
        }));
        let encoded = encode_frame(&frame);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| decode_frame(&encoded));
        });
    }

    group.finish();
}

/// Benchmark roundtrip (encode + decode)
fn bench_roundtrip_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip_throughput");

    for size in [1024, 4096, 16384, 65536].iter() {
        let payload = vec![0xABu8; *size];

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let frame = Frame::Data(Box::new(DataFrame {
                stream_id: 1,
                data: payload.clone().into(),
                end_of_stream: false,
            }));

            b.iter(|| {
                let encoded = encode_frame(&frame);
                decode_frame(&encoded)
            });
        });
    }

    group.finish();
}

/// Benchmark batch encoding (multiple frames)
fn bench_batch_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_encoding");

    for batch_size in [10, 50, 100].iter() {
        let frames: Vec<Frame> = (0..*batch_size)
            .map(|i| {
                Frame::Data(Box::new(DataFrame {
                    stream_id: i as u32,
                    data: vec![0u8; 256].into(),
                    end_of_stream: false,
                }))
            })
            .collect();

        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, _| {
                b.iter(|| {
                    frames
                        .iter()
                        .map(|f| encode_frame(f))
                        .collect::<Vec<_>>()
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_encode_throughput,
    bench_decode_throughput,
    bench_roundtrip_throughput,
    bench_batch_encoding
);
criterion_main!(benches);
