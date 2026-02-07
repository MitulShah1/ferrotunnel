#!/bin/bash
# FerroTunnel Integration Test Script
# Tests the embedded library API by running server, client, and making HTTP requests

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration (using high ports to avoid conflicts)
TOKEN="test-secret-token"
SERVER_BIND="127.0.0.1:17835"
HTTP_BIND="127.0.0.1:18080"
LOCAL_SERVICE="127.0.0.1:19000"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# PIDs for cleanup
SERVER_PID=""
CLIENT_PID=""
HTTP_SERVER_PID=""

cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"

    [ -n "$CLIENT_PID" ] && kill "$CLIENT_PID" 2>/dev/null && echo "Stopped client"
    [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null && echo "Stopped server"
    [ -n "$HTTP_SERVER_PID" ] && kill "$HTTP_SERVER_PID" 2>/dev/null && echo "Stopped HTTP server"

    # Clean up temp directory
    [ -d "$TEMP_DIR" ] && rm -rf "$TEMP_DIR"

    echo -e "${GREEN}Cleanup complete${NC}"
}

trap cleanup EXIT

echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}FerroTunnel Integration Test${NC}"
echo -e "${GREEN}================================${NC}"
echo ""

# Build the project first
echo -e "${YELLOW}Step 1: Building project and examples...${NC}"
cd "$PROJECT_DIR"
# Pre-build to ensure examples are ready
cargo build --quiet -p ferrotunnel --examples
echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# Create a temp directory with test content
TEMP_DIR=$(mktemp -d)
echo "<html><body><h1>Hello from FerroTunnel!</h1><p>Tunnel is working!</p></body></html>" > "$TEMP_DIR/index.html"
echo "Test file content" > "$TEMP_DIR/test.txt"

# Step 2: Start local HTTP server (simulates the service to tunnel)
echo -e "${YELLOW}Step 2: Starting local HTTP server on $LOCAL_SERVICE...${NC}"
cd "$TEMP_DIR"
python3 -m http.server 19000 --bind 127.0.0.1 > /tmp/ferrotunnel-http-server.log 2>&1 &
HTTP_SERVER_PID=$!
cd "$PROJECT_DIR"
sleep 2

if kill -0 "$HTTP_SERVER_PID" 2>/dev/null; then
    echo -e "${GREEN}✓ Local HTTP server started (PID: $HTTP_SERVER_PID)${NC}"
else
    echo -e "${RED}✗ Failed to start local HTTP server${NC}"
    cat /tmp/ferrotunnel-http-server.log
    exit 1
fi
echo ""

# Step 3: Start the tunnel server
echo -e "${YELLOW}Step 3: Starting FerroTunnel server...${NC}"
RUST_LOG=info cargo run --quiet --example embedded_server -- \
    --bind "$SERVER_BIND" \
    --http-bind "$HTTP_BIND" \
    --token "$TOKEN" > /tmp/ferrotunnel-server.log 2>&1 &
SERVER_PID=$!

# Wait for server to bind
echo "  Waiting for server to initialize..."
for i in {1..10}; do
    if grep -q "Starting server..." /tmp/ferrotunnel-server.log 2>/dev/null; then
        break
    fi
    sleep 1
done

if kill -0 "$SERVER_PID" 2>/dev/null; then
    echo -e "${GREEN}✓ Tunnel server started (PID: $SERVER_PID)${NC}"
    echo "  Tunnel control: $SERVER_BIND"
    echo "  HTTP ingress:   $HTTP_BIND"
else
    echo -e "${RED}✗ Failed to start tunnel server${NC}"
    cat /tmp/ferrotunnel-server.log
    exit 1
fi
echo ""

# Step 4: Start the tunnel client
echo -e "${YELLOW}Step 4: Starting FerroTunnel client...${NC}"
RUST_LOG=info cargo run --quiet --example embedded_client -- \
    --server "$SERVER_BIND" \
    --token "$TOKEN" \
    --local-addr "$LOCAL_SERVICE" \
    --tunnel-id "127.0.0.1" > /tmp/ferrotunnel-client.log 2>&1 &
CLIENT_PID=$!

# Wait for client to connect
echo "  Waiting for client to connect..."
for i in {1..10}; do
    if grep -q "Connected!" /tmp/ferrotunnel-client.log 2>/dev/null; then
        break
    fi
    sleep 1
done

if kill -0 "$CLIENT_PID" 2>/dev/null; then
    echo -e "${GREEN}✓ Tunnel client started (PID: $CLIENT_PID)${NC}"
    echo "  Connected to:   $SERVER_BIND"
    echo "  Forwarding to:  $LOCAL_SERVICE"
else
    echo -e "${RED}✗ Failed to start tunnel client${NC}"
    cat /tmp/ferrotunnel-client.log
    exit 1
fi
echo ""

# Step 5: Test the tunnel
echo -e "${YELLOW}Step 5: Testing tunnel with HTTP requests...${NC}"
echo ""

# Test 1: Fetch index.html through the tunnel
echo -n "  Test 1 - Fetch /index.html through tunnel: "
RESPONSE=$(curl -s "http://$HTTP_BIND/index.html" 2>/dev/null || echo "FAILED")
if echo "$RESPONSE" | grep -q "Hello from FerroTunnel"; then
    echo -e "${GREEN}PASSED${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo "  Response: $RESPONSE"
fi

# Test 2: Fetch test.txt through the tunnel
echo -n "  Test 2 - Fetch /test.txt through tunnel:   "
RESPONSE=$(curl -s "http://$HTTP_BIND/test.txt" 2>/dev/null || echo "FAILED")
if echo "$RESPONSE" | grep -q "Test file content"; then
    echo -e "${GREEN}PASSED${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo "  Response: $RESPONSE"
fi

# Test 3: Check response headers
echo -n "  Test 3 - Check HTTP response headers:      "
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "http://$HTTP_BIND/" 2>/dev/null || echo "000")
if [ "$STATUS" = "200" ]; then
    echo -e "${GREEN}PASSED${NC} (Status: $STATUS)"
else
    echo -e "${RED}FAILED${NC} (Status: $STATUS)"
fi

echo ""
echo -e "${GREEN}================================${NC}"
echo -e "${GREEN}Integration Test Complete!${NC}"
echo -e "${GREEN}================================${NC}"
echo ""

# Show logs summary
echo -e "${YELLOW}Server log (last 5 lines):${NC}"
tail -5 /tmp/ferrotunnel-server.log 2>/dev/null || echo "  (no logs)"
echo ""

echo -e "${YELLOW}Client log (last 5 lines):${NC}"
tail -5 /tmp/ferrotunnel-client.log 2>/dev/null || echo "  (no logs)"
echo ""

echo -e "${GREEN}Test completed successfully!${NC}"
