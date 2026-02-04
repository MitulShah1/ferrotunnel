#![allow(clippy::unwrap_used)]
#![allow(clippy::cast_possible_truncation)]
use bytes::Bytes;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ferrotunnel_core::stream::PrioritizedFrame;
use ferrotunnel_core::transport::batched_sender::run_batched_sender;
use ferrotunnel_protocol::codec::TunnelCodec;
use ferrotunnel_protocol::frame::StreamPriority;
use ferrotunnel_protocol::Frame;
use kanal::bounded_async;
use tokio::io::{duplex, AsyncReadExt};

fn bench_batched_sender_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("batched_sender_throughput");

    // Test useful payload sizes
    for size in &[1024, 16384, 65536] {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter_custom(|iters| {
                async move {
                    // Setup pipeline
                    let (frame_tx, frame_rx) = bounded_async::<PrioritizedFrame>(100);
                    let (writer, mut reader) = duplex(65536); // Pipe buffer size

                    // Spawn batched sender
                    tokio::spawn(run_batched_sender(frame_rx, writer, TunnelCodec::new()));

                    // Sink task (reader)
                    tokio::spawn(async move {
                        let mut buf = vec![0u8; 65536];
                        loop {
                            match reader.read(&mut buf).await {
                                Ok(0) | Err(_) => break,
                                Ok(_) => {}
                            }
                        }
                    });

                    let payload = Bytes::from(vec![0u8; size]);
                    let start = std::time::Instant::now();

                    for i in 0..iters {
                        let frame = Frame::Data {
                            stream_id: i as u32,
                            data: payload.clone(), // Cheap clone (ref count increment)
                            end_of_stream: false,
                        };
                        frame_tx
                            .send((StreamPriority::Normal, frame))
                            .await
                            .unwrap();
                    }

                    // Close channel and wait for flush
                    drop(frame_tx);

                    // We don't have a signal for "done" from reader easily within benchmark iter_custom
                    // without coordination.
                    // But iter_custom usually needs to return elapsed time.
                    // The time to *send* is what we measure?
                    // Or end-to-end?
                    // Ideally end-to-end.
                    // But measuring "time to send" is also valid for sender throughput.
                    // If channel fills up, send waits. So backpressure propagates.

                    start.elapsed()
                }
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_batched_sender_throughput);
criterion_main!(benches);
