# FerroTunnel Load Generator

A high-performance load generation tool designed to stress test FerroTunnel instances.

## Features

- **High Concurrency**: Uses Tokio to simulate thousands of concurrent clients.
- **Configurable Load**: Set request rate (RPS), connection count, and payload size.
- **Latency Analysis**: Measures P50, P90, P99 latency using HdrHistogram.
- **Protocol Support**: Can target both direct TCP and HTTP endpoints.

## Usage

```bash
# Run a load test against a local server
cargo run -p ferrotunnel-loadgen -- \
    --target http://localhost:8080 \
    --connections 100 \
    --rate 1000 \
    --duration 30s
```

### Arguments

- `--target`: URL or address to stress test.
- `--connections`: Number of concurrent connections to maintain.
- `--rate`: Target requests per second (RPS).
- `--duration`: Duration of the test (e.g., `30s`, `5m`).
- `--payload-size`: Size of the payload in bytes (default: 0).
