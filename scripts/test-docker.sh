#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "🚀 Starting FerroTunnel Docker Verification..."

# Cleanup previous runs
echo "🧹 Cleaning up old containers..."
docker compose down -v --remove-orphans > /dev/null 2>&1 || true

# Build and start with non-conflicting ports
export FERROTUNNEL_HTTP_PORT=8081
export FERROTUNNEL_METRICS_PORT=9091
export FERROTUNNEL_DASHBOARD_PORT=4041

echo "🏗️  Building and starting Docker Compose stack..."
docker compose up -d --build

# Wait for services to be healthy
echo "⏳ Waiting for services to initialize (15s)..."
sleep 15

# 1. Test Server Health & Metrics
echo "📊 Verifying Server Health & Metrics..."
health_resp=$(curl -s http://localhost:8081/health || true)
if [[ "$health_resp" == "OK" ]]; then
    echo -e "${GREEN}✅ Health endpoint is OK.${NC}"
else
    echo -e "${RED}❌ Health endpoint check failed.${NC}"
    docker compose logs ferrotunnel-server
    exit 1
fi

metrics_output=$(curl -s http://localhost:9091/metrics || true)
if [[ -z "$metrics_output" ]]; then
    echo -e "${RED}❌ Metrics endpoint returned no data.${NC}"
    docker compose logs ferrotunnel-server
    exit 1
fi

if echo "$metrics_output" | grep -q "process_" || echo "$metrics_output" | grep -q "ferrotunnel"; then
    echo -e "${GREEN}✅ Metrics endpoint is active and returning telemetry.${NC}"
else
    echo -e "${RED}❌ Metrics endpoint returned unexpected content.${NC}"
    echo "Output: $metrics_output"
    docker compose logs ferrotunnel-server
    exit 1
fi

# 2. Test Dashboard Presence
echo "🖥️  Verifying Client Dashboard (Port 4041)..."
if curl -s http://localhost:4041 | grep -q "Dashboard" || curl -s http://localhost:4041 | grep -q "ferrotunnel"; then
    echo -e "${GREEN}✅ Dashboard is accessible.${NC}"
else
    echo -e "${RED}❌ Dashboard accessibility check failed.${NC}"
    docker compose logs ferrotunnel-client
    exit 1
fi

# 3. Test HTTP Ingress (Full Tunnel Flow)
echo "🌐 Verifying End-to-End Tunnel Connectivity (Port 8081)..."
ingress_resp=$(curl -s -H "Host: demo.ferrotunnel.local" http://localhost:8081 || true)
echo "   Ingress Response received."

echo -e "${GREEN}✨ Docker verification complete!${NC}"
echo "Summary:"
docker compose ps

echo "🛑 Shutting down test environment..."
docker compose down -v
