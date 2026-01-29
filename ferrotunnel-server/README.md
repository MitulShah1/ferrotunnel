# ferrotunnel-server

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel-server)](https://crates.io/crates/ferrotunnel-server)

CLI server binary for [FerroTunnel](https://github.com/MitulShah1/ferrotunnel).

## Installation

```bash
cargo install ferrotunnel-server
```

## Usage

```bash
ferrotunnel-server \
  --bind 0.0.0.0:7835 \
  --http-bind 0.0.0.0:8080 \
  --token my-secret-token
```

## Options

| Option | Env Variable | Default | Description |
|--------|--------------|---------|-------------|
| `--bind` | `FERROTUNNEL_BIND` | `0.0.0.0:7835` | Tunnel control plane address |
| `--http-bind` | `FERROTUNNEL_HTTP_BIND` | `0.0.0.0:8080` | HTTP ingress address |
| `--token` | `FERROTUNNEL_TOKEN` | (required) | Authentication token |
| `--log-level` | `RUST_LOG` | `info` | Log level |
| `--metrics-bind` | `FERROTUNNEL_METRICS_BIND` | `0.0.0.0:9090` | Prometheus metrics address |

## Ports

- **7835** - Tunnel control plane (clients connect here)
- **8080** - HTTP ingress (public traffic enters here)
- **9090** - Prometheus metrics (configurable via `--metrics-bind`)

## Example

```bash
# Start the server
ferrotunnel-server --token my-secret-token

# Now clients can connect and HTTP traffic on :8080 is tunneled
```

## Library Usage

For embedding in your application, use the main `ferrotunnel` crate instead:

```rust
use ferrotunnel::Server;

let mut server = Server::builder()
    .bind("0.0.0.0:7835".parse()?)
    .http_bind("0.0.0.0:8080".parse()?)
    .token("secret")
    .build()?;

server.start().await?;
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
