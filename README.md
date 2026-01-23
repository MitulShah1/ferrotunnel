# FerroTunnel 🦀

[![CI](https://github.com/MitulShah1/ferrotunnel/workflows/CI/badge.svg)](https://github.com/MitulShah1/ferrotunnel/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/ferrotunnel)](https://crates.io/crates/ferrotunnel)
[![Documentation](https://docs.rs/ferrotunnel/badge.svg)](https://docs.rs/ferrotunnel)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Security](https://img.shields.io/badge/security-policy-green)](SECURITY.md)

**Wire Protocol Foundation for Reverse Tunneling**

FerroTunnel is a Rust-based reverse tunnel implementation. This repository contains the **Phase 1 foundation**: the wire protocol and core types.

## Current Status: Phase 1 ✅

**What's implemented:**
- ✅ Complete wire protocol (`ferrotunnel-protocol`)
- ✅ Frame types and codec with length-prefixed bincode encoding
- ✅ Common error types (`ferrotunnel-common`)
- ✅ Comprehensive unit tests (10+ tests)
- ✅ Full documentation

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
- Phase 2: Basic Tunnel (client-server communication)
- Phase 3: HTTP Proxying
- Phase 4: Library API (embeddable)
- Phase 5: Plugin System
- Phase 6: Observability Dashboard
- Phase 7-8: Production Hardening & Release

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed architecture documentation and workspace structure explanation.

## Documentation

- [ROADMAP.md](ROADMAP.md) - Development roadmap
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [Protocol Documentation](ferrotunnel-protocol/src/lib.rs) - Wire protocol details

## Security

Security is a top priority for FerroTunnel. If you discover a security vulnerability, please see our [Security Policy](SECURITY.md) for responsible disclosure guidelines.

**Quick contact:** security@ferrotunnel.dev

## Code of Conduct

This project adheres to the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for details.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

This project is in early development (Phase 1). Contributions are welcome!

Before contributing, please:
1. Read our [Code of Conduct](CODE_OF_CONDUCT.md)
2. Review [ARCHITECTURE.md](ARCHITECTURE.md) to understand the project structure
3. Check the [ROADMAP.md](ROADMAP.md) to see what's being worked on

For security issues, see [SECURITY.md](SECURITY.md).
