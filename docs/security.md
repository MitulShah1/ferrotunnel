# FerroTunnel Security Best Practices

## Token Management

### Token Requirements

- **Minimum length**: 32 bytes (256 bits)
- **Format**: Printable ASCII characters only
- **Generation**: Use cryptographically secure random generator

```bash
# Generate a secure token
openssl rand -base64 32

# Or using /dev/urandom
head -c 32 /dev/urandom | base64
```

### Token Storage

- **Never** commit tokens to version control
- Use environment variables or secret managers
- Rotate tokens regularly (recommended: every 90 days)

```bash
# Environment variable
export FERROTUNNEL_TOKEN="$(cat /run/secrets/tunnel-token)"

# Secret manager (example with AWS)
FERROTUNNEL_TOKEN=$(aws secretsmanager get-secret-value \
  --secret-id ferrotunnel/token --query SecretString --output text)
```

### Token Hashing

FerroTunnel supports storing hashed tokens for additional security:

```rust
use ferrotunnel_core::auth::hash_token;

let token = "my-secret-token";
let hash = hash_token(token);
// Store hash, not plaintext
```

## TLS Configuration

### Requirements

- **TLS 1.3 only** - Enforced by rustls
- Modern cipher suites (AEAD only)
- Valid certificates with proper SANs

### Certificate Checklist

- [ ] Certificate matches server hostname
- [ ] Subject Alternative Names (SANs) include all hostnames/IPs
- [ ] Certificate not expired
- [ ] Certificate chain is complete
- [ ] Private key permissions are 600 (owner read/write only)

```bash
# Verify certificate
openssl x509 -in server.crt -text -noout

# Check expiration
openssl x509 -in server.crt -enddate -noout

# Verify chain
openssl verify -CAfile ca.crt server.crt
```

### Client Certificate Authentication

For high-security environments, enable mutual TLS:

```toml
[tls]
enabled = true
client_auth = true
ca_cert_path = "/path/to/client-ca.crt"
```

## Rate Limiting

### Default Limits

| Limit | Default | Description |
|-------|---------|-------------|
| Streams/sec | 100 | New stream opens per second |
| Bytes/sec | 10MB | Data throughput per session |
| Burst | 2x | Burst allowance multiplier |

### Tuning Guidelines

```toml
[rate_limit]
# For API gateway (many small requests)
streams_per_sec = 500
bytes_per_sec = 5242880  # 5MB/s

# For file transfer (fewer large requests)
streams_per_sec = 50
bytes_per_sec = 104857600  # 100MB/s
```

## Resource Limits

### Memory Protection

```toml
[limits]
max_sessions = 1000
max_streams_per_session = 100
max_frame_bytes = 16777216  # 16MB
max_inflight_frames = 100
```

### Connection Limits

- Set `max_sessions` based on available memory (~100KB per session)
- Set `max_streams_per_session` based on expected concurrency
- Use `max_frame_bytes` to prevent memory exhaustion

## Network Security

### Firewall Rules

```bash
# Allow only tunnel port
iptables -A INPUT -p tcp --dport 8443 -j ACCEPT
iptables -A INPUT -p tcp --dport 8443 -m state --state ESTABLISHED -j ACCEPT

# Rate limit connections
iptables -A INPUT -p tcp --dport 8443 -m connlimit --connlimit-above 100 -j REJECT
```

### Private Network Deployment

For internal services, bind to private interface:

```toml
[server]
bind = "10.0.0.1:8443"  # Private IP only
```

### Reverse Proxy

For additional protection, use a reverse proxy:

```nginx
# nginx.conf
stream {
    upstream ferrotunnel {
        server 127.0.0.1:8443;
    }

    server {
        listen 443;
        proxy_pass ferrotunnel;
        proxy_timeout 300s;
    }
}
```

## Threat Model

### Trust Boundaries

```
┌─────────────────────────────────────────────────────┐
│                    INTERNET                         │
│                   (Untrusted)                       │
└─────────────────────────┬───────────────────────────┘
                          │ TLS 1.3
                          ▼
┌─────────────────────────────────────────────────────┐
│              FerroTunnel Server                     │
│  ┌─────────────────────────────────────────────┐    │
│  │ Token Authentication                        │    │
│  │ Rate Limiting                               │    │
│  │ Resource Limits                             │    │
│  └─────────────────────────────────────────────┘    │
└─────────────────────────┬───────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────┐
│              Internal Network                       │
│                   (Trusted)                         │
└─────────────────────────────────────────────────────┘
```

### Attack Vectors & Mitigations

| Attack | Mitigation |
|--------|------------|
| Token brute force | Rate limiting, long tokens, constant-time compare |
| Connection flooding | Max sessions limit, firewall rules |
| Memory exhaustion | Frame size limits, stream limits |
| MITM | TLS 1.3, certificate validation |
| Replay attacks | Session IDs, timestamps |

## Security Checklist

### Before Deployment

- [ ] TLS enabled with valid certificates
- [ ] Token is 32+ bytes, randomly generated
- [ ] Token stored securely (not in code/config files)
- [ ] Resource limits configured appropriately
- [ ] Firewall rules in place
- [ ] Logging enabled

### Regular Maintenance

- [ ] Rotate tokens every 90 days
- [ ] Renew certificates before expiration
- [ ] Review logs for anomalies
- [ ] Update to latest version
- [ ] Run `cargo audit` on dependencies
