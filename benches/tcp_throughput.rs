#![allow(clippy::unwrap_used)]

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn run_echo_server(addr: &str) -> String {
    let listener = TcpListener::bind(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = vec![0u8; 65536];
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

fn bench_tcp_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let addr = rt.block_on(async { run_echo_server("127.0.0.1:0").await });

    std::thread::sleep(Duration::from_millis(100));

    let mut group = c.benchmark_group("tcp_throughput");
    let sizes = [1024, 4096, 16384, 65536];

    for size in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                b.to_async(&rt).iter_custom(|iters| {
                    let addr = addr.clone();
                    async move {
                        let mut stream = TcpStream::connect(&addr).await.unwrap();
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

criterion_group!(benches, bench_tcp_throughput);
criterion_main!(benches);
