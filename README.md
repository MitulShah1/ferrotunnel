# FerroTunnel 🦀

[![CI](https://github.com/MitulShah1/ferrotunnel/workflows/CI/badge.svg)](https://github.com/MitulShah1/ferrotunnel/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/ferrotunnel)](https://crates.io/crates/ferrotunnel)
[![Documentation](https://docs.rs/ferrotunnel/badge.svg)](https://docs.rs/ferrotunnel)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Security](https://img.shields.io/badge/security-policy-green)](SECURITY.md)

**The First Embeddable Rust Reverse Tunnel**

FerroTunnel is a secure reverse tunnel system in Rust. Unlike CLI-only alternatives, FerroTunnel can be **embedded directly into your applications** using a simple builder API.

## Quick Start: Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
ferrotunnel = "0.9.6"
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

## Features

- 🔒 **Secure** - Token-based authentication
- 🛡️ **Encrypted** - Native TLS 1.3 support for secure transit
- ⚡  **Fast** - Built on Tokio for high-performance async I/O
- 🔌 **Embeddable** - Use as a library in your own applications
- 🛡️ **Resilient** - Automatic reconnection, heartbeat monitoring
- 📦 **Modular** - Use only what you need


## Crates

### `ferrotunnel-protocol`

Wire protocol for tunnel communication with 12 message types:
- Control frames: Handshake, Register
- Stream frames: OpenStream, Data, CloseStream
- Keepalive: Heartbeat
- Error handling
- Plugin support

### `ferrotunnel-common`

Shared utilities and error types.

### `ferrotunnel-plugin`

Trait-based plugin system for intercepting traffic:
- Request/Response hooks
- Built-in Auth and Rate Limiting
- Custom logic support

### `ferrotunnel-observability`

Metrics and tracing infrastructure:
- Prometheus metrics endpoint (`:9090/metrics`)
- OpenTelemetry distributed tracing
- OTLP/gRPC exporter support

## Tools

FerroTunnel includes powerful tools to ensure reliability and performance:

- **[`ferrotunnel-soak`](tools/soak)**: Long-duration stability testing tool.
- **[`ferrotunnel-loadgen`](tools/loadgen)**: High-performance load generator.
- **`ferrotunnel-protocol/fuzz`**: Fuzz testing for protocol robustness.

## Hardening

We prioritize security and stability. See our [Hardening Overview](extra/hardening_overview.md) for details on:
- 🛡️ **Fuzz Testing**: Continuous fuzzing to catch edge cases.
- ⚡ **Benchmarks**: Performance tracking for latency and throughput.
- 🔒 **Security Audits**: Automated dependency auditing.

## Deployment

### Docker

Quick start with Docker Compose:

```bash
docker-compose up --build
```

Build production image manually:

```bash
docker build -t ferrotunnel-server .
```

### Pre-compiled Binaries

Binaries for Linux, macOS, and Windows are available on the [GitHub Releases](https://github.com/MitulShah1/ferrotunnel/releases) page.

## Building

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Build with optimizations
cargo build --release

# Generate documentation
cargo doc --open
```

## Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Test specific crate
cargo test --package ferrotunnel-protocol
```

## Development Roadmap

See [ROADMAP.md](ROADMAP.md) for the complete 16-week development plan.

**Upcoming phases:**
- Phase 9: Final v1.0.0 Release

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed architecture documentation and workspace structure explanation.

## Documentation

- [ROADMAP.md](ROADMAP.md) - Development roadmap
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [Protocol Documentation](ferrotunnel-protocol/src/lib.rs) - Wire protocol details

## Security

Security is a top priority for FerroTunnel. If you discover a security vulnerability, please see our [Security Policy](SECURITY.md) for responsible disclosure guidelines.

**Quick contact:** shahmitul005@gmail.com

## Code of Conduct

This project adheres to the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for details.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

This project is in active development. Contributions are welcome!

Before contributing, please:
1. Read our [Contributing Guide](CONTRIBUTING.md)
2. Review our [Code of Conduct](CODE_OF_CONDUCT.md)
3. Review [ARCHITECTURE.md](ARCHITECTURE.md) to understand the project structure
4. Check the [ROADMAP.md](ROADMAP.md) to see what's being worked on

For security issues, see [SECURITY.md](SECURITY.md).
