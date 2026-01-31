//! End-to-End Tunnel Benchmarks
//!
//! Benchmarks that measure complete tunnel operations.
#![allow(clippy::unwrap_used)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ferrotunnel_protocol::frame::{DataFrame, HandshakeFrame};
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

/// Benchmark tunnel handshake and connection setup
fn bench_tunnel_setup(c: &mut Criterion) {
    let mut group = c.benchmark_group("tunnel_setup");

    // Benchmark protocol frame encoding (lightweight, no actual network)
    group.bench_function("frame_encode_handshake", |b| {
        let frame = Frame::Handshake(Box::new(HandshakeFrame {
            min_version: 1,
            max_version: 1,
            tunnel_id: Some("test-tunnel-id".to_string()),
            token: "secret-token-12345".to_string(),
            capabilities: vec![],
        }));

        b.iter(|| {
            let encoded = encode_frame(&frame);
            black_box(encoded)
        });
    });

    group.bench_function("frame_encode_data_1kb", |b| {
        let data = vec![0u8; 1024];
        let frame = Frame::Data(Box::new(DataFrame {
            stream_id: 1,
            data: data.into(),
            end_of_stream: false,
        }));

        b.iter(|| {
            let encoded = encode_frame(&frame);
            black_box(encoded)
        });
    });

    group.bench_function("frame_decode_roundtrip", |b| {
        let frame = Frame::Handshake(Box::new(HandshakeFrame {
            min_version: 1,
            max_version: 1,
            tunnel_id: Some("test-tunnel-id".to_string()),
            token: "secret-token-12345".to_string(),
            capabilities: vec![],
        }));
        let encoded = encode_frame(&frame);

        b.iter(|| {
            let decoded = decode_frame(&encoded);
            black_box(decoded)
        });
    });

    group.finish();
}

/// Benchmark plugin chain execution
fn bench_plugin_chain(c: &mut Criterion) {
    use ferrotunnel_plugin::builtin::{LoggerPlugin, RateLimitPlugin, TokenAuthPlugin};
    use ferrotunnel_plugin::{PluginRegistry, RequestContext};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("plugin_chain");

    // Setup registry with plugins
    let registry = rt.block_on(async {
        let mut reg = PluginRegistry::new();
        reg.register(Arc::new(RwLock::new(TokenAuthPlugin::new(vec![
            "token".to_string()
        ]))));
        reg.register(Arc::new(RwLock::new(RateLimitPlugin::new(10000))));
        reg.register(Arc::new(RwLock::new(LoggerPlugin::new())));
        reg.init_all().await.unwrap();
        reg
    });

    // Reuse request and context to avoid allocation noise
    // But Request is consumed, so we must build it inside the loop
    let ctx = RequestContext {
        tunnel_id: "bench".to_string(),
        session_id: "session".to_string(),
        remote_addr: "127.0.0.1:8080".parse().unwrap(),
        timestamp: std::time::SystemTime::now(),
    };

    group.bench_function("execute_3_plugins", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut req = http::Request::builder()
                    .method("GET")
                    .uri("/api/test")
                    .header("X-Tunnel-Token", "token")
                    .body(())
                    .unwrap();

                let result = registry.execute_request_hooks(&mut req, &ctx).await;
                black_box(result)
            })
        });
    });

    group.finish();
}

criterion_group!(benches, bench_tunnel_setup, bench_plugin_chain);
criterion_main!(benches);
