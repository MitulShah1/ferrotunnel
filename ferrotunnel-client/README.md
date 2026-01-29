# ferrotunnel-client

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel-client)](https://crates.io/crates/ferrotunnel-client)

CLI client binary for [FerroTunnel](https://github.com/MitulShah1/ferrotunnel).

## Installation

```bash
cargo install ferrotunnel-client
```

## Usage

```bash
ferrotunnel-client \
  --server tunnel.example.com:7835 \
  --token my-secret-token \
  --local-addr 127.0.0.1:8000
```

## Options

| Option | Env Variable | Default | Description |
|--------|--------------|---------|-------------|
| `--server` | `FERROTUNNEL_SERVER` | (required) | Server address (host:port) |
| `--token` | `FERROTUNNEL_TOKEN` | (required) | Authentication token |
| `--local-addr` | - | `127.0.0.1:8000` | Local service to forward to |
| `--dashboard-port` | - | `4040` | Dashboard UI port |
| `--no-dashboard` | - | `false` | Disable dashboard |
| `--log-level` | `RUST_LOG` | `info` | Log level |

## Example

Start a local web server and tunnel it:

```bash
# Terminal 1: Start local service
python3 -m http.server 8000

# Terminal 2: Start tunnel client
ferrotunnel-client --server localhost:7835 --token secret --local-addr 127.0.0.1:8000
```

## Library Usage

For embedding in your application, use the main `ferrotunnel` crate instead:

```rust
use ferrotunnel::Client;

let mut client = Client::builder()
    .server_addr("tunnel.example.com:7835")
    .token("secret")
    .local_addr("127.0.0.1:8000")
    .build()?;

client.start().await?;
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
