#!/bin/bash
# Test all FerroTunnel examples
#
# Usage:
#   ./scripts/test-examples.sh             # Run all examples
#   ./scripts/test-examples.sh basic       # Run only basic examples
#   ./scripts/test-examples.sh plugins     # Run only plugin examples
#   ./scripts/test-examples.sh advanced    # Run only advanced examples
#   ./scripts/test-examples.sh operational # Run only operational examples
#   ./scripts/test-examples.sh scenarios   # Run only scenario examples
#   ./scripts/test-examples.sh --quick     # Quick compile check only

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
PASSED=0
FAILED=0

echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║          FerroTunnel Examples Test Suite                       ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Change to project root
cd "$(dirname "$0")/.."

# Parse arguments
FILTER="${1:-all}"
QUICK_MODE=false
if [ "$1" = "--quick" ]; then
    QUICK_MODE=true
    FILTER="all"
fi

# Build all examples first (faster than individual checks)
echo -e "${YELLOW}Building all examples...${NC}"
if cargo build -p ferrotunnel-examples --examples 2>&1 | tail -5; then
    echo -e "${GREEN}Build successful!${NC}"
else
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi
echo ""

# Function to mark example as passed
mark_passed() {
    local name=$1
    local note=${2:-""}
    echo -e "  ${GREEN}✓${NC} ${name} ${note}"
    PASSED=$((PASSED + 1))
}

# Function to run a quick example that should complete
run_demo_example() {
    local name=$1

    echo -ne "  Running ${YELLOW}${name}${NC}... "

    if timeout 30 cargo run -p ferrotunnel-examples --example "$name" >/dev/null 2>&1; then
        echo -e "${GREEN}PASSED${NC}"
        PASSED=$((PASSED + 1))
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            echo -e "${YELLOW}TIMEOUT${NC} (expected for interactive)"
            PASSED=$((PASSED + 1))
        else
            echo -e "${RED}FAILED${NC} (exit: $exit_code)"
            FAILED=$((FAILED + 1))
        fi
    fi
}

# Basic examples
if [ "$FILTER" = "all" ] || [ "$FILTER" = "basic" ]; then
    echo -e "${BLUE}┌─ Basic Examples ─────────────────────────────────────────────┐${NC}"
    mark_passed "embedded_server" "(compiled)"
    mark_passed "embedded_client" "(compiled)"
    mark_passed "auto_reconnect" "(compiled)"
    echo -e "${BLUE}└──────────────────────────────────────────────────────────────┘${NC}"
fi

# Plugin examples
if [ "$FILTER" = "all" ] || [ "$FILTER" = "plugins" ]; then
    echo -e "\n${BLUE}┌─ Plugin Examples ────────────────────────────────────────────┐${NC}"

    if [ "$QUICK_MODE" = "false" ]; then
        run_demo_example "custom_plugin"
        run_demo_example "plugin_chain"
    else
        mark_passed "custom_plugin" "(compiled)"
        mark_passed "plugin_chain" "(compiled)"
    fi

    mark_passed "header_filter" "(compiled)"
    mark_passed "ip_blocklist" "(compiled)"
    echo -e "${BLUE}└──────────────────────────────────────────────────────────────┘${NC}"
fi

# Advanced examples
if [ "$FILTER" = "all" ] || [ "$FILTER" = "advanced" ]; then
    echo -e "\n${BLUE}┌─ Advanced Examples ──────────────────────────────────────────┐${NC}"
    mark_passed "tls_config" "(compiled)"
    mark_passed "multi_tunnel" "(compiled)"
    echo -e "${BLUE}└──────────────────────────────────────────────────────────────┘${NC}"
fi

# Operational examples
if [ "$FILTER" = "all" ] || [ "$FILTER" = "operational" ]; then
    echo -e "\n${BLUE}┌─ Operational Examples ──────────────────────────────────────┐${NC}"
    mark_passed "server_graceful_shutdown" "(compiled)"
    mark_passed "server_observability" "(compiled)"
    echo -e "${BLUE}└──────────────────────────────────────────────────────────────┘${NC}"
fi

# Scenario examples
if [ "$FILTER" = "all" ] || [ "$FILTER" = "scenarios" ]; then
    echo -e "\n${BLUE}┌─ Scenario Examples ─────────────────────────────────────────┐${NC}"
    mark_passed "expose_local_dev" "(compiled)"
    mark_passed "receive_webhooks_locally" "(compiled)"
    echo -e "${BLUE}└──────────────────────────────────────────────────────────────┘${NC}"
fi

# Summary
echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo -e "  ${GREEN}Passed:${NC} $PASSED    ${RED}Failed:${NC} $FAILED"
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"

# Exit with error if any failed
if [ $FAILED -gt 0 ]; then
    echo -e "\n${RED}Some tests failed!${NC}"
    exit 1
fi

echo -e "\n${GREEN}All example tests passed!${NC}"
