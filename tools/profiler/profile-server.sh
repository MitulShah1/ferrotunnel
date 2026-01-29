#!/bin/bash
# Profile the FerroTunnel server under load
#
# Usage: ./tools/profiler/profile-server.sh [duration_seconds]
#
# Requirements:
#   - cargo-flamegraph: cargo install flamegraph
#   - perf (Linux): sudo apt-get install linux-tools-generic

set -e

# Configuration
DURATION=${1:-30}
OUTPUT_DIR="${PROFILE_OUTPUT:-./target/profiles}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
SAMPLE_FREQ="${SAMPLE_FREQUENCY:-99}"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║          FerroTunnel Server Profiler                       ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Build release with debug symbols
echo -e "${YELLOW}Building release with debug symbols...${NC}"
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release -p ferrotunnel-server

# Check if cargo-flamegraph is installed
if ! command -v cargo-flamegraph &> /dev/null; then
    echo -e "${YELLOW}Installing cargo-flamegraph...${NC}"
    cargo install flamegraph
fi

# Start the server in background with profiling
echo -e "${YELLOW}Starting server with profiling (${DURATION}s)...${NC}"
echo -e "${YELLOW}Make sure to generate load using tools/loadgen!${NC}"
echo ""

OUTPUT_FILE="${OUTPUT_DIR}/server_${TIMESTAMP}.svg"

# Use flamegraph to profile
timeout "$DURATION" cargo flamegraph \
    --root \
    --output "$OUTPUT_FILE" \
    --bin ferrotunnel-server \
    -- --bind "127.0.0.1:9000" \
    2>/dev/null || true

echo ""
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}Profiling complete!${NC}"
echo -e "  Flamegraph: ${BLUE}${OUTPUT_FILE}${NC}"
echo ""
echo -e "Open in browser: ${YELLOW}firefox ${OUTPUT_FILE}${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
