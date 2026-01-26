# FerroTunnel Protocol Fuzzing

This directory contains fuzz targets for the FerroTunnel protocol codec.

## Prerequisites

```bash
# Install cargo-fuzz (requires nightly)
cargo install cargo-fuzz
rustup install nightly
```

## Running Fuzz Tests

```bash
cd ferrotunnel-protocol

# Run codec decoder fuzzing
cargo +nightly fuzz run codec_decode

# Run frame validation fuzzing
cargo +nightly fuzz run frame_validation

# Run with time limit (24 hours)
cargo +nightly fuzz run codec_decode -- -max_total_time=86400

# Run with specific number of runs
cargo +nightly fuzz run codec_decode -- -runs=1000000
```

## Crash Reproduction

If a crash is found, reproduce it with:

```bash
cargo +nightly fuzz run codec_decode artifacts/codec_decode/<crash_file>
```

## Coverage

Generate coverage report:

```bash
cargo +nightly fuzz coverage codec_decode
```

## Targets

- `codec_decode`: Tests the binary protocol decoder with arbitrary bytes
- `frame_validation`: Tests frame validation after successful decode
