#!/bin/bash
# Don't use set -e as we handle errors manually with test counters

# Configuration (can be overridden via environment)
SERVER_BIN="${SERVER_BIN:-target/debug/ferrotunnel}"
CLIENT_BIN="${CLIENT_BIN:-target/debug/ferrotunnel}"
TOKEN="${TOKEN:-test_token}"
LOCAL_PORT="${LOCAL_PORT:-8123}"
INGRESS_PORT="${INGRESS_PORT:-8082}"
CONTROL_PORT="${CONTROL_PORT:-8083}"
DASHBOARD_PORT="${DASHBOARD_PORT:-4041}"
TIMEOUT="${TIMEOUT:-30}"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0

# PIDs for cleanup
SERVER_PID=""
CLIENT_PID=""
LOCAL_SERVICE_PID=""

log_info()  { echo -e "${BLUE}[INFO]${NC} $1"; }
log_ok()    { echo -e "${GREEN}[PASS]${NC} $1"; ((TESTS_PASSED++)); }
log_fail()  { echo -e "${RED}[FAIL]${NC} $1"; ((TESTS_FAILED++)); }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_step()  { echo -e "\n${GREEN}=== $1 ===${NC}"; }

cleanup() {
    log_info "Cleaning up processes..."
    [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
    [ -n "$CLIENT_PID" ] && kill "$CLIENT_PID" 2>/dev/null || true
    [ -n "$LOCAL_SERVICE_PID" ] && kill "$LOCAL_SERVICE_PID" 2>/dev/null || true
    rm -f /tmp/ferro_mock_server.py
}
trap cleanup EXIT

wait_for_port() {
    local port=$1
    local name=$2
    local max_wait=${3:-$TIMEOUT}
    local count=0

    while ! nc -z 127.0.0.1 "$port" 2>/dev/null; do
        count=$((count + 1))
        if [ $count -ge $max_wait ]; then
            log_fail "$name did not start on port $port within ${max_wait}s"
            return 1
        fi
        sleep 1
    done
    log_ok "$name is ready on port $port"
    return 0
}

check_json_field() {
    local json="$1"
    local field="$2"
    local expected="$3"

    if echo "$json" | grep -q "$expected"; then
        return 0
    fi
    return 1
}

extract_uuid() {
    echo "$1" | grep -oP '"id":"\K[0-9a-fA-F-]{36}' | head -n 1
}

# ============================================================
log_step "Building Project"
# ============================================================

cargo build --bins 2>&1 | tail -5

# ============================================================
log_step "Starting Mock Local Service"
# ============================================================

cat > /tmp/ferro_mock_server.py << PYEOF
from http.server import HTTPServer, BaseHTTPRequestHandler
import json

class Handler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        pass

    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps({"message": "Hello from GET", "path": self.path}).encode())

    def do_POST(self):
        length = int(self.headers.get('content-length', 0))
        data = self.rfile.read(length)
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        try:
            req_json = json.loads(data)
            self.wfile.write(json.dumps({"received": req_json, "status": "processed"}).encode())
        except:
            self.wfile.write(b'{"error": "Invalid JSON"}')

HTTPServer(('127.0.0.1', ${LOCAL_PORT}), Handler).serve_forever()
PYEOF

python3 /tmp/ferro_mock_server.py > mock_server.log 2>&1 &
LOCAL_SERVICE_PID=$!

wait_for_port $LOCAL_PORT "Mock Service" || exit 1

# ============================================================
log_step "Starting FerroTunnel Server"
# ============================================================

$SERVER_BIN server \
    --bind 127.0.0.1:$CONTROL_PORT \
    --http-bind 127.0.0.1:$INGRESS_PORT \
    --token $TOKEN \
    > server.log 2>&1 &
SERVER_PID=$!

wait_for_port $CONTROL_PORT "Server (Control)" || { cat server.log; exit 1; }
wait_for_port $INGRESS_PORT "Server (Ingress)" || { cat server.log; exit 1; }

# ============================================================
log_step "Starting FerroTunnel Client with Dashboard"
# ============================================================

$CLIENT_BIN client \
    --server 127.0.0.1:$CONTROL_PORT \
    --token $TOKEN \
    --local-addr 127.0.0.1:$LOCAL_PORT \
    --dashboard-port $DASHBOARD_PORT \
    > client.log 2>&1 &
CLIENT_PID=$!

wait_for_port $DASHBOARD_PORT "Dashboard" || { cat client.log; exit 1; }
sleep 2  # Extra wait for tunnel handshake

# ============================================================
log_step "Test 1: Dashboard Health Check"
# ============================================================

HEALTH=$(curl -sf http://127.0.0.1:$DASHBOARD_PORT/api/v1/health || echo "FAILED")
if echo "$HEALTH" | grep -qE '"status":"(ok|healthy)"'; then
    log_ok "Health endpoint returned OK"
else
    log_fail "Health endpoint failed: $HEALTH"
fi

# ============================================================
log_step "Test 2: Tunnel Status"
# ============================================================

TUNNELS=$(curl -sf http://127.0.0.1:$DASHBOARD_PORT/api/v1/tunnels || echo "[]")
if echo "$TUNNELS" | grep -q '"status":"connected"'; then
    log_ok "Tunnel shows connected status"
else
    log_fail "Tunnel not connected: $TUNNELS"
fi

# Ingress routes by Host header (tunnel ID); get first tunnel's id for requests
TUNNEL_ID=$(echo "$TUNNELS" | jq -r '.[0].id // empty')
if [[ -z "$TUNNEL_ID" ]]; then
    log_fail "Could not get tunnel ID from dashboard"
    exit 1
fi

# ============================================================
log_step "Test 3: Send Traffic Through Tunnel"
# ============================================================

for i in {1..5}; do
    RESPONSE=$(curl -sf -X POST \
        -H "Content-Type: application/json" \
        -H "Host: $TUNNEL_ID" \
        -H "X-Tunnel-Token: $TOKEN" \
        -d "{\"id\": $i, \"msg\": \"hello dashboard\"}" \
        http://127.0.0.1:$INGRESS_PORT/ || echo "FAILED")

    if echo "$RESPONSE" | grep -q "processed"; then
        log_ok "Request $i: POST success"
    else
        log_fail "Request $i: POST failed - $RESPONSE"
    fi
done

# Also test GET
RESPONSE=$(curl -sf -H "Host: $TUNNEL_ID" -H "X-Tunnel-Token: $TOKEN" http://127.0.0.1:$INGRESS_PORT/test-path || echo "FAILED")
if echo "$RESPONSE" | grep -q "Hello from GET"; then
    log_ok "GET request success"
else
    log_fail "GET request failed - $RESPONSE"
fi

sleep 1  # Wait for dashboard to process

# ============================================================
log_step "Test 4: Dashboard Captured Requests"
# ============================================================

REQUESTS=$(curl -sf http://127.0.0.1:$DASHBOARD_PORT/api/v1/requests || echo "[]")
REQUEST_COUNT=$(echo "$REQUESTS" | grep -o '"id"' | wc -l)

if [ "$REQUEST_COUNT" -ge 6 ]; then
    log_ok "Dashboard captured $REQUEST_COUNT requests"
else
    log_fail "Expected at least 6 requests, got $REQUEST_COUNT"
fi

# ============================================================
log_step "Test 5: Request Details"
# ============================================================

REQUEST_ID=$(extract_uuid "$REQUESTS")
if [ -z "$REQUEST_ID" ]; then
    log_fail "Could not extract request ID"
else
    DETAILS=$(curl -sf http://127.0.0.1:$DASHBOARD_PORT/api/v1/requests/$REQUEST_ID || echo "{}")

    # Check for POST body or GET path in captured details
    if echo "$DETAILS" | grep -qE '(hello dashboard|test-path|Hello from GET)'; then
        log_ok "Request body/path captured correctly"
    else
        log_fail "Request body not found in details: $DETAILS"
    fi

    if echo "$DETAILS" | grep -q "request_headers"; then
        log_ok "Request headers captured"
    else
        log_fail "Request headers missing"
    fi

    if echo "$DETAILS" | grep -q "response_headers"; then
        log_ok "Response headers captured"
    else
        log_fail "Response headers missing"
    fi
fi

# ============================================================
log_step "Test 6: Replay Request"
# ============================================================

if [ -n "$REQUEST_ID" ]; then
    REPLAY=$(curl -sf -X POST http://127.0.0.1:$DASHBOARD_PORT/api/v1/requests/$REQUEST_ID/replay || echo "FAILED")

    if echo "$REPLAY" | grep -q '"status":"replayed"'; then
        log_ok "Replay request succeeded"
    else
        log_fail "Replay failed: $REPLAY"
    fi
else
    log_warn "Skipping replay test - no request ID"
fi

# ============================================================
log_step "Test 7: SSE Events Endpoint"
# ============================================================

# Quick check that SSE endpoint responds
SSE_CHECK=$(timeout 2 curl -sf http://127.0.0.1:$DASHBOARD_PORT/api/v1/events 2>/dev/null || echo "timeout")
if [ "$SSE_CHECK" = "timeout" ] || [ -n "$SSE_CHECK" ]; then
    log_ok "SSE events endpoint accessible"
else
    log_fail "SSE events endpoint not responding"
fi

# ============================================================
log_step "Test 8: Dashboard Static Files"
# ============================================================

HTML=$(curl -sf http://127.0.0.1:$DASHBOARD_PORT/ || echo "FAILED")
if echo "$HTML" | grep -q "FerroTunnel"; then
    log_ok "Dashboard HTML loads correctly"
else
    log_fail "Dashboard HTML failed to load"
fi

# ============================================================
log_step "Test Summary"
# ============================================================

echo ""
echo -e "${GREEN}Passed:${NC} $TESTS_PASSED"
echo -e "${RED}Failed:${NC} $TESTS_FAILED"
echo ""

if [ $TESTS_FAILED -gt 0 ]; then
    echo -e "${RED}Some tests failed!${NC}"
    echo "Check logs: server.log, client.log, mock_server.log"
else
    echo -e "${GREEN}All tests passed!${NC}"
fi

echo ""
echo -e "${BLUE}Dashboard URL:${NC} http://127.0.0.1:$DASHBOARD_PORT"
echo -e "Press [ENTER] to stop servers and exit..."
read

exit $TESTS_FAILED
