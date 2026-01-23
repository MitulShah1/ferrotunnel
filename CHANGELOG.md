# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-01-23

### Added

#### Basic Tunnel Implementation
- New `ferrotunnel-core` crate implementing the core tunnel logic
  - TCP transport layer abstraction
  - Session management with thread-safe `DashMap` storage
  - Heartbeat mechanism for connection keep-alive
  - Handshake protocol with token authentication
- New CLI binaries
  - `ferrotunnel-server`: TCP listener with token-based auth
  - `ferrotunnel-client`: Tunneled client with automatic reconnection
- Structured logging with `tracing` and `tracing-subscriber`
- CLI argument parsing with `clap`

#### Project Infrastructure & Community
- Main `ferrotunnel` library crate with convenience re-exports
- `CODE_OF_CONDUCT.md` - Community guidelines based on Rust CoC
- `SECURITY.md` - Security policy and vulnerability reporting
- Automated publishing workflow via GitHub Actions
- Comprehensive linting and formatting setup (rustfmt, clippy)
- Makefile with development commands
- VS Code integration settings
- CI/CD workflows for testing and publishing

### Changed
- **BREAKING**: Restructured to tokio-style flat layout (removed `crates/` folder)
- Moved `ferrotunnel-common` and `ferrotunnel-protocol` to root level
- Updated all documentation to reflect new structure

## [0.1.0] - 2026-01-23

### Added

#### Project Infrastructure
- Initial workspace structure with two crates:
  - `ferrotunnel-common` - Common utilities and error types
  - `ferrotunnel-protocol` - Wire protocol definitions and codec
- Comprehensive linting and formatting setup:
  - `rustfmt.toml` for code formatting (100 char line width)
  - `clippy.toml` for linting configuration
  - Workspace-wide lint rules in `Cargo.toml`
  - Makefile with convenient development commands
- VS Code integration with `.vscode/settings.json`
- GitHub Actions CI/CD workflow (`.github/workflows/ci.yml`)
  - Format checking
  - Clippy linting with `-D warnings`
  - Tests on Linux, macOS, and Windows
  - Testing with stable and beta Rust toolchains

#### ferrotunnel-common
- `TunnelError` enum with comprehensive error variants
- Error type conversions (from `std::io::Error`, `bincode::Error`)
- Result type alias for ergonomic error handling

#### ferrotunnel-protocol
- `Frame` enum defining all protocol frames:
  - Control frames: `Handshake`, `HandshakeAck`, `Register`, `RegisterAck`
  - Stream frames: `OpenStream`, `StreamAck`, `Data`, `CloseStream`
  - Keepalive: `Heartbeat`, `HeartbeatAck`
  - Error handling: `Error`
  - Plugin support: `PluginData`
- Status enums: `HandshakeStatus`, `RegisterStatus`, `StreamStatus`
- Protocol types: `Protocol`, `CloseReason`, `ErrorCode`
- `TunnelCodec` for length-prefixed frame encoding/decoding
- Frame size validation (4MB max)
- Full test coverage for frame serialization and codec

#### Documentation
- `README.md` with project overview
- `ARCHITECTURE.md` describing system design
- `ROADMAP.md` with development phases
- `LICENSE` (MIT OR Apache-2.0)
- CHANGELOG.md (this file)

### Configuration
- Workspace dependencies managed centrally
- Profile optimizations for release builds:
  - LTO enabled
  - Code generation units set to 1
  - Binary stripping enabled
- Strict lint configuration:
  - `unsafe_code = "forbid"`
  - Pedantic clippy lints enabled
  - Warnings for `unwrap`, `expect`, `todo`, `dbg!` macros

### Testing
- 9 passing unit tests across both crates
- Codec round-trip tests
- Frame serialization tests
- Partial frame handling tests

[Unreleased]: https://github.com/MitulShah1/ferrotunnel/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/MitulShah1/ferrotunnel/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MitulShah1/ferrotunnel/releases/tag/v0.1.0
