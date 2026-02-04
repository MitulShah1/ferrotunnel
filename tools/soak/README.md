# FerroTunnel Soak Testing Tool

Soak testing tool for verifying long-term stability of FerroTunnel.

## Features

- **Continuous Traffic**: Generates sustained connection attempts and data transfer.
- **Resource Monitoring**: Tracks RSS memory usage to detect leaks.
- **Concurrency Control**: Configurable number of parallel tunnel simulations.
- **Metrics Logging**: JSONL output for post-test analysis.
- **Graceful Shutdown**: Automatically stops after a set duration.

## Usage

### 1. Start the Server
The soak tool is a client that connects to a running FerroTunnel server. You must start the server first:

```bash
cargo run --release --bin ferrotunnel -- server --token my-secret-token --bind 127.0.0.1:7835
```

### 2. Run the Soak Test

In a new terminal:

```bash
# Run for 1 hour with 50 concurrent connections
cargo run -p ferrotunnel-soak -- \
    --tunnel-addr 127.0.0.1:7835 \
    --token my-secret-token \
    --target 127.0.0.1:9999 \
    --concurrency 50 \
    --duration 60
```

> **Note:** If you see `Connection refused` errors, it means the tool cannot reach the server at `127.0.0.1:7835`. Ensure the server is running and accessible.

### Arguments

- `--target <TARGET>`: Address of the target application (default: `127.0.0.1:9999`)
- `--tunnel-addr <TUNNEL_ADDR>`: Address of the FerroTunnel server (default: `127.0.0.1:7835`)
- `--token <TOKEN>`: Authentication token (default: `my-secret-token`)
- `--concurrency <CONCURRENCY>`: Number of simultaneous tunnels (default: `10`)
- `--duration <DURATION>`: Test duration in minutes (0 = infinite) (default: `0`)
- `--output <OUTPUT>`: File to write metrics to (default: `soak_metrics.jsonl`)

## Analysis

The tool produces a `soak_metrics.jsonl` file. You can analyze this to look for:
1. **Memory Leaks**: Plot `rss_mb` over time. Linear growth indicates a leak.
2. **Error Rate**: Check `errors` count. It should remain 0 for a healthy system.
