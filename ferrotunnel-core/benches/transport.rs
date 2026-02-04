#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::pedantic)]
#![allow(unused_imports, unused_variables)]
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use ferrotunnel_core::transport::{self, BoxedStream, TransportConfig};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Starts an echo server on the given address and returns its local socket address.
///
/// # Panics
/// Panics if binding to `addr` fails or if the listener's local address cannot be obtained.
async fn run_echo_server(addr: &str) -> String {
    let listener = TcpListener::bind(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 1024];
                loop {
                    let n = match socket.read(&mut buf).await {
                        Ok(0) => return,
                        Ok(n) => n,
                        Err(_) => return,
                    };
                    if socket.write_all(&buf[..n]).await.is_err() {
                        return;
                    }
                }
            });
        }
    });

    local_addr
}

fn bench_transport_connect(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let addr = rt.block_on(async { run_echo_server("127.0.0.1:0").await });

    // Ensure server is up
    std::thread::sleep(Duration::from_millis(100));

    let mut group = c.benchmark_group("transport_connect");

    group.bench_function("tcp_connect", |b| {
        b.to_async(&rt).iter(|| {
            let addr = addr.clone();
            async move {
                let config = TransportConfig::default();
                let _stream = transport::connect(black_box(&config), black_box(&addr))
                    .await
                    .unwrap();
            }
        });
    });

    group.finish();
}

fn bench_transport_throughput(c: &mut Criterion) {
    const KB: usize = 1024;
    let rt = tokio::runtime::Runtime::new().unwrap();
    let addr = rt.block_on(async { run_echo_server("127.0.0.1:0").await });

    // Ensure server is up
    std::thread::sleep(Duration::from_millis(100));

    let mut group = c.benchmark_group("transport_throughput");
    let sizes = [KB, 64 * KB];

    for size in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                b.to_async(&rt).iter_custom(|iters| {
                    let addr = addr.clone();
                    async move {
                        let config = TransportConfig::default();
                        let mut stream = transport::connect(&config, &addr).await.unwrap();
                        let payload = vec![0u8; size];
                        let mut buf = vec![0u8; size];

                        let start = std::time::Instant::now();
                        for _ in 0..iters {
                            stream.write_all(&payload).await.unwrap();
                            stream.read_exact(&mut buf).await.unwrap();
                        }
                        start.elapsed()
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_transport_connect, bench_transport_throughput);
criterion_main!(benches);
