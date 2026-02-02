# FerroTunnel

[![CI](https://github.com/MitulShah1/ferrotunnel/workflows/CI/badge.svg)](https://github.com/MitulShah1/ferrotunnel/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/ferrotunnel)](https://crates.io/crates/ferrotunnel)
[![Documentation](https://docs.rs/ferrotunnel/badge.svg)](https://docs.rs/ferrotunnel)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Security](https://img.shields.io/badge/security-policy-green)](SECURITY.md)

**The First Embeddable Rust Reverse Tunnel**

FerroTunnel is a secure, high-performance reverse tunnel system in Rust. Unlike CLI-only alternatives, FerroTunnel can be **embedded directly into your applications** using a simple builder API, making it ideal for IoT devices, microservices, and custom networking solutions.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
ferrotunnel = "1.0.0"
tokio = { version = "1", features = ["full"] }
```

### Embedded Client

```rust
use ferrotunnel::Client;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    let mut client = Client::builder()
        .server_addr("tunnel.example.com:7835")
        .token("my-secret-token")
        .local_addr("127.0.0.1:8080")
        .build()?;

    let info = client.start().await?;
    println!("Connected! Session: {:?}", info.session_id);

    // Keep running until Ctrl+C
    tokio::signal::ctrl_c().await?;
    client.shutdown().await?;
    Ok(())
}
```

### Embedded Server

```rust
use ferrotunnel::Server;

#[tokio::main]
async fn main() -> ferrotunnel::Result<()> {
    let mut server = Server::builder()
        .bind("0.0.0.0:7835".parse().unwrap())
        .http_bind("0.0.0.0:8080".parse().unwrap())
        .token("my-secret-token")
        .build()?;

    server.start().await?;
    Ok(())
}
```

### CLI

Install and run without embedding:

```bash
cargo install ferrotunnel-cli
```

**Server:**

```bash
ferrotunnel server --token "your-secret-token"
```

**Client:**

```bash
ferrotunnel client \
  --server tunnel.example.com:7835 \
  --token "your-secret-token" \
  --local-addr 127.0.0.1:8080
```

See [ferrotunnel-cli](ferrotunnel-cli/) for full CLI reference.

## Features

### Embeddable Library API

The only Rust tunnel that can be embedded directly into your applications. No external processes, no CLI wrappers - just import and use.

```rust
// Embed tunneling in your IoT device, microservice, or custom application
let client = Client::builder()
    .server_addr("tunnel.example.com:7835")
    .token("device-token")
    .local_addr("127.0.0.1:8080")
    .build()?;
```

### Plugin System

Trait-based extensibility for authentication, rate limiting, logging, and custom logic:

```rust
#[async_trait]
impl Plugin for CustomAuth {
    async fn on_request(&self, req: &mut Request<()>, ctx: &RequestContext)
        -> Result<PluginAction>
    {
        if !validate_token(req.headers()) {
            return Ok(PluginAction::Reject { status: 401, reason: "Unauthorized".into() });
        }
        Ok(PluginAction::Continue)
    }
}
```

**Built-in Plugins:**
- Token-based authentication
- Rate limiting (streams/sec, bytes/sec)
- Request/response logging
- Circuit breaker for fault tolerance

### Built-in Dashboard

Real-time WebUI for monitoring tunnels and inspecting requests without external tools:

- Live tunnel status and health
- Request/response inspector
- Traffic graphs and statistics
- Prometheus metrics endpoint (`/metrics`)

### Security & Hardening

Production-ready security from day one:

- **TLS 1.3** encryption with rustls
- **Token-based authentication** with constant-time comparison
- **Resource limits** for sessions, streams, and frame sizes
- **Rate limiting** per session
- **No unsafe code** (`unsafe_code = "forbid"`)
- **Fuzz-tested** protocol decoder
- **Automated security audits** with cargo-deny

### Observability

Full observability stack built-in:

- **Prometheus metrics** - Request rates, latencies, error counts
- **OpenTelemetry tracing** - Distributed tracing with OTLP export
- **Structured logging** - JSON-formatted logs for aggregation

### Resilience

Built for production reliability:

- **Automatic reconnection** with exponential backoff
- **Heartbeat monitoring** for connection health
- **Circuit breakers** for downstream protection
- **Graceful shutdown** with connection draining

## Performance Benchmarks

Benchmarks run on Ubuntu 22.04, AMD Ryzen 9 5900X, 64GB RAM.

### Latency Comparison

```
Latency (p99) - Lower is Better
═══════════════════════════════════════════════════════════════

FerroTunnel   ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   < 5ms
Rathole       ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   ~ 5ms
frp           ████████████████░░░░░░░░░░░░░░░░░░░░░░░░   ~ 10ms
Direct TCP    ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   < 0.5ms (baseline)

              0ms        5ms        10ms       15ms       20ms
```

### Throughput Comparison

```
Throughput (Single Stream) - Higher is Better
═══════════════════════════════════════════════════════════════

FerroTunnel   ██████████████████████████████████████░░   > 5 Gbps
Rathole       ████████████████████████████████░░░░░░░░   ~ 4 Gbps
frp           ████████████████░░░░░░░░░░░░░░░░░░░░░░░░   ~ 2 Gbps
Direct TCP    ████████████████████████████████████████   ~ 10 Gbps (baseline)

              0          2          4          6          8          10 Gbps
```

### Memory Usage

```
Memory (1000 Tunnels) - Lower is Better
═══════════════════════════════════════════════════════════════

FerroTunnel   ██████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   ~ 100 MB
Rathole       ████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░   ~ 80 MB
frp           ██████████████████████████████░░░░░░░░░░   ~ 300 MB

              0          100        200        300        400 MB
```

### Performance Summary

| Metric | FerroTunnel | Notes |
|--------|-------------|-------|
| Latency (p50) | < 1ms | Single-hop tunnel |
| Latency (p99) | < 5ms | Under load |
| Throughput | > 5 Gbps | TCP tunnel, single stream |
| Concurrent Streams | 10,000+ | Per server |
| Memory (idle) | ~15 MB | Server with no connections |
| Memory (1000 tunnels) | < 100 MB | Server under load |

Run benchmarks yourself:

```bash
cargo bench --workspace
```

## Crates

| Crate | Description |
|-------|-------------|
| `ferrotunnel` | Main library with Client/Server builder API |
| `ferrotunnel-cli` | Unified CLI binary (`ferrotunnel server`, `ferrotunnel client`) |
| `ferrotunnel-protocol` | Wire protocol (12 frame types, binary codec) |
| `ferrotunnel-core` | Tunnel implementation, multiplexing, transport |
| `ferrotunnel-http` | HTTP ingress and proxy |
| `ferrotunnel-plugin` | Plugin traits and built-in plugins |
| `ferrotunnel-observability` | Metrics, tracing, and dashboard |
| `ferrotunnel-common` | Shared types and error handling |

## Tools

- **[`ferrotunnel-soak`](tools/soak)** - Long-duration stability testing
- **[`ferrotunnel-loadgen`](tools/loadgen)** - High-performance load generator
- **[`ferrotunnel-protocol/fuzz`](ferrotunnel-protocol/fuzz)** - Fuzz testing for protocol robustness

## Deployment

### Docker

```bash
# Quick start
docker-compose up --build

# Production build
docker build -t ferrotunnel .
```

### Pre-compiled Binaries

Binaries for Linux, macOS, and Windows are available on the [GitHub Releases](https://github.com/MitulShah1/ferrotunnel/releases) page.

### From Source

```bash
# Build all
cargo build --workspace --release

# Install CLI
cargo install --path ferrotunnel-cli
```

## Documentation

- [Architecture](ARCHITECTURE.md) - System design and crate structure
- [Security](docs/security.md) - Security practices and hardening
- [Deployment](docs/deployment.md) - Production deployment guide
- [Plugin Development](docs/plugin-development.md) - Building custom plugins
- [Troubleshooting](docs/troubleshooting.md) - Common issues and solutions
- [Changelog](CHANGELOG.md) - Version history
- [Roadmap](ROADMAP.md) - Development roadmap

## Security

Security is a top priority. If you discover a vulnerability, please see our [Security Policy](SECURITY.md) for responsible disclosure guidelines.

**Contact:** shahmitul005@gmail.com

## Contributing

Contributions are welcome! Before contributing:

1. Read the [Contributing Guide](CONTRIBUTING.md)
2. Review the [Code of Conduct](CODE_OF_CONDUCT.md)
3. Check [ARCHITECTURE.md](ARCHITECTURE.md) for project structure
4. See [ROADMAP.md](ROADMAP.md) for planned features

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
