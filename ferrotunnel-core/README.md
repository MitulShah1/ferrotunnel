# ferrotunnel-core

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel-core)](https://crates.io/crates/ferrotunnel-core)
[![Documentation](https://docs.rs/ferrotunnel-core/badge.svg)](https://docs.rs/ferrotunnel-core)

Core tunnel implementation for [FerroTunnel](https://github.com/MitulShah1/ferrotunnel).

## Overview

This crate provides the core tunnel logic:

- `TunnelClient` - Client-side tunnel connection
- `TunnelServer` - Server-side tunnel listener
- `Multiplexer` - Stream multiplexing over a single connection
- `Session` - Session management with heartbeat tracking
- `TlsTransport` - Encrypted transport support

## Components

### Tunnel
- **TunnelClient** - Connects to server, handles handshake, runs message loop
- **TunnelServer** - Listens for connections, authenticates clients, manages sessions

### Stream
- **Multiplexer** - Multiplexes virtual streams over one TCP connection
- **VirtualStream** - AsyncRead/AsyncWrite implementation for tunneled data

### Transport
- **TCP transport** - Standard TCP connection
- **TLS support** - Native TLS 1.3 encryption (rustls)

## Usage

```rust
use ferrotunnel_core::{TunnelClient, TunnelServer};

// Server
let server = TunnelServer::new("0.0.0.0:7835".parse()?, "secret".into());
server.run().await?;

// Client
let mut client = TunnelClient::new("localhost:7835".into(), "secret".into());
client.connect_and_run(|stream| async { /* handle stream */ }).await?;
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
