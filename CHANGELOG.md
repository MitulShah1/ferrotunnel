# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.3] - 2026-02-16

### Added

#### HTTP/2 Support
- **HTTP/2 ingress**: Server-side ingress now supports both HTTP/1.1 and HTTP/2 via automatic protocol detection using `hyper-util`'s `AutoBuilder`
- **HTTP/2 protocol variant**: Added `HTTP2` variant to the `Protocol` enum for future protocol-specific handling
- **Connection-close error filtering**: Added helper function to reduce log noise from benign connection close errors

#### Connection Pooling
- **Connection pool module**: New `pool` module (`ferrotunnel-http/src/pool.rs`) for efficient connection reuse
- **HTTP/1.1 pooling**: Idle HTTP/1.1 connections are stored in a LIFO queue (VecDeque) for cache warmth, with configurable limits (default: 32 per host, 90s timeout)
- **HTTP/2 multiplexing**: Single shared HTTP/2 connection per target with automatic clone-cheap multiplexing
- **Background eviction**: Automatic cleanup of expired idle connections every 30 seconds
- **Pool configuration**: `PoolConfig` struct with `max_idle_per_host`, `idle_timeout`, and `prefer_h2` options
- **`HttpProxy::with_pool_config()`**: New constructor for custom pool configuration

### Changed

#### Performance
- **Client proxy connection reuse**: `LocalProxyService` now acquires connections from the pool instead of creating new TCP connections per request, significantly reducing connection overhead
- **Connection lifecycle management**: Connections are returned to the pool after successful requests, but not for upgraded (WebSocket) connections or failed requests

#### Dependencies
- **hyper**: Added `http2` feature flag
- **hyper-util**: Added `server-auto` and `tokio` features for HTTP/2 auto-detection
- **thiserror**: Added for connection pool error types

### Fixed
- **Test compatibility**: Connection pool constructor now checks for tokio runtime availability before spawning background tasks, preventing test failures

## [1.0.2] - 2026-02-11

### Added

#### WebSocket Tunneling
- **Full WebSocket tunnel support**: Transparent WebSocket upgrade handling through the tunnel — real-time applications (chat, dashboards, gaming) now work out of the box
- **Automatic upgrade detection**: HTTP ingress detects `Connection: Upgrade` + `Upgrade: websocket` headers and opens streams with `Protocol::WebSocket`
- **Bidirectional bridging**: After the 101 handshake, upgraded connections are bridged with zero-copy `copy_bidirectional` for minimal overhead
- **End-to-end integration tests**: Two new WebSocket integration tests (`test_websocket_upgrade_through_tunnel`, `test_websocket_raw_upgrade_101`)

#### Graceful Shutdown
- **CLI signal handling**: Both `ferrotunnel server` and `ferrotunnel client` now handle Ctrl-C / SIGTERM gracefully, logging shutdown and cleaning up resources before exit
- **Server shutdown**: Server `tokio::select!` races all services against `ctrl_c()` for clean process termination
- **Client shutdown**: Client reconnection loop exits cleanly on signal, calling `shutdown_tracing()` before exit

### Changed

#### HTTP Proxy
- **Upgrade support**: HTTP/1 connections in both ingress and proxy now use `.with_upgrades()` for hyper upgrade protocol compatibility

## [1.0.1] - 2026-02-07

### Added

#### Installation
- **Homebrew Formula**: Introduce `brew install ferrotunnel` command for macOS users via [MitulShah1/homebrew-ferrotunnel](https://github.com/MitulShah1/homebrew-ferrotunnel) tap

#### Tunnel Routing
- **`--tunnel-id` CLI flag**: New `--tunnel-id` option for `ferrotunnel client` to set the tunnel ID used for HTTP Host-header routing (`FERROTUNNEL_TUNNEL_ID` env var supported)
- **`.tunnel_id()` builder method**: New method on `Client::builder()` for setting the tunnel ID when using the library API

### Fixed

#### Tunnel Routing
- **HTTP ingress routing**: Fixed "Tunnel not found" error when accessing tunnels via direct IP. The client now registers a `tunnel_id` that matches the Host header used by incoming HTTP requests

#### Docker Verification
- **Metrics Endpoint**: Fixed issue where the metrics server was not enabled by default in the Docker environment, causing verification scripts to report missing data.

### Improved

#### Docker Optimization
- **Optimized Docker image size**: Reduced from 34.8 MB to **13.4 MB** (61.6% smaller)
- **Faster build times**: Build time reduced from 6.5 minutes to **2.5 minutes** (62% faster)
- **Minimal base image**: Switched to Google's `distroless/cc-debian12` for minimal attack surface
- **Aggressive compiler optimizations**: Size-focused compile flags (`-C opt-level=z`, single codegen unit, panic=abort)
- **Enhanced caching**: cargo-chef for faster incremental builds
- **Binary stripping**: Comprehensive symbol removal for smaller binaries

#### Documentation
- Enhanced README with security comparisons and CVE analysis
- Updated ROADMAP to prioritize user adoption (WebSocket, HTTP/2, gRPC)
- Improved architecture diagrams

## [1.0.0] - 2026-02-05

### Highlights

FerroTunnel v1.0.0 is the first stable release.

### Features

#### Core Tunnel System
- **Protocol**: Custom binary protocol with length-prefixed frames, heartbeats, and multiplexing
- **Multiplexer**: Multiple concurrent virtual streams over a single TCP connection
- **Transport**: TCP and TLS 1.3 support with mutual TLS (mTLS) authentication
- **Reconnection**: Automatic reconnection with exponential backoff

#### HTTP & TCP Ingress
- **HTTP Ingress**: Hyper-based HTTP server for receiving public requests
- **TCP Ingress**: Raw TCP forwarding support
- **HTTP Proxy**: Client-side proxy to local services

#### Plugin System
- **Plugin Trait**: Async trait with `on_request` and `on_response` hooks
- **Plugin Registry**: Chain multiple plugins with control flow actions
- **Built-in Plugins**:
  - `LoggerPlugin` - Structured request logging
  - `TokenAuthPlugin` - Header-based token authentication
  - `RateLimitPlugin` - IP-based rate limiting
  - `CircuitBreakerPlugin` - Failure isolation

#### Observability
- **Prometheus Metrics**: Counters, gauges, and histograms
- **OpenTelemetry**: Distributed tracing with OTLP exporter support
- **Real-Time Dashboard**: Web UI at `http://localhost:4040` with:
  - Live traffic charts
  - Request/response inspector
  - Request replay functionality
  - SSE-based real-time updates

#### Unified CLI
- Single `ferrotunnel` binary with subcommands:
  - `ferrotunnel server` - Run the tunnel server
  - `ferrotunnel client` - Run the tunnel client
  - `ferrotunnel version` - Show version information
- Full TLS support via CLI flags and environment variables
- Optional observability (disabled by default for lower latency)

#### Library API
- **Embeddable**: Use as a library in your Rust applications
- **Builder Pattern**: `Client::builder()` and `Server::builder()` APIs
- **Lifecycle Management**: `start()`, `shutdown()`, `stop()` methods

#### Performance
- Zero-copy frame decoding with `Bytes`
- Batched I/O to reduce syscall overhead
- Lock-free concurrency with `DashMap`
- `mimalloc` allocator for improved performance
- TCP_NODELAY and optimized buffer sizes

#### Security
- TLS 1.3 with rustls
- Mutual TLS (mTLS) client authentication
- Token-based authentication
- Rate limiting and circuit breakers
- Protocol fuzzing test suite

#### Developer Tools
- `tools/loadgen` - Load generator for benchmarking
- `tools/soak` - Long-duration stability testing
- `tools/profiler` - CPU and memory profiling scripts

### Crates

| Crate | Description |
|-------|-------------|
| `ferrotunnel` | Main library with builder APIs |
| `ferrotunnel-cli` | Unified CLI binary |
| `ferrotunnel-core` | Core tunnel logic and transport |
| `ferrotunnel-protocol` | Wire protocol and codec |
| `ferrotunnel-http` | HTTP/TCP ingress and proxy |
| `ferrotunnel-plugin` | Plugin system and built-ins |
| `ferrotunnel-observability` | Metrics, tracing, and dashboard |
| `ferrotunnel-common` | Shared types and errors |

[Unreleased]: https://github.com/MitulShah1/ferrotunnel/compare/v1.0.2...HEAD
[1.0.2]: https://github.com/MitulShah1/ferrotunnel/releases/tag/v1.0.2
[1.0.1]: https://github.com/MitulShah1/ferrotunnel/releases/tag/v1.0.1
[1.0.0]: https://github.com/MitulShah1/ferrotunnel/releases/tag/v1.0.0
