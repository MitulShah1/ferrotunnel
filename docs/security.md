# FerroTunnel Security Best Practices

## Why Rust Matters: Memory Safety Comparison

### Traditional C/C++ Tunnels: A History of Memory Vulnerabilities

Tunneling solutions built in C/C++ have suffered from **30+ critical memory safety vulnerabilities** over the past decade. These vulnerabilities stem from manual memory management, lack of bounds checking, and unsafe concurrency primitives inherent to C/C++.

**Memory Safety CVEs in Traditional Tunnels:**

| Tunnel | Language | Memory Safety CVEs | Critical Examples |
|--------|----------|-------------------|-------------------|
| OpenSSH | C | 15+ | [CVE-2024-6387](https://nvd.nist.gov/vuln/detail/CVE-2024-6387) (race condition RCE), [CVE-2023-25136](https://nvd.nist.gov/vuln/detail/CVE-2023-25136) (double-free), [CVE-2025-26465/26466](https://nvd.nist.gov/vuln/detail/CVE-2025-26465) (memory exhaustion), [CVE-2016-1907](https://nvd.nist.gov/vuln/detail/CVE-2016-1907) (out-of-bounds read) |
| OpenVPN | C | 10+ | [CVE-2024-1305](https://nvd.nist.gov/vuln/detail/CVE-2024-1305) (integer overflow → memory corruption), [CVE-2024-27459](https://nvd.nist.gov/vuln/detail/CVE-2024-27459) (stack overflow → RCE), [CVE-2024-28820](https://nvd.nist.gov/vuln/detail/CVE-2024-28820) (buffer overflow), [CVE-2025-50054](https://nvd.nist.gov/vuln/detail/CVE-2025-50054) (buffer overflow kernel crash) |
| stunnel | C | 8+ | [CVE-2011-2940](https://nvd.nist.gov/vuln/detail/CVE-2011-2940) (heap memory corruption → RCE), [CVE-2002-0002](https://nvd.nist.gov/vuln/detail/CVE-2002-0002) (format string → RCE), [CVE-2013-1762](https://nvd.nist.gov/vuln/detail/CVE-2013-1762) (buffer overflow in NTLM) |

*Search complete CVE databases:*
- [OpenSSH CVEs on NVD](https://nvd.nist.gov/vuln/search/results?query=openssh&results_type=overview)
- [OpenVPN CVEs on NVD](https://nvd.nist.gov/vuln/search/results?query=openvpn&results_type=overview)
- [stunnel CVEs on NVD](https://nvd.nist.gov/vuln/search/results?query=stunnel&results_type=overview)

### How Rust Eliminates These Vulnerabilities

FerroTunnel uses Rust's type system and ownership model to **eliminate entire vulnerability classes at compile time**, not runtime.

**Compile-Time Guarantees vs C/C++ Runtime Risks:**

| Vulnerability Class | C/C++ Approach | Rust Protection in FerroTunnel |
|---------------------|----------------|-------------------------------|
| **Buffer Overflows** | Manual bounds checking, easy to miss | ✅ **Compile-time bounds enforcement** - Array/slice access is always checked |
| **Use-After-Free** | Manual tracking of pointer lifetimes | ✅ **Ownership system** - Compiler prevents access after drop |
| **Double-Free** | Manual memory management, error-prone | ✅ **Impossible by design** - Each value has exactly one owner |
| **Data Races** | Mutex discipline, runtime detection | ✅ **Compile-time thread safety** - `Send`/`Sync` traits enforce safe concurrency |
| **Integer Overflows** | Silent wraparound, undefined behavior | ✅ **Checked arithmetic** - Panics in debug, wrapping explicit in release |
| **Null Pointer Dereference** | NULL checks, runtime crashes | ✅ **`Option<T>` type system** - Compiler forces explicit handling |
| **Memory Leaks** | Manual cleanup required | ✅ **RAII with Drop trait** - Automatic cleanup guaranteed |
| **Format String Bugs** | `printf` family vulnerabilities | ✅ **Type-safe formatting** - Format strings checked at compile time |

### FerroTunnel's Zero-Unsafe-Code Guarantee

**Workspace-Level Enforcement:**
```rust
// In workspace Cargo.toml
#![forbid(unsafe)]
```

This means:
- **Every single line** of FerroTunnel code is memory-safe
- **No unsafe blocks** anywhere in the codebase
- **No FFI boundaries** (except OS syscalls via libc, which is unavoidable)
- **Pure Rust dependencies** for critical paths (rustls instead of OpenSSL)

**Why This Matters:**
- OpenSSH's [CVE-2024-6387 "regreSSHion"](https://nvd.nist.gov/vuln/detail/CVE-2024-6387) was a race condition leading to RCE—**impossible in Rust** due to compile-time race detection
- OpenVPN's [CVE-2024-1305](https://nvd.nist.gov/vuln/detail/CVE-2024-1305) integer overflow → memory corruption—**prevented in Rust** by checked arithmetic
- stunnel's [CVE-2011-2940](https://nvd.nist.gov/vuln/detail/CVE-2011-2940) heap corruption—**cannot occur in Rust** due to ownership rules

### FerroTunnel Security Architecture

**Modern Cryptography (Pure Rust):**
- **TLS 1.3-only** via [rustls](https://github.com/rustls/rustls) (no C dependencies)
- **No legacy protocols** (SSLv3, TLS 1.0/1.1/1.2 disabled by default)
- **Mutual TLS (mTLS)** for client certificate authentication
- **Constant-time token comparison** (timing attack resistant)

**Defense in Depth:**
- **Token-based authentication** with SHA-256 hashing
- **Built-in rate limiting** (per-session stream and byte limits)
- **Frame size limits** (prevents memory exhaustion attacks)
- **Automated dependency scanning** (`cargo-audit` in CI/CD pipeline)
- **Supply chain security** (`cargo-deny` bans known vulnerable crates)

**Observability for Security:**
- **Prometheus metrics** for anomaly detection
- **OpenTelemetry tracing** for request inspection
- **Built-in dashboard** for real-time monitoring
- **Audit logging** for all authentication attempts

---

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
