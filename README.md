# FerroTunnel 🦀

[![CI](https://github.com/MitulShah1/ferrotunnel/workflows/CI/badge.svg)](https://github.com/MitulShah1/ferrotunnel/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/ferrotunnel)](https://crates.io/crates/ferrotunnel)
[![Documentation](https://docs.rs/ferrotunnel/badge.svg)](https://docs.rs/ferrotunnel)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.90%2B-orange.svg)](https://www.rust-lang.org)

**High-performance reverse tunnel you can embed in your Rust applications.**

FerroTunnel multiplexes streams over a single connection (like ngrok/Cloudflare Tunnel) but ships as a **library-first** crate. Expose local services behind NAT, route HTTP by hostname, intercept requests with plugins with minimal memory footprint and sub-millisecond latency. Works as CLI or `Client::builder()` API. Written in Rust.

## Quick Start

### CLI

```bash
# Install
cargo install ferrotunnel-cli

# Start server
ferrotunnel server --token secret

# Start client (in another terminal; token from env or secure prompt if omitted)
ferrotunnel client --server localhost:7835 --local-addr 127.0.0.1:8080
```

### Library

```toml
[dependencies]
ferrotunnel = "0.1"
tokio = { version = "1", features = ["full"] }
```

```rust
use ferrotunnel::Client;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    let mut client = Client::builder()
        .server_addr("tunnel.example.com:7835")
        .token("my-secret-token")
        .local_addr("127.0.0.1:8080")
        .build()?;

    client.start().await?;

    tokio::signal::ctrl_c().await?;
    client.shutdown().await
}
```

## Features

| Feature | Description |
|---------|-------------|
| **Embeddable** | Use as a library with builder APIs |
| **Plugin System** | Auth, rate limiting, logging, circuit breaker |
| **Dashboard** | Real-time WebUI at `localhost:4040` |
| **TLS 1.3** | Secure connections with rustls |
| **Mutual TLS** | Client certificate authentication |
| **Observability** | Prometheus metrics + OpenTelemetry tracing |
| **TCP & HTTP** | Forward both HTTP and raw TCP traffic |



**Choose FerroTunnel when**: You need many services over a single connection, HTTP routing, plugins, or resource efficiency.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed analysis of the multiplexing trade-off.

## CLI Reference


### Server

```bash
ferrotunnel server [OPTIONS]
```

| Option | Env Variable | Default | Description |
|--------|--------------|---------|-------------|
| `--token` | `FERROTUNNEL_TOKEN` | required | Auth token |
| `--bind` | `FERROTUNNEL_BIND` | `0.0.0.0:7835` | Control plane |
| `--http-bind` | `FERROTUNNEL_HTTP_BIND` | `0.0.0.0:8080` | HTTP ingress |
| `--tcp-bind` | `FERROTUNNEL_TCP_BIND` | - | TCP ingress |
| `--tls-cert` | `FERROTUNNEL_TLS_CERT` | - | TLS certificate |
| `--tls-key` | `FERROTUNNEL_TLS_KEY` | - | TLS private key |

### Client

```bash
ferrotunnel client [OPTIONS]
```

| Option | Env Variable | Default | Description |
|--------|--------------|---------|-------------|
| `--server` | `FERROTUNNEL_SERVER` | required | Server address |
| `--token` | `FERROTUNNEL_TOKEN` | optional | Auth token; if omitted, uses env or prompts securely |
| `--local-addr` | `FERROTUNNEL_LOCAL_ADDR` | `127.0.0.1:8000` | Local service |
| `--dashboard-port` | `FERROTUNNEL_DASHBOARD_PORT` | `4040` | Dashboard port |
| `--tls` | `FERROTUNNEL_TLS` | false | Enable TLS |
| `--tls-ca` | `FERROTUNNEL_TLS_CA` | - | CA certificate |

See [ferrotunnel-cli/README.md](ferrotunnel-cli/README.md) for all options.

## Crates

| Crate | Description |
|-------|-------------|
| [`ferrotunnel`](ferrotunnel/) | Main library with builder APIs |
| [`ferrotunnel-cli`](ferrotunnel-cli/) | Unified CLI binary |
| [`ferrotunnel-core`](ferrotunnel-core/) | Tunnel logic and transport |
| [`ferrotunnel-protocol`](ferrotunnel-protocol/) | Wire protocol and codec |
| [`ferrotunnel-http`](ferrotunnel-http/) | HTTP/TCP ingress and proxy |
| [`ferrotunnel-plugin`](ferrotunnel-plugin/) | Plugin system |
| [`ferrotunnel-observability`](ferrotunnel-observability/) | Metrics and dashboard |
| [`ferrotunnel-common`](ferrotunnel-common/) | Shared types |

## Installation

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/MitulShah1/ferrotunnel/releases).

### From Source

```bash
cargo install ferrotunnel-cli
```

### Docker

```bash
docker-compose up --build
```

## Documentation

- [CLI Reference](ferrotunnel-cli/README.md)
- [Contributing](CONTRIBUTING.md) & [Code of Conduct](CODE_OF_CONDUCT.md)
- [Architecture](ARCHITECTURE.md)
- [Benchmark & Performance](docs/benchmark.md)
- [Deployment Guide](docs/deployment.md)
- [Plugin Development](docs/plugin-development.md)
- [Security](docs/security.md)

## Development

```bash
# Build
cargo build --workspace

# Test
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Benchmark
cargo bench --workspace
```

### Developer Tools

- [`tools/loadgen`](tools/loadgen/) - Load testing
- [`tools/soak`](tools/soak/) - Stability testing
- [`tools/profiler`](tools/profiler/) - Performance profiling

## Benchmark

FerroTunnel is benchmarked against [rathole](https://github.com/rapiz1/rathole) and [frp](https://github.com/fatedier/frp). Unlike rathole/frp which use 1:1 TCP forwarding, FerroTunnel uses **multiplexed streams over a single connection** the same architecture used by [ngrok](https://ngrok.com/docs/http/) and [Cloudflare Tunnel](https://developers.cloudflare.com/speed/optimization/protocol/http2-to-origin/) (HTTP/2 multiplexing). This enables HTTP routing, plugins, and multi-service tunnels.

| Metric | FerroTunnel | Rathole | frp |
|--------|-------------|---------|-----|
| **Throughput** | 382 MB/s | 1349 MB/s | 690 MB/s |
| **Latency (P99)** | 0.114ms | 0.075ms | 0.131ms |
| **Memory/conn** | 47.3 KB | 35.8 KB | 113.7 KB |

**Why the throughput difference?** Multiplexing adds frame encoding/decoding overhead this is the cost of features like HTTP host routing, request plugins, and running many services over one tunnel. FerroTunnel is **18% faster than frp** on latency and uses **58% less memory**.

<p align="center">
  <img src="docs/static/server_heap_graph.png" alt="Server Heap Graph" width="45%">
  <img src="docs/static/top_allocations.png" alt="Top Allocations" width="45%">
</p>
<p align="center"><em>Memory profile: flat heap usage, minimal allocations under load</em></p>

See [docs/benchmark.md](docs/benchmark.md) for detailed analysis of the architectural trade-offs and [ferrotunnel-benchmarks](https://github.com/MitulShah1/ferrotunnel-benchmarks) for reproducible tests.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
