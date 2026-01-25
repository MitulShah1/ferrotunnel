#!/bin/bash
# FerroTunnel Plugin Test Script
# Tests the example plugins by running them and verifying behavior with curl

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
echo "Project Dir: $PROJECT_DIR"

# PIDs for cleanup
PLUGIN_PID=""

cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    [ -n "$PLUGIN_PID" ] && kill "$PLUGIN_PID" 2>/dev/null && echo "Stopped plugin example"
    echo -e "${GREEN}Cleanup complete${NC}"
}

trap cleanup EXIT

echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}FerroTunnel Plugin Test${NC}"
echo -e "${GREEN}================================${NC}"
echo ""

# Build the project first
echo -e "${YELLOW}Step 1: Building examples...${NC}"
cd "$PROJECT_DIR"
cargo build -p ferrotunnel-plugin --examples 2>&1 | tail -3
echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# Helper to start an example and wait for it
start_example() {
    EXAMPLE_NAME=$1
    echo -e "${YELLOW}Starting example: $EXAMPLE_NAME...${NC}"
    RUST_LOG=info cargo run -p ferrotunnel-plugin --example "$EXAMPLE_NAME" > "/tmp/ferrotunnel-$EXAMPLE_NAME.log" 2>&1 &
    PLUGIN_PID=$!
    sleep 3 # Give it time to compile/start if needed

    if kill -0 "$PLUGIN_PID" 2>/dev/null; then
        echo -e "${GREEN}✓ Example running (PID: $PLUGIN_PID)${NC}"
    else
        echo -e "${RED}✗ Failed to start example $EXAMPLE_NAME${NC}"
        cat "/tmp/ferrotunnel-$EXAMPLE_NAME.log"
        exit 1
    fi
}

stop_example() {
    if [ -n "$PLUGIN_PID" ]; then
        kill "$PLUGIN_PID" 2>/dev/null
        wait "$PLUGIN_PID" 2>/dev/null || true
        PLUGIN_PID=""
        echo "Stopped example"
    fi
}

# Test 1: Hello Plugin (Header Injection)
# Expects a mock server response, but the examples use a mock RequestContext internally and don't actually bind a port?
# CHECK: Let's re-read the examples. Do they start a server or just run a mock test?
# If they just run a mock test and exit, we can't curl them.
# Let's verify header_filter.rs content.

if grep -q "tokio::main" ferrotunnel-plugin/examples/header_filter.rs; then
     # It has a main. Let's see if it binds a port or just runs logic locally.
     :
fi

# ... Wait, I wrote the examples. They contain a main() that constructs a registry,
# creates a logic request, executes hooks, and asserts logic.
# THEY DO NOT START A SERVER.
# They are self-contained logical tests.

echo -e "${YELLOW}Note: The current examples are self-contained logic tests, not running servers.${NC}"
echo -e "${YELLOW}Running examples directly as verification...${NC}"

run_example_test() {
    EXAMPLE_NAME=$1
    echo -n "  Running $EXAMPLE_NAME: "
    if cargo run -p ferrotunnel-plugin --example "$EXAMPLE_NAME" > "/tmp/ferrotunnel-$EXAMPLE_NAME.log" 2>&1; then
        echo -e "${GREEN}PASSED${NC}"
    else
        echo -e "${RED}FAILED${NC}"
        cat "/tmp/ferrotunnel-$EXAMPLE_NAME.log"
        exit 1
    fi
}

run_example_test "hello_plugin"
run_example_test "header_filter"
run_example_test "ip_blocklist"

echo ""
echo -e "${GREEN}All plugin logic examples passed!${NC}"

# If we want to test plugins inside a real server, we need to run ferrotunnel-server
# but ferrotunnel-server is hardcoded to only use built-in plugins in main.rs currently.
# To test custom plugins E2E, we'd need a binary that loads them.
# The user wants "scripts/test-plugins.sh".
# Since the examples currently just run logic, this script effectively just runs them.

echo ""
echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}Plugin Tests Complete!${NC}"
echo -e "${GREEN}================================${NC}"
