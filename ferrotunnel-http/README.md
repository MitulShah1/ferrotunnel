# ferrotunnel-http

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel-http)](https://crates.io/crates/ferrotunnel-http)
[![Documentation](https://docs.rs/ferrotunnel-http/badge.svg)](https://docs.rs/ferrotunnel-http)

HTTP ingress and proxy for [FerroTunnel](https://github.com/MitulShah1/ferrotunnel).

## Overview

This crate provides HTTP handling for the tunnel:

- `HttpIngress` - Server-side HTTP listener that routes requests through tunnels
- `HttpProxy` - Client-side proxy that forwards tunneled requests to local services

## Components

### HttpIngress (Server)
Hyper-based HTTP server that:
- Listens for incoming HTTP requests
- Routes requests to connected tunnel clients
- Supports HTTP/1.1 and WebSocket upgrades

### HttpProxy (Client)
Handles tunneled HTTP requests by:
- Receiving virtual streams from the multiplexer
- Connecting to local services
- Proxying data bidirectionally

## Usage

```rust
use ferrotunnel_http::{HttpIngress, HttpProxy};

// Server-side: Create ingress
let ingress = HttpIngress::new("0.0.0.0:8080".parse()?, sessions, registry);
ingress.start().await?;

// Client-side: Create proxy
let proxy = HttpProxy::new("127.0.0.1:3000".into());
proxy.handle_stream(virtual_stream);
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
