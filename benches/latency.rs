#![allow(clippy::unwrap_used)]
#![allow(clippy::cast_possible_truncation)]

//! Latency benchmarks measuring percentiles
//!
//! Focuses on tail latency (p50, p95, p99, p999)

use bytes::Bytes;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ferrotunnel_protocol::{Frame, TunnelCodec};
use kanal::bounded_async;
use std::time::{Duration, Instant};
use tokio::io::{duplex, AsyncWriteExt};
use tokio::runtime::Runtime;
use tokio_util::codec::Encoder;

/// Measure latency distribution for frame encoding
fn bench_frame_encoding_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("frame_encoding_latency");
    group.measurement_time(Duration::from_secs(10));

    for size in [1024, 16384, 65536] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter_custom(|iters| async move {
                let mut latencies = Vec::with_capacity(iters as usize);

                for _ in 0..iters {
                    let data = Bytes::from(vec![0u8; size]);
                    let frame = Frame::Data {
                        stream_id: 1,
                        data,
                        end_of_stream: false,
                    };

                    let (mut writer, _reader) = duplex(size * 2);
                    let mut codec = TunnelCodec::new();

                        let start = Instant::now();

                        let mut buf = bytes::BytesMut::new();
                        codec.encode(frame, &mut buf).unwrap();

                        writer.write_all(&buf).await.unwrap();

                        let elapsed = start.elapsed();
                    latencies.push(elapsed);
                }

                // Calculate percentiles
                latencies.sort();
                let p50 = latencies[latencies.len() / 2];
                let p95 = latencies[latencies.len() * 95 / 100];
                let p99 = latencies[latencies.len() * 99 / 100];
                let p999 = latencies[latencies.len() * 999 / 1000];

                    println!(
                        "\n  {size}B encoding - p50: {p50:?}, p95: {p95:?}, p99: {p99:?}, p999: {p999:?}"
                    );

                latencies.iter().sum()
            });
        });
    }

    group.finish();
}

/// Measure end-to-end latency through batched sender
fn bench_batched_sender_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("batched_sender_latency", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let (tx, rx) = bounded_async::<Frame>(1000);
            let (writer, _reader) = duplex(1024 * 1024);

            // Spawn batched sender
            tokio::spawn(async move {
                ferrotunnel_core::transport::batched_sender::run_batched_sender(
                    rx,
                    writer,
                    TunnelCodec::new(),
                )
                .await;
            });

            let mut latencies = Vec::with_capacity(iters as usize);

            for _ in 0..iters {
                let frame = Frame::Heartbeat { timestamp: 12345 };

                let start = Instant::now();
                tx.send(frame).await.unwrap();

                // Wait for batch flush (approximation)
                tokio::time::sleep(Duration::from_micros(50)).await;

                let elapsed = start.elapsed();
                latencies.push(elapsed);
            }

            // Calculate percentiles
            latencies.sort();
            let p50 = latencies[latencies.len() / 2];
            let p95 = latencies[latencies.len() * 95 / 100];
            let p99 = latencies[latencies.len() * 99 / 100];

            println!("\n  Batched sender - p50: {p50:?}, p95: {p95:?}, p99: {p99:?}");

            latencies.iter().sum()
        });
    });
}

/// Measure multiplexer stream I/O latency
fn bench_multiplexer_io_latency(c: &mut Criterion) {
    use ferrotunnel_core::stream::multiplexer::Multiplexer;
    use ferrotunnel_protocol::frame::Protocol;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("multiplexer_io_latency");
    group.measurement_time(Duration::from_secs(10));

    for size in [128, 1024, 4096] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.to_async(&rt).iter_custom(|iters| async move {
                let (frame_tx, _frame_rx) = bounded_async(1000);
                let (mux, stream_rx) = Multiplexer::new(frame_tx, true);

                // Echo server
                tokio::spawn(async move {
                    while let Ok(mut stream) = stream_rx.recv().await {
                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 65536];
                            if let Ok(n) = stream.read(&mut buf).await {
                                let _ = stream.write_all(&buf[..n]).await;
                            }
                        });
                    }
                });

                let mut latencies = Vec::with_capacity(iters as usize);

                for _ in 0..iters {
                    let mut stream = mux.open_stream(Protocol::TCP).await.unwrap();
                    let data = vec![0u8; size];

                    let start = Instant::now();
                    stream.write_all(&data).await.unwrap();

                    let mut response = vec![0u8; size];
                    stream.read_exact(&mut response).await.unwrap();

                    let elapsed = start.elapsed();
                    latencies.push(elapsed);
                }

                // Calculate percentiles
                latencies.sort();
                let p50 = latencies[latencies.len() / 2];
                let p95 = latencies[latencies.len() * 95 / 100];
                let p99 = latencies[latencies.len() * 99 / 100];

                println!("\n  {size}B I/O - p50: {p50:?}, p95: {p95:?}, p99: {p99:?}");

                latencies.iter().sum()
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_frame_encoding_latency,
    bench_batched_sender_latency,
    bench_multiplexer_io_latency,
);
criterion_main!(benches);
