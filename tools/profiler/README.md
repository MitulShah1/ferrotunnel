# FerroTunnel Profiler Tools

Scripts and utilities for profiling FerroTunnel performance.

## Prerequisites

Install profiling tools:

```bash
# Linux (Ubuntu/Debian)
sudo apt-get install linux-tools-generic perf

# Rust profiling tools
cargo install flamegraph
cargo install cargo-flamegraph
```

## Scripts

### `profile-server.sh`

Profile the tunnel server under load:

```bash
./tools/profiler/profile-server.sh
```

Generates a flamegraph of server CPU usage.

### `profile-codec.sh`

Profile the protocol codec (encode/decode):

```bash
./tools/profiler/profile-codec.sh
```

### `profile-memory.sh`

Profile memory allocations:

```bash
./tools/profiler/profile-memory.sh
```

## Output

All profiles are saved to `./target/profiles/`:

- `*.svg` - Flamegraph visualizations
- `*.data` - Raw perf data
- `*.txt` - Summary reports

## Quick Profiling

For quick CPU profiling without scripts:

```bash
# Build with debug symbols
cargo build --release --workspace

# Run with flamegraph
cargo flamegraph --bin ferrotunnel-server -- --config config.toml

# Or use perf directly
perf record -g target/release/ferrotunnel-server --config config.toml
perf report
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PROFILE_DURATION` | How long to profile (seconds) | 30 |
| `PROFILE_OUTPUT` | Output directory | `./target/profiles` |
| `SAMPLE_FREQUENCY` | perf sample frequency (Hz) | 99 |
