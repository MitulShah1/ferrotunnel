# FerroTunnel Load Generator

A high-performance load generation tool designed to stress test FerroTunnel instances.

## Features

- **High Concurrency**: Uses Tokio to simulate thousands of concurrent clients.
- **Configurable Load**: Set request rate (RPS), connection count, and payload size.
- **Latency Analysis**: Measures P50, P90, P99 latency using HdrHistogram.
- **Protocol Support**: Can target both direct TCP and HTTP endpoints.

## Usage

```bash
# Run a baseline load test against a local server
cargo run -p ferrotunnel-loadgen -- \
    --mode baseline \
    --target 127.0.0.1:8080 \
    --concurrency 100 \
    --requests 1000
```

### Options

- `--mode <MODE>`: Test mode: `echo-server`, `echo-client`, or `baseline` (default: `baseline`).
- `--target <TARGET>`: Target address for client mode (default: `127.0.0.1:9999`).
- `--bind <BIND>`: Bind address for server mode (default: `127.0.0.1:9999`).
- `--concurrency <CONCURRENCY>`: Number of concurrent connections/streams (default: `100`).
- `--requests <REQUESTS>`: Number of requests per connection (default: `1000`).
- `--payload-size <PAYLOAD_SIZE>`: Payload size in bytes (default: `1024`).
- `--duration <DURATION>`: Test duration in seconds (0 = run until requests complete) (default: `0`).
