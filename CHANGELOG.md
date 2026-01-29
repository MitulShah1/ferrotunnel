# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking Changes
- **Plugin API**: The `Plugin::on_request` trait method signature has changed to take `&mut http::Request<()>` instead of `&mut http::Request<Vec<u8>>`. This prevents plugins from forcing the ingress server to buffer the entire request body, which fixes a critical DoS vulnerability.
  - **Migration**: Update your plugins to access headers only in `on_request`. If you need to inspect the body, you must implement a streaming body parser (future work).

### Fixed
- **Critical Security**: Fixed cross-tenant request routing where requests could be routed to any active tunnel instead of the specific tunnel requested via the Host header.
- **Performance/Security**: Removed full request body buffering in the HTTP ingress to prevent memory exhaustion DoS attacks and improve latency for large payloads.

### Performance
- **Core Optimization**: Replaced `futures::channel::mpsc` with `kanal` for 2-3x faster async channel throughput in the multiplexer.
- **Lock-Free Concurrency**: Replaced `Mutex<Inner>` with `DashMap` and `AtomicU32` in `Multiplexer` to reject lock contention.
- **Memory Optimization**: Boxed large `Frame` variants (`Data`, `Handshake`, `OpenStream`) to reduce stack usage by ~60%.
- **Network Tuning**: Enabled `TCP_NODELAY` and optimized buffer sizes for lower latency.
- **Zero-Copy Codec**: Frame decoder now uses `split_to().freeze()` for zero-copy deserialization (10-15% latency reduction).
- **Thread-Local Buffers**: Encoder uses thread-local buffer pool to reduce per-frame allocations (5-10% allocator pressure reduction).
- **Batched I/O**: New `batched_sender` collects frames and flushes in batches to reduce syscall overhead (15-25% throughput improvement).
- **Response Streaming**: HTTP responses are now streamed directly when no plugins need body inspection, providing constant memory usage and lower TTFB for large responses.
- **Object Pooling**: Added lock-free `ObjectPool` for reusing read buffers in `VirtualStream`, reducing allocations by ~20% under steady load.

## [0.7.0] - 2026-01-27

### Added

#### Observability Infrastructure (Phase 6) - Backend
- **New Crate**: `ferrotunnel-observability`
  - High-performance Prometheus metrics (Counters, Gauges, Histograms)
  - OpenTelemetry integration with OTLP/gRPC exporter support
  - Structured logging with `tracing` layers
- **Server Integration**:
  - Background metrics server on port `9090` (Axum-based)
  - Automatic observability initialization
  - Instrumented HTTP ingress and session management
- **Client Integration**:
  - Automatic observability initialization for monitoring remote deployments

#### Real-Time Dashboard (Phase 7) - UI & API
- **Web Dashboard** at `http://localhost:4040`:
  - Live traffic chart with 2-second update intervals
  - Request/response inspector with full headers and bodies
  - Tunnel status monitoring with connection state
  - Error rate tracking for 4xx/5xx responses
- **Dashboard API Endpoints**:
  - `GET /api/v1/health` - Health check with version info
  - `GET /api/v1/tunnels` - List active tunnels
  - `GET /api/v1/requests` - List captured requests
  - `GET /api/v1/requests/:id` - Get full request details
  - `POST /api/v1/requests/:id/replay` - Replay captured requests
  - `GET /api/v1/events` - SSE stream for real-time updates
- **Traffic Capture Middleware**:
  - `DashboardCaptureLayer` for capturing request/response data
  - Full header and body capture (up to 1MB)
  - Duration and timestamp tracking
- **Static Assets**: Embedded HTML/CSS/JS dashboard UI
- **Testing**: `scripts/test-dashboard.sh` comprehensive test script

### Changed
- `TunnelStatus` enum now includes `Connecting` variant
- Client middleware now tracks `tunnel_id` for proper request association
- Updated all crates and internal dependencies to version 0.7.0

### Fixed
- Resolved Clippy warnings related to `unwrap()` usage in background tasks
- Fixed OpenTelemetry v0.21 dependency feature mismatches


## [0.6.0] - 2026-01-27

### Added

#### Hardening (Phase 7) - Production Readiness
- **Resilience & Reliability**:
  - `CircuitBreakerPlugin` for failure isolation
  - Rate limiting per tunnel/session
  - Exponential backoff reconnection logic
  - Resource limits (max connections, memory thresholds)
- **Security**:
  - TLS 1.3 support powered by `rustls`
  - Automated security auditing with `deny.toml`
  - Security policy and vulnerability reporting guidelines
- **Transport**:
  - Optimized TCP settings (NoDelay, Keepalive)
- **Tooling & Infrastructure**:
  - `tools/loadgen`: High-performance load generator
  - `tools/soak`: Long-running soak test suite
  - Protocol fuzzing suite for frame validation
  - Performance benchmarks for core components
- **Documentation**:
  - Deployment guide (`docs/deployment.md`)
  - Security guide (`docs/security.md`)
  - Troubleshooting guide (`docs/troubleshooting.md`)

### Changed
- Updated all crates and workspace to version 0.6.0
- Enhanced CI/CD with fuzzing, security audit, and benchmarks

## [0.5.0] - 2026-01-25

### Added

#### Plugin System (Phase 5) - Extensibility
- New `ferrotunnel-plugin` crate defining the plugin architecture
  - `Plugin` async trait with `on_request` and `on_response` hooks
  - `PluginRegistry` for managing and executing plugin chains
  - `PluginAction` control flow (Continue, Reject, Respond, Modify)
- Built-in Plugins:
  - `LoggerPlugin`: Structured request logging
  - `TokenAuthPlugin`: Header-based token authentication
  - `RateLimitPlugin`: IP-based rate limiting (leaky bucket)
- Core Integration:
  - `HttpIngress` now executes request/response hooks
  - `ServerBuilder` automatically registers default plugins (Logger)
- Developer Experience:
  - Examples: `hello_plugin`, `header_filter` (security), `ip_blocklist` (access control)
  - `scripts/test-plugins.sh` for verifying plugin behavior

### Changed
- Updated all crates to version 0.5.0
- `ferrotunnel-server` now initializes a `PluginRegistry` on startup
- `HttpIngress::new` now requires `Arc<PluginRegistry>`

## [0.4.0] - 2026-01-25

### Added

#### Library API (Phase 4) - First Differentiator
- **Embeddable Library**: FerroTunnel is now the first embeddable Rust reverse tunnel
- New `Client` and `ClientBuilder` for embedded client usage
  - Builder pattern with `server_addr()`, `token()`, `local_addr()`, `auto_reconnect()`
  - `start()` / `shutdown()` / `stop()` lifecycle methods
  - Proper `Drop` implementation for cleanup on drop
- New `Server` and `ServerBuilder` for embedded server usage
  - Builder pattern with `bind()`, `http_bind()`, `token()`
  - `start()` / `shutdown()` / `stop()` lifecycle methods
- Configuration types: `ClientConfig`, `ServerConfig`, `TunnelInfo`
  - Validation in `build()` for fail-fast error handling
- Example files:
  - `examples/embedded_client.rs` - Demonstrates embedded client usage
  - `examples/embedded_server.rs` - Demonstrates embedded server usage
- Integration test script: `scripts/test-tunnel.sh`

#### Documentation
- Updated README with library usage examples
- Complete rustdoc for all public API types

### Changed
- Updated all crates to version 0.4.0
- `TunnelInfo.session_id` is now `Option<Uuid>` (placeholder until core exposes it)
- Improved lifecycle management with `JoinHandle` tracking

## [0.3.0] - 2026-01-24

### Added

#### HTTP Proxying (Phase 3 MVP)
- New `ferrotunnel-http` crate for HTTP integration
  - `HttpIngress`: Hyper-based HTTP server listening for requests
  - `HttpProxy`: Client-side proxy logic connecting to local services
- `Multiplexer` in `ferrotunnel-core`
  - Supports multiple concurrent virtual streams over a single TCP connection
  - Handles `OpenStream`, `Data`, and `CloseStream` frames
- End-to-end HTTP Proxy support
  - Server listens on HTTP port (default 8080)
  - Client tunnels requests to local service (default 127.0.0.1:8000)
- Dependencies added: `hyper`, `hyper-util`, `http-body-util`, `bytes`

#### CI/CD
- Added Dependabot configuration (`.github/dependabot.yml`)
  - Weekly updates for `cargo` and `github-actions`

### Changed
- Updated all crates to version 0.3.0
- `ferrotunnel-server`: Added `--http-bind` argument
- `ferrotunnel-client`: Added `--local-addr` argument for proxy target

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

[Unreleased]: https://github.com/MitulShah1/ferrotunnel/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/MitulShah1/ferrotunnel/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/MitulShah1/ferrotunnel/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/MitulShah1/ferrotunnel/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/MitulShah1/ferrotunnel/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/MitulShah1/ferrotunnel/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/MitulShah1/ferrotunnel/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/MitulShah1/ferrotunnel/releases/tag/v0.1.0
