# FerroTunnel Deployment Guide

## Quick Start

### Server

```bash
# Basic server (no TLS)
ferrotunnel-server --bind 0.0.0.0:8080 --token "your-secret-token"

# With TLS
ferrotunnel-server --bind 0.0.0.0:8443 \
  --token "your-secret-token" \
  --tls-cert /path/to/server.crt \
  --tls-key /path/to/server.key
```

### Client

```bash
# Connect to server
ferrotunnel-client --server tunnel.example.com:8443 \
  --token "your-secret-token" \
  --local 127.0.0.1:3000 \
  --tls-ca /path/to/ca.crt
```

## System Requirements

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 1 core | 2+ cores |
| Memory | 128MB | 512MB+ |
| Disk | 50MB | 100MB |
| Network | 10 Mbps | 100+ Mbps |

## TLS Setup

### Development (Self-Signed)

```bash
# Generate CA
openssl genrsa -out ca.key 4096
openssl req -new -x509 -days 365 -key ca.key -out ca.crt \
  -subj "/CN=FerroTunnel CA"

# Generate server certificate
openssl genrsa -out server.key 2048
openssl req -new -key server.key -out server.csr \
  -subj "/CN=tunnel.example.com"
openssl x509 -req -days 365 -in server.csr -CA ca.crt -CAkey ca.key \
  -CAcreateserial -out server.crt \
  -extfile <(echo "subjectAltName=DNS:tunnel.example.com,DNS:localhost,IP:127.0.0.1")
```

### Production (Let's Encrypt)

```bash
# Using certbot
certbot certonly --standalone -d tunnel.example.com

# Certificate locations
# /etc/letsencrypt/live/tunnel.example.com/fullchain.pem
# /etc/letsencrypt/live/tunnel.example.com/privkey.pem
```

## Configuration

### Server Configuration File

```toml
# /etc/ferrotunnel/server.toml

[server]
bind = "0.0.0.0:8443"
token = "${FERROTUNNEL_TOKEN}"  # Use environment variable

[tls]
enabled = true
cert_path = "/etc/ferrotunnel/server.crt"
key_path = "/etc/ferrotunnel/server.key"

[limits]
max_sessions = 1000
max_streams_per_session = 100
max_frame_bytes = 16777216  # 16MB

[rate_limit]
streams_per_sec = 100
bytes_per_sec = 10485760  # 10MB/s
```

### Client Configuration File

```toml
# ~/.config/ferrotunnel/client.toml

[client]
server = "tunnel.example.com:8443"
token = "${FERROTUNNEL_TOKEN}"
local = "127.0.0.1:3000"

[tls]
enabled = true
ca_cert_path = "/path/to/ca.crt"
server_name = "tunnel.example.com"

[resilience]
reconnect_base_ms = 1000
reconnect_max_ms = 60000
```

## Running as a Service

### systemd (Linux)

Create `/etc/systemd/system/ferrotunnel-server.service`:

```ini
[Unit]
Description=FerroTunnel Server
After=network.target

[Service]
Type=simple
User=ferrotunnel
Group=ferrotunnel
ExecStart=/usr/local/bin/ferrotunnel-server --config /etc/ferrotunnel/server.toml
Restart=always
RestartSec=5
Environment=FERROTUNNEL_TOKEN=your-secret-token

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ReadOnlyPaths=/etc/ferrotunnel

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable ferrotunnel-server
sudo systemctl start ferrotunnel-server
```

### Docker

```dockerfile
# Dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ferrotunnel-server /usr/local/bin/
EXPOSE 8443
ENTRYPOINT ["ferrotunnel-server"]
```

```yaml
# docker-compose.yml
version: '3.8'
services:
  ferrotunnel:
    build: .
    ports:
      - "8443:8443"
    environment:
      - FERROTUNNEL_TOKEN=${FERROTUNNEL_TOKEN}
    volumes:
      - ./certs:/etc/ferrotunnel:ro
    command: >
      --bind 0.0.0.0:8443
      --token ${FERROTUNNEL_TOKEN}
      --tls-cert /etc/ferrotunnel/server.crt
      --tls-key /etc/ferrotunnel/server.key
    restart: unless-stopped
```

## Scaling

### Resource Limits Tuning

| Workload | max_sessions | max_streams | Memory |
|----------|--------------|-------------|--------|
| Light | 100 | 50 | ~50MB |
| Medium | 500 | 100 | ~100MB |
| Heavy | 1000 | 100 | ~200MB |

### Load Balancing

For high availability, run multiple server instances behind a load balancer:

```
                    ┌─────────────────┐
                    │  Load Balancer  │
                    │   (TCP/TLS)     │
                    └────────┬────────┘
                             │
            ┌────────────────┼────────────────┐
            ▼                ▼                ▼
    ┌───────────────┐ ┌───────────────┐ ┌───────────────┐
    │ FerroTunnel   │ │ FerroTunnel   │ │ FerroTunnel   │
    │   Server 1    │ │   Server 2    │ │   Server 3    │
    └───────────────┘ └───────────────┘ └───────────────┘
```

## Monitoring

### Health Check Endpoint

```bash
# Check if server is healthy
curl -f http://localhost:9090/health
```

### Metrics

Prometheus metrics available at `/metrics`:

- `ferrotunnel_sessions_active` - Current active sessions
- `ferrotunnel_streams_active` - Current active streams
- `ferrotunnel_bytes_transferred_total` - Total bytes transferred
- `ferrotunnel_errors_total` - Total errors by type

### Logging

Set log level via environment:

```bash
RUST_LOG=info ferrotunnel-server ...
RUST_LOG=debug ferrotunnel-server ...  # Verbose
RUST_LOG=ferrotunnel=trace ferrotunnel-server ...  # Very verbose
```
