# ferrotunnel-http

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel-http)](https://crates.io/crates/ferrotunnel-http)
[![Documentation](https://docs.rs/ferrotunnel-http/badge.svg)](https://docs.rs/ferrotunnel-http)

HTTP ingress and proxy for [FerroTunnel](https://github.com/MitulShah1/ferrotunnel).

## Overview

This crate provides HTTP handling for the tunnel:

- `HttpIngress` - Server-side HTTP listener that routes requests through tunnels
- `TcpIngress` - Server-side TCP listener for raw socket tunneling
- `HttpProxy` - Client-side proxy that forwards tunneled requests to local services

## Components

### HttpIngress (Server)
Hyper-based HTTP server that:
- Listens for incoming HTTP requests
- Routes requests to connected tunnel clients
- Supports HTTP/1.1 and WebSocket upgrades

### TcpIngress (Server)
Socket listener that:
- Accepts raw TCP connections
- Routes traffic through the tunnel protocol
- Protocol-agnostic (supports Database, SSH, etc.)

### HttpProxy (Client)
Handles tunneled HTTP requests by:
- Receiving virtual streams from the multiplexer
- Connecting to local services
- Proxying data bidirectionally

## Usage

```rust
use ferrotunnel_http::{HttpIngress, TcpIngress, HttpProxy};

// Server-side: Create ingress
let ingress = HttpIngress::new("0.0.0.0:8080".parse()?, sessions.clone(), registry);
let tcp_ingress = TcpIngress::new("0.0.0.0:5000".parse()?, sessions);

tokio::try_join!(ingress.start(), tcp_ingress.start())?;

// Client-side: Create proxy
let proxy = HttpProxy::new("127.0.0.1:3000".into());
proxy.handle_stream(virtual_stream);
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
