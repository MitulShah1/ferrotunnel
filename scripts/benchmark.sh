#!/bin/bash
# Benchmark runner script for FerroTunnel performance testing

set -e

BASELINE="${1:-main}"
BENCHMARKS="${2:-full_stack,latency,tcp_throughput}"

echo "========================================"
echo "FerroTunnel Performance Benchmarks"
echo "========================================"
echo "Baseline: $BASELINE"
echo "Benchmarks: $BENCHMARKS"
echo ""

# Build in release mode first
echo "Building benchmarks in release mode..."
cargo build --release --benches

# Run benchmarks
IFS=',' read -ra BENCH_ARRAY <<< "$BENCHMARKS"
for bench in "${BENCH_ARRAY[@]}"; do
    echo ""
    echo "Running benchmark: $bench"
    echo "----------------------------------------"

    if [ "$BASELINE" = "save" ]; then
        echo "Saving baseline..."
        cargo bench --bench "$bench" -- --save-baseline main
    elif [ "$BASELINE" != "main" ]; then
        echo "Comparing against baseline: $BASELINE"
        cargo bench --bench "$bench" -- --baseline "$BASELINE"
    else
        cargo bench --bench "$bench"
    fi
done

echo ""
echo "========================================"
echo "Benchmarks complete!"
echo "========================================"
echo ""
echo "Results saved in: target/criterion/"
echo "To compare with baseline, run:"
echo "  ./scripts/benchmark.sh <baseline-name>"
echo ""
echo "To save current as baseline:"
echo "  ./scripts/benchmark.sh save"
