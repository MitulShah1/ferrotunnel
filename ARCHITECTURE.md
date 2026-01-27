# FerroTunnel Architecture

## Project Structure

FerroTunnel follows the **tokio-style workspace pattern** - the industry standard for multi-crate Rust projects.

### Current Structure (Phase 8)

```
ferrotunnel/
├── Cargo.toml                  # Workspace configuration
├── ROADMAP.md                  # Development plan
├── README.md
├── ARCHITECTURE.md
├── CHANGELOG.md
├── LICENSE
├── ferrotunnel/                # Main library (Facade & Builders)
│   ├── src/
│   │   ├── lib.rs              # Re-exports & prelude
│   │   ├── client.rs           # Client Builder API
│   │   ├── server.rs           # Server Builder API
│   │   └── config.rs           # Configuration types
├── ferrotunnel-core/           # Core tunnel logic
│   ├── src/
│   │   ├── tunnel/             # Connection management
│   │   ├── stream/             # Multiplexing
│   │   ├── transport/          # Transport layer (TCP/TLS)
│   │   ├── auth.rs             # Token-based authentication
│   │   ├── rate_limit.rs       # Rate limiting logic
│   │   ├── reconnect.rs        # Reconnect with backoff
│   │   └── resource_limits.rs  # Resource monitoring
├── ferrotunnel-http/           # HTTP handling
│   ├── src/
│   │   ├── ingress.rs          # HTTP Ingress
│   │   └── proxy.rs            # HTTP/WS Proxy
├── ferrotunnel-protocol/       # Wire protocol & codec
│   └── src/
├── ferrotunnel-plugin/         # Plugin system
│   ├── src/
│   │   ├── traits.rs
│   │   ├── registry.rs
│   │   └── builtin/
├── ferrotunnel-observability/  # Phase 6 & 7: Monitoring & Dashboard
│   ├── src/
│   │   ├── metrics.rs          # Prometheus metrics
│   │   ├── tracing.rs          # OpenTelemetry
│   │   ├── dashboard/          # Real-time Dashboard
│   │   │   ├── server.rs       # Dashboard server (Axum + SSE)
│   │   │   ├── api.rs          # REST API
│   │   │   └── static/         # Embedded Web UI
│   │   └── lib.rs              # Initialization API
├── ferrotunnel-common/         # Shared types & errors
│   └── src/
├── ferrotunnel-client/         # Client binary
├── ferrotunnel-server/         # Server binary
├── examples/                   # Embeddable examples
└── tools/                      # Testing & Diagnostic tools
    ├── loadgen/                # Load generator
    └── soak/                   # Soak tester
```

**Key improvements over nested `crates/` folder:**
- ✅ Matches industry standards (tokio, serde, clap, axum)
- ✅ Each crate is a top-level folder (easier navigation)
- ✅ Main `ferrotunnel` crate provides unified API
- ✅ Clear separation without nesting confusion

## Future Structure (v1.0.0)

Complete structure after all phases - **for implementation reference**:

```
ferrotunnel/
├── Cargo.toml                  # Workspace root
├── ROADMAP.md
├── README.md
├── ARCHITECTURE.md
├── CHANGELOG.md
├── LICENSE
│
├── ferrotunnel/                # ✅ Phase 1: Main library (public API)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # Re-exports & prelude
│       ├── client.rs           # Phase 4: Client builder
│       ├── server.rs           # Phase 4: Server builder
│       └── config.rs           # Phase 4: Configuration
│
├── ferrotunnel-protocol/       # ✅ Phase 1: Wire protocol
│   └── src/
│       ├── frame.rs            # Message types
│       ├── codec.rs            # Encoder/decoder
│       └── constants.rs        # Protocol constants
│
├── ferrotunnel-common/         # ✅ Phase 1: Shared utilities
│   └── src/
│       └── error.rs            # Error types
│
├── ferrotunnel-core/           # Phase 2: Core tunnel logic
│   └── src/
│       ├── tunnel/
│       │   ├── client.rs       # Client implementation
│       │   ├── server.rs       # Server implementation
│       │   └── session.rs      # Session management
│       ├── stream/
│       │   ├── multiplexer.rs  # Stream multiplexing
│       │   └── router.rs       # Request routing
│       ├── transport/
│       │   ├── tcp.rs          # TCP transport
│       │   ├── tls.rs          # Phase 8: TLS support
│       │   └── quic.rs         # Future: QUIC
│       └── reconnect.rs        # Phase 8: Auto-reconnect
│
├── ferrotunnel-http/           # Phase 3: HTTP handling
│   └── src/
│       ├── ingress.rs          # HTTP ingress
│       ├── proxy.rs            # Reverse proxy
│       └── upgrade.rs          # WebSocket upgrades
│
├── ferrotunnel-plugin/         # ✅ Phase 5: Plugin system
│   └── src/
│       ├── traits.rs           # Plugin traits
│       ├── registry.rs         # Plugin registry
│       ├── context.rs          # Plugin context
│       └── builtin/
│           ├── logger.rs       # Logging plugin
│           ├── auth.rs         # Auth plugin
│           └── ratelimit.rs    # Rate limiting
│
├── ferrotunnel-observability/  # Phase 6: Monitoring
│   └── src/
│       ├── metrics.rs          # Prometheus metrics
│       ├── tracing.rs          # OpenTelemetry
│       └── dashboard/
│           ├── server.rs       # Dashboard server
│           ├── api.rs          # REST API
│           └── static/         # Web UI
│
├── ferrotunnel-client/         # Phase 2: Client binary
│   └── src/
│       └── main.rs
│
├── ferrotunnel-server/         # Phase 2: Server binary
│   └── src/
│       └── main.rs
│
├── examples/                   # Phase 4+: Usage examples
│   ├── embedded_client.rs
│   ├── embedded_server.rs
│   └── custom_plugin.rs
│
└── tests/                      # Phase 3+: Integration tests
    └── integration/
```

## Implementation Order

1. ✅ **Phase 1**: `ferrotunnel`, `protocol`, `common`
2. ✅ **Phase 2**: `core` + client/server binaries
3. ✅ **Phase 3**: `http` handling
4. ✅ **Phase 4**: Complete main library API
5. ✅ **Phase 5**: `plugin` system
6. ✅ **Phase 6**: `observability` infrastructure (Backend)
7. ✅ **Phase 7**: `observability` dashboard (UI + API)
8. ✅ **Phase 8**: Hardening & Security
9. **Phase 9**: v1.0.0 release

## Why This Structure?

### Tokio-Style Workspace

This matches the structure used by major Rust projects:

- **tokio**: `tokio/`, `tokio-util/`, `tokio-stream/`, etc.
- **serde**: `serde/`, `serde_derive/`, `serde_json/`, etc.
- **clap**: `clap/`, `clap_derive/`, etc.
- **axum**: `axum/`, `axum-core/`, `axum-extra/`, etc.

### Benefits

✅ **Clear Navigation**: No nested `crates/` folder
✅ **Industry Standard**: Familiar to Rust developers
✅ **Independent Publishing**: Each crate publishable separately
✅ **Shared Dependencies**: Workspace manages versions
✅ **Better Caching**: Cargo caches builds efficiently
✅ **Main Crate**: `ferrotunnel` provides unified API

### Compared to Single Crate

**Single crate** (simple projects):
```
my-project/
├── Cargo.toml
└── src/
    └── lib.rs
```

**Workspace** (multi-component projects like FerroTunnel):
```
ferrotunnel/
├── Cargo.toml              # Workspace
├── ferrotunnel/            # Main library
├── ferrotunnel-protocol/   # Protocol
└── ferrotunnel-common/     # Shared
```

FerroTunnel needs a workspace because:
- Multiple publishable crates
- Client + server binaries
- Plugin system requires separate crate
- Clear separation of concerns

## Crate Descriptions

### `ferrotunnel` ✅

**Main library crate** - The primary entry point for using FerroTunnel as a library:
- **Builder API**: `Client::builder()` and `Server::builder()` for ergonomic configuration.
- **Facade**: Re-exports commonly used types from subcrates.
- **Prelude**: `ferrotunnel::prelude::*` for easy imports.

### `Library API` ✅

FerroTunnel is designed to be **embeddable**. You can include the `ferrotunnel` crate in your own Rust applications to create custom tunnel clients or servers.

**Example: Embedded Client**
```rust
use ferrotunnel::Client;

let client = Client::builder()
    .server_addr("tunnel.example.com:7835")
    .token("my-token")
    .build()?;
client.start().await?;
```

### `ferrotunnel-protocol` ✅

**Wire protocol** for tunnel communication:
- **12 frame types**: Handshake, Register, Data, etc.
- **Length-prefixed codec**: Efficient bincode encoding
- **Validation**: Max frame size (4MB default)
- **Zero-copy**: Uses `Bytes` for performance

### `ferrotunnel-common` ✅

**Shared types** and utilities:
- **Error types**: Comprehensive `TunnelError` enum
- **Result alias**: `Result<T>` for consistency
- **UUID handling**: Session and stream identifiers

### `ferrotunnel-core` ✅

**Core tunnel engine**:
- **Connection**: Manages the persistent control connection / Heartbeats.
- **Session**: Concept of a "tunnel session".
- **Multiplexer**: Handles multiple concurrent streams over one connection.

### `ferrotunnel-http` ✅

**HTTP Layer**:
- **Ingress**: Receives public HTTP requests and routes them to sessions.
- **Proxy**: Forwards requests from the client to localhost.

### `ferrotunnel-client` & `ferrotunnel-server` ✅

**Reference Implementations**:
- CLI binaries for running the tunnel and server standalone.
- Built on top of the library API.

### `ferrotunnel-observability` ✅

**Metrics, Tracing, and Dashboard** (Phase 6 & 7):
- **Dashboard server**: Axum-based server providing a real-time WebUI (port 4040) and SSE stream for live updates.
- **REST API**: Endpoints for inspecting tunnels, requests, and replaying traffic.
- **Prometheus Metrics**: High-performance counters and histograms on port 9090.
- **OpenTelemetry**: Distributed tracing support for request-level visibility.
- **Unified Init**: Convenience API for initializing observability in any binary.

### `tools/` ✅

**Diagnostic and Testing Suite** (Phase 8):
- **loadgen**: High-performance load generator for throughput testing.
- **soak**: Long-running suite for memory leak and stability detection.

## Building

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Build specific crate
cargo build --package ferrotunnel-protocol

# Run linting
make check

# Format code
make fmt

# Generate documentation
cargo doc --open
```

## Publishing

The workspace allows independent publishing:

```bash
# Publish in dependency order
cd ferrotunnel-common && cargo publish
cd ../ferrotunnel-protocol && cargo publish
cd ../ferrotunnel && cargo publish
```

Or use automated GitHub Actions (see `.github/workflows/publish.yml`).

## Development Workflow

1. **Make changes** to any crate
2. **Run checks**: `make check` (format + lint)
3. **Run tests**: `make test`
4. **Commit**: Changes pass CI automatically
5. **Release**: Tag version, CI publishes to crates.io

## References

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Tokio Architecture](https://github.com/tokio-rs/tokio)
- [Semantic Versioning](https://semver.org/)
