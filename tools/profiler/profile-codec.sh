#!/bin/bash
# Profile the FerroTunnel protocol codec
#
# Usage: ./tools/profiler/profile-codec.sh
#
# Runs the codec benchmarks with profiling enabled

set -e

OUTPUT_DIR="${PROFILE_OUTPUT:-./target/profiles}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║          FerroTunnel Codec Profiler                        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

mkdir -p "$OUTPUT_DIR"

# Build benchmarks with debug symbols
echo -e "${YELLOW}Building benchmarks with debug symbols...${NC}"
CARGO_PROFILE_BENCH_DEBUG=true cargo build --release -p ferrotunnel-protocol --benches

# Check for flamegraph
if ! command -v cargo-flamegraph &> /dev/null; then
    echo -e "${YELLOW}Installing cargo-flamegraph...${NC}"
    cargo install flamegraph
fi

OUTPUT_FILE="${OUTPUT_DIR}/codec_${TIMESTAMP}.svg"

echo -e "${YELLOW}Profiling codec benchmarks...${NC}"
echo ""

# Profile the codec benchmarks
cargo flamegraph \
    --root \
    --output "$OUTPUT_FILE" \
    --bench codec \
    -p ferrotunnel-protocol \
    -- --bench

echo ""
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}Profiling complete!${NC}"
echo -e "  Flamegraph: ${BLUE}${OUTPUT_FILE}${NC}"
echo ""
echo -e "Open in browser: ${YELLOW}firefox ${OUTPUT_FILE}${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
