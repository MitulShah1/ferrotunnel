#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::pedantic)]
#![allow(unused_imports, unused_variables)]
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use ferrotunnel_core::stream::multiplexer::Multiplexer;
use ferrotunnel_protocol::frame::{Frame, Protocol};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use std::time::Duration;

fn bench_multiplexer_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiplexer_throughput");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Throughput for processing incoming data frames
    const MSG_SIZE: usize = 1024;
    group.throughput(Throughput::Bytes(MSG_SIZE as u64));

    group.bench_function("process_data_frame", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let (frame_tx, mut frame_rx) = mpsc::channel(100);
            let (multiplexer, _new_stream_rx) = Multiplexer::new(frame_tx, true);

            // We need a dummy consumer for frame_rx so the channel doesn't fill up
            tokio::spawn(async move { while frame_rx.next().await.is_some() {} });

            let frame = Frame::Data {
                stream_id: 1,
                data: bytes::Bytes::from(vec![0u8; MSG_SIZE]),
                end_of_stream: false,
            };

            let start = std::time::Instant::now();
            for _ in 0..iters {
                // Since process_frame is async and might block on channel, we bench the send
                multiplexer.send_frame(frame.clone()).await.unwrap();
            }
            start.elapsed()
        });
    });

    group.finish();
}

fn bench_multiplexer_stream_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiplexer_control");
    let rt = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("open_stream", |b| {
        b.to_async(&rt).iter_custom(|iters| async move {
            let (frame_tx, mut frame_rx) = mpsc::channel(100);
            let (multiplexer, _new_stream_rx) = Multiplexer::new(frame_tx, true);

            // Dummy consumer
            tokio::spawn(async move { while frame_rx.next().await.is_some() {} });

            let start = std::time::Instant::now();
            for _ in 0..iters {
                // Open a new stream
                let _stream = multiplexer.open_stream(Protocol::HTTP).await.unwrap();
            }
            start.elapsed()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_multiplexer_throughput,
    bench_multiplexer_stream_creation
);
criterion_main!(benches);
