# FerroTunnel Troubleshooting Guide

## Connection Issues

### Client Can't Connect

**Symptoms**: Connection refused, timeout, or immediate disconnect

**Checklist**:
1. Verify server is running: `systemctl status ferrotunnel-server`
2. Check server logs: `journalctl -u ferrotunnel-server -f`
3. Verify port is open: `nc -zv server.example.com 8443`
4. Check firewall rules: `iptables -L -n | grep 8443`

**Common causes**:
- Server not listening on expected address
- Firewall blocking port
- Token mismatch

### TLS Handshake Failures

**Symptoms**: "handshake failed", "certificate verify failed"

**Checklist**:
1. Verify certificate is valid: `openssl x509 -in server.crt -noout -dates`
2. Check hostname matches: `openssl x509 -in server.crt -noout -text | grep DNS`
3. Verify CA is correct: `openssl verify -CAfile ca.crt server.crt`

**Common errors**:

| Error | Cause | Solution |
|-------|-------|----------|
| `certificate has expired` | Cert expired | Renew certificate |
| `hostname mismatch` | Wrong server_name | Add SAN or fix server_name |
| `unknown CA` | CA not trusted | Provide correct ca_cert_path |
| `self-signed certificate` | No CA provided | Add --tls-ca option |

### Frequent Disconnections

**Symptoms**: Connection drops every few minutes

**Checklist**:
1. Check heartbeat interval (default: 30s)
2. Look for network issues: `ping server.example.com`
3. Check server resource limits
4. Review client reconnection logs

**Solutions**:
- Increase session_timeout on server
- Check for NAT/firewall idle timeouts
- Enable TCP keepalive

## Authentication Errors

### Invalid Token

**Symptoms**: "Handshake rejected: InvalidToken"

**Causes**:
- Token mismatch between client and server
- Token contains invalid characters
- Token exceeds maximum length

**Debug**:
```bash
# Check token length
echo -n "$FERROTUNNEL_TOKEN" | wc -c

# Verify no hidden characters
echo -n "$FERROTUNNEL_TOKEN" | xxd | head
```

### Token Too Long

**Symptoms**: "token too long: X bytes exceeds limit"

**Solution**: Use a shorter token (max 256 bytes default)

## Performance Issues

### High Latency

**Symptoms**: Requests take longer than expected

**Checklist**:
1. Measure baseline: `ping server.example.com`
2. Check server CPU/memory: `top -p $(pgrep ferrotunnel)`
3. Review concurrent connections
4. Check for rate limiting

**Solutions**:
- Reduce max_streams_per_session if overloaded
- Increase server resources
- Use connection pooling

### Low Throughput

**Symptoms**: Slow file transfers, buffering

**Checklist**:
1. Check rate limits: `bytes_per_sec` setting
2. Measure network bandwidth: `iperf3 -c server`
3. Check frame size limits

**Solutions**:
```toml
[rate_limit]
bytes_per_sec = 104857600  # 100MB/s

[limits]
max_frame_bytes = 16777216  # 16MB
```

### Memory Growth

**Symptoms**: Server memory usage increases over time

**Checklist**:
1. Check active sessions: metrics or logs
2. Look for session leaks
3. Review max_sessions limit

**Solutions**:
- Lower session_timeout to clean up faster
- Reduce max_streams_per_session
- Restart server periodically (if leak suspected)

## Error Messages

### Common Errors

| Error | Meaning | Solution |
|-------|---------|----------|
| `max sessions reached` | Too many clients | Increase max_sessions or wait |
| `max streams reached` | Session overloaded | Reduce concurrent requests |
| `rate limited` | Too many requests | Slow down or increase limits |
| `frame too large` | Payload exceeds limit | Reduce payload or increase max_frame_bytes |
| `circuit breaker open` | Backend unhealthy | Fix backend, wait for recovery |

### Protocol Errors

| Error | Meaning | Solution |
|-------|---------|----------|
| `expected handshake` | Wrong protocol | Verify client/server versions match |
| `unsupported version` | Protocol mismatch | Update client or server |
| `connection closed` | Remote closed | Check remote logs |

## Debugging

### Enable Debug Logging

```bash
# Info level (default)
RUST_LOG=info ferrotunnel-server ...

# Debug level (more detail)
RUST_LOG=debug ferrotunnel-server ...

# Trace level (very verbose)
RUST_LOG=trace ferrotunnel-server ...

# Specific module
RUST_LOG=ferrotunnel_core::tunnel=debug ferrotunnel-server ...
```

### Network Capture

```bash
# Capture tunnel traffic
tcpdump -i eth0 port 8443 -w tunnel.pcap

# Analyze with tshark
tshark -r tunnel.pcap -Y 'tcp.port == 8443'
```

### Connection Testing

```bash
# Test TCP connectivity
nc -zv server.example.com 8443

# Test TLS
openssl s_client -connect server.example.com:8443 -servername server.example.com

# Test with curl (if HTTP)
curl -v --cacert ca.crt https://server.example.com:8443/
```

## FAQ

### Q: How many connections can one server handle?

A: Depends on resources. With default limits:
- ~1000 sessions
- ~100 streams per session
- ~100,000 total streams

Memory usage: roughly 100KB per session + 10KB per stream.

### Q: Can I run multiple servers?

A: Yes, use a TCP load balancer. Sessions are independent, so any server can handle any client.

### Q: How do I rotate tokens without downtime?

A: 
1. Add new token to server (if supported)
2. Update clients to use new token
3. Remove old token from server

### Q: Why does reconnection take so long?

A: Exponential backoff prevents overwhelming the server. After failures:
- 1st retry: ~1s
- 2nd retry: ~2s
- 3rd retry: ~4s
- Max: 60s

### Q: How do I know if I'm being rate limited?

A: Check logs for "rate limited" messages. Monitor:
- `ferrotunnel_rate_limited_total` metric
- HTTP 429 responses (if applicable)
