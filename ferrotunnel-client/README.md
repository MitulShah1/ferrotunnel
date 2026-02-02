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
| `--server` | `FERROTUNNEL_SERVER` | (required) | FerroTunnel Server address (`host:port`) |
| `--token` | `FERROTUNNEL_TOKEN` | (required) | Authentication token |
| `--local-addr` | `FERROTUNNEL_LOCAL_ADDR` | `127.0.0.1:8000` | Local service to forward to |
| `--dashboard-port` | `FERROTUNNEL_DASHBOARD_PORT` | `4040` | Dashboard port |
| `--no-dashboard` | - | `false` | Disable the web dashboard |
| `--observability` | `FERROTUNNEL_OBSERVABILITY` | `false` | Enable metrics and tracing (disabled by default for lower latency) |
| `--log-level` | `RUST_LOG` | `info` | Log level (`trace`, `debug`, `info`, `warn`, `error`) |
| `--tls` | `FERROTUNNEL_TLS` | `false` | Enable TLS for server connection |
| `--tls-skip-verify` | `FERROTUNNEL_TLS_SKIP_VERIFY` | `false` | Skip TLS certificate verification (insecure) |
| `--tls-ca` | `FERROTUNNEL_TLS_CA` | - | Path to CA certificate for verification |
| `--tls-server-name` | `FERROTUNNEL_TLS_SERVER_NAME` | - | SNI hostname for TLS verification |
| `--tls-cert` | `FERROTUNNEL_TLS_CERT` | - | Path to client certificate file (mTLS) |
| `--tls-key` | `FERROTUNNEL_TLS_KEY` | - | Path to client private key file (mTLS) |

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
