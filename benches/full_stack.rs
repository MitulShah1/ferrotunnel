#![allow(clippy::unwrap_used)]
#![allow(clippy::cast_possible_truncation)]

//! Full-stack end-to-end benchmarks
//!
//! Measures complete tunnel performance including:
//! - Protocol encoding/decoding
//! - Multiplexing
//! - Network I/O
//! - Batching

use bytes::Bytes;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ferrotunnel_protocol::{Frame, TunnelCodec};
use futures_util::{SinkExt, StreamExt};
use kanal::bounded_async;
use std::time::Duration;
use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Runtime;
use tokio_util::codec::{FramedRead, FramedWrite};

/// Benchmark frame encoding throughput
fn bench_frame_encoding(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("frame_encoding");

    for size in [1024, 4096, 16384, 65536] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let data = Bytes::from(vec![0u8; size]);
                let frame = Frame::Data {
                    stream_id: 1,
                    data,
                    end_of_stream: false,
                };

                let (mut writer, _reader) = duplex(size * 2);
                let codec = TunnelCodec::new();
                let mut framed = FramedWrite::new(&mut writer, codec);

                framed.send(frame).await.unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark frame decoding throughput
fn bench_frame_decoding(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("frame_decoding");

    for size in [1024, 4096, 16384, 65536] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            // Pre-encode a frame
            let encoded = rt.block_on(async {
                let data = Bytes::from(vec![0u8; size]);
                let frame = Frame::Data {
                    stream_id: 1,
                    data,
                    end_of_stream: false,
                };

                let (mut writer, reader) = duplex(size * 2);
                let codec = TunnelCodec::new();
                let mut framed = FramedWrite::new(&mut writer, codec);

                framed.send(frame).await.unwrap();
                drop(framed);
                drop(writer);

                // Read all bytes
                let mut buf = Vec::new();
                let mut reader = reader;
                reader.read_to_end(&mut buf).await.unwrap();
                buf
            });

            b.to_async(&rt).iter(|| async {
                let (writer, _reader) = duplex(size * 2);
                drop(writer);

                // Write pre-encoded data
                let (mut w, r) = duplex(size * 2);
                w.write_all(&encoded).await.unwrap();
                drop(w);

                let codec = TunnelCodec::new();
                let mut framed = FramedRead::new(r, codec);

                let _ = framed.next().await.unwrap().unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark batched sender throughput
fn bench_batched_sender(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("batched_sender");

    for batch_size in [1, 8, 32, 128] {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async move {
                    let (tx, rx) = bounded_async::<Frame>(batch_size * 2);
                    let (writer, _reader) = duplex(128 * 1024);

                    // Spawn batched sender
                    tokio::spawn(async move {
                        ferrotunnel_core::transport::batched_sender::run_batched_sender(
                            rx,
                            writer,
                            TunnelCodec::new(),
                        )
                        .await;
                    });

                    // Send frames
                    for i in 0..batch_size {
                        let frame = Frame::Data {
                            stream_id: i as u32, // Loop variable is small, safe to cast
                            data: Bytes::from(vec![0u8; 1024]),
                            end_of_stream: false,
                        };
                        tx.send(frame).await.unwrap();
                    }

                    // Give time for batch to flush
                    tokio::time::sleep(Duration::from_millis(1)).await;
                });
            },
        );
    }

    group.finish();
}

/// Benchmark multiplexer stream creation
fn bench_multiplexer_stream_creation(c: &mut Criterion) {
    use ferrotunnel_core::stream::multiplexer::Multiplexer;
    use ferrotunnel_protocol::frame::Protocol;

    let rt = Runtime::new().unwrap();

    c.bench_function("multiplexer_stream_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let (tx, _rx) = bounded_async(100);
            let (mux, _new_stream_rx) = Multiplexer::new(tx, true);

            // Create 10 streams
            for _ in 0..10 {
                let _ = mux.open_stream(Protocol::TCP).await.unwrap();
            }
        });
    });
}

/// Benchmark complete round-trip through multiplexer
fn bench_multiplexer_round_trip(c: &mut Criterion) {
    use ferrotunnel_core::stream::multiplexer::Multiplexer;
    use ferrotunnel_protocol::frame::Protocol;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("multiplexer_round_trip");

    for size in [1024, 4096, 16384] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let (frame_tx, _frame_rx) = bounded_async(100);
                let (mux, stream_rx) = Multiplexer::new(frame_tx, true);

                // Simulate server-side accepting stream
                tokio::spawn(async move {
                    while let Ok(mut stream) = stream_rx.recv().await {
                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 65536];
                            while let Ok(n) = stream.read(&mut buf).await {
                                if n == 0 {
                                    break;
                                }
                                stream.write_all(&buf[..n]).await.unwrap();
                            }
                        });
                    }
                });

                // Client: open stream, send data, read response
                let mut stream = mux.open_stream(Protocol::TCP).await.unwrap();
                let data = vec![0u8; size];
                stream.write_all(&data).await.unwrap();

                let mut response = vec![0u8; size];
                stream.read_exact(&mut response).await.unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark bytes pool performance
fn bench_bytes_pool(c: &mut Criterion) {
    use ferrotunnel_core::stream::bytes_pool::{acquire_bytes, release_bytes};

    let mut group = c.benchmark_group("bytes_pool");

    group.bench_function("acquire_cold", |b| {
        b.iter(|| {
            let buf = acquire_bytes(4096);
            std::mem::forget(buf); // Don't release to keep pool empty
        });
    });

    group.bench_function("acquire_warm", |b| {
        // Pre-fill pool
        for _ in 0..10 {
            let buf = acquire_bytes(4096);
            release_bytes(buf);
        }

        b.iter(|| {
            let buf = acquire_bytes(4096);
            release_bytes(buf);
        });
    });

    group.bench_function("acquire_release_cycle", |b| {
        b.iter(|| {
            let mut buf = acquire_bytes(4096);
            buf.extend_from_slice(&[0u8; 1024]);
            let data = buf.freeze();
            drop(data);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_frame_encoding,
    bench_frame_decoding,
    bench_batched_sender,
    bench_multiplexer_stream_creation,
    bench_multiplexer_round_trip,
    bench_bytes_pool,
);
criterion_main!(benches);
