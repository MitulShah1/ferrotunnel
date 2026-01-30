#!/bin/bash
# Profile memory allocations in FerroTunnel
#
# Usage: ./tools/profiler/profile-memory.sh [binary] [args...]
#
# Requirements:
#   - heaptrack (Linux): sudo apt-get install heaptrack heaptrack-gui
#   - OR valgrind with massif: sudo apt-get install valgrind

set -e

OUTPUT_DIR="${PROFILE_OUTPUT:-./target/profiles}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BINARY="${1:-ferrotunnel-server}"
shift || true
ARGS="$@"

# If profiling server and no args provided, add default args
if [ "$BINARY" == "ferrotunnel-server" ] && [ -z "$ARGS" ]; then
    ARGS="--bind 127.0.0.1:9000 --token profiling-test-token --http-bind 127.0.0.1:8080"
fi

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║          FerroTunnel Memory Profiler                       ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

mkdir -p "$OUTPUT_DIR"

# Build release
echo -e "${YELLOW}Building release...${NC}"
cargo build --release -p "$BINARY" 2>/dev/null || cargo build --release --workspace

BINARY_PATH="./target/release/${BINARY}"

if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${YELLOW}Binary not found: ${BINARY_PATH}${NC}"
    echo "Available binaries:"
    ls -1 ./target/release/ | grep -v '\.d$' | head -10
    exit 1
fi

# Try heaptrack first, fall back to valgrind
if command -v heaptrack &> /dev/null; then
    echo -e "${YELLOW}Using heaptrack for memory profiling...${NC}"
    echo -e "Press Ctrl+C to stop profiling and generate report"
    echo ""

    OUTPUT_FILE="${OUTPUT_DIR}/heaptrack_${BINARY}_${TIMESTAMP}.gz"

    heaptrack -o "$OUTPUT_FILE" "$BINARY_PATH" $ARGS

    echo ""
    echo -e "${GREEN}Profiling complete!${NC}"
    echo -e "  Output: ${BLUE}${OUTPUT_FILE}${NC}"
    echo ""
    echo -e "Analyze with: ${YELLOW}heaptrack_gui ${OUTPUT_FILE}${NC}"

elif command -v valgrind &> /dev/null; then
    echo -e "${YELLOW}Using valgrind massif for memory profiling...${NC}"
    echo -e "Press Ctrl+C to stop profiling"
    echo ""

    OUTPUT_FILE="${OUTPUT_DIR}/massif_${BINARY}_${TIMESTAMP}.out"

    valgrind --tool=massif --massif-out-file="$OUTPUT_FILE" "$BINARY_PATH" $ARGS

    echo ""
    echo -e "${GREEN}Profiling complete!${NC}"
    echo -e "  Output: ${BLUE}${OUTPUT_FILE}${NC}"
    echo ""
    echo -e "Analyze with: ${YELLOW}ms_print ${OUTPUT_FILE}${NC}"

else
    echo -e "${YELLOW}No memory profiler found!${NC}"
    echo ""
    echo "Install one of:"
    echo "  - heaptrack: sudo apt-get install heaptrack heaptrack-gui"
    echo "  - valgrind:  sudo apt-get install valgrind"
    exit 1
fi

echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
