# ferrotunnel

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel)](https://crates.io/crates/ferrotunnel)
[![Documentation](https://docs.rs/ferrotunnel/badge.svg)](https://docs.rs/ferrotunnel)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)

A production-ready, secure reverse tunnel system in Rust.

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
ferrotunnel = "0.9"
```

## Features

- 🔒 **Secure** - TLS encryption, token-based authentication
- ⚡ **Fast** - Built on Tokio for high-performance async I/O
- 🔌 **Protocol Support** - HTTP, HTTPS, WebSocket, gRPC, TCP
- 📊 **Observable** - Comprehensive logging and metrics
- 🛡️ **Resilient** - Automatic reconnection, heartbeat monitoring

## Documentation

See the [full documentation](https://docs.rs/ferrotunnel) for detailed usage.

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
