//! FerroTunnel Load Testing Tool
//!
//! Tests concurrent stream handling and measures latency/throughput.

use anyhow::Result;
use bytes::Bytes;
use clap::Parser;
use hdrhistogram::Histogram;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Barrier;
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(name = "ferrotunnel-loadgen")]
#[command(about = "Load testing tool for FerroTunnel")]
struct Args {
    /// Test mode: echo-server, echo-client, baseline
    #[arg(long, default_value = "baseline")]
    mode: String,

    /// Target address for client mode
    #[arg(long, default_value = "127.0.0.1:9999")]
    target: String,

    /// Bind address for server mode
    #[arg(long, default_value = "127.0.0.1:9999")]
    bind: String,

    /// Number of concurrent connections/streams
    #[arg(long, default_value = "100")]
    concurrency: usize,

    /// Number of requests per connection
    #[arg(long, default_value = "1000")]
    requests: usize,

    /// Payload size in bytes
    #[arg(long, default_value = "1024")]
    payload_size: usize,

    /// Test duration in seconds (0 = run requests count)
    #[arg(long, default_value = "0")]
    duration: u64,
}

/// Metrics collected during load test
#[derive(Debug)]
struct Metrics {
    total_requests: AtomicU64,
    total_bytes: AtomicU64,
    errors: AtomicU64,
    start_time: Instant,
}

impl Metrics {
    fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    fn record_request(&self, bytes: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    fn report(&self) {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let requests = self.total_requests.load(Ordering::Relaxed);
        let bytes = self.total_bytes.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);

        info!("=== Load Test Results ===");
        info!("Duration: {:.2}s", elapsed);
        info!("Total requests: {}", requests);
        info!(
            "Total bytes: {} ({:.2} MB)",
            bytes,
            bytes as f64 / 1_000_000.0
        );
        info!("Errors: {}", errors);
        info!("Throughput: {:.2} req/s", requests as f64 / elapsed);
        info!(
            "Bandwidth: {:.2} MB/s",
            (bytes as f64 / 1_000_000.0) / elapsed
        );
    }
}

/// Run echo server for testing
async fn run_echo_server(bind: &str) -> Result<()> {
    let listener = TcpListener::bind(bind).await?;
    info!("Echo server listening on {}", bind);

    loop {
        let (mut socket, addr) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];
            loop {
                match socket.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if socket.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = addr;
        });
    }
}

/// Run load test client
async fn run_load_client(
    target: String,
    concurrency: usize,
    requests_per_conn: usize,
    payload_size: usize,
) -> Result<()> {
    let metrics = Arc::new(Metrics::new());
    let barrier = Arc::new(Barrier::new(concurrency));
    let mut histogram = Histogram::<u64>::new(3)?;

    let payload = Bytes::from(vec![b'X'; payload_size]);

    info!(
        "Starting load test: {} connections, {} requests each, {} byte payload",
        concurrency, requests_per_conn, payload_size
    );

    let mut handles = Vec::with_capacity(concurrency);

    for i in 0..concurrency {
        let target = target.clone();
        let metrics = Arc::clone(&metrics);
        let barrier = Arc::clone(&barrier);
        let payload = payload.clone();

        let handle = tokio::spawn(async move {
            // Wait for all connections to be ready
            barrier.wait().await;

            let mut latencies = Vec::with_capacity(requests_per_conn);

            match TcpStream::connect(&target).await {
                Ok(mut stream) => {
                    for _ in 0..requests_per_conn {
                        let start = Instant::now();

                        // Send payload
                        if stream.write_all(&payload).await.is_err() {
                            metrics.record_error();
                            continue;
                        }

                        // Read response
                        let mut response = vec![0u8; payload.len()];
                        if stream.read_exact(&mut response).await.is_err() {
                            metrics.record_error();
                            continue;
                        }

                        let latency = start.elapsed();
                        latencies.push(latency.as_micros() as u64);
                        metrics.record_request(payload.len() as u64 * 2);
                    }
                }
                Err(e) => {
                    error!("Connection {} failed: {}", i, e);
                    metrics.record_error();
                }
            }

            latencies
        });

        handles.push(handle);
    }

    // Collect results
    for handle in handles {
        match handle.await {
            Ok(latencies) => {
                for lat in latencies {
                    if histogram.record(lat).is_err() {
                        warn!("Failed to record latency");
                    }
                }
            }
            Err(e) => {
                error!("Task failed: {}", e);
            }
        }
    }

    // Report metrics
    metrics.report();

    // Report latency percentiles
    info!("=== Latency (microseconds) ===");
    info!("p50: {} µs", histogram.value_at_quantile(0.50));
    info!("p90: {} µs", histogram.value_at_quantile(0.90));
    info!("p95: {} µs", histogram.value_at_quantile(0.95));
    info!("p99: {} µs", histogram.value_at_quantile(0.99));
    info!("max: {} µs", histogram.max());

    Ok(())
}

/// Run baseline test (local echo, no tunnel)
async fn run_baseline(
    concurrency: usize,
    requests_per_conn: usize,
    payload_size: usize,
) -> Result<()> {
    let bind = "127.0.0.1:19999";

    // Start echo server
    let server_handle = tokio::spawn(async move {
        if let Err(e) = run_echo_server(bind).await {
            error!("Echo server error: {}", e);
        }
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Run client
    run_load_client(
        bind.to_string(),
        concurrency,
        requests_per_conn,
        payload_size,
    )
    .await?;

    server_handle.abort();
    Ok(())
}

#[cfg(target_os = "linux")]
fn print_memory_usage() {
    if let Ok(me) = procfs::process::Process::myself() {
        if let Ok(status) = me.status() {
            if let Some(rss) = status.vmrss {
                info!("Memory (RSS): {} KB", rss);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn print_memory_usage() {
    info!("Memory tracking not available on this platform");
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    let args = Args::parse();

    info!("FerroTunnel Load Generator");
    print_memory_usage();

    match args.mode.as_str() {
        "echo-server" => {
            run_echo_server(&args.bind).await?;
        }
        "echo-client" => {
            run_load_client(
                args.target,
                args.concurrency,
                args.requests,
                args.payload_size,
            )
            .await?;
        }
        "baseline" => {
            info!("Running baseline test (local echo, no tunnel)");
            run_baseline(args.concurrency, args.requests, args.payload_size).await?;
        }
        _ => {
            error!("Unknown mode: {}", args.mode);
            std::process::exit(1);
        }
    }

    print_memory_usage();
    Ok(())
}
