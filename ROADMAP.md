# FerroTunnel Development Roadmap

Embeddable, extensible, and observable reverse tunnel for Rust developers.

---

## Vision

**FerroTunnel** is not just another tunnel - it's the **embeddable**, **extensible**, and **observable** reverse tunnel for Rust developers.

### Core Differentiators

🎯 **Library-First** - Published to crates.io, embedded in your apps
🎯 **Plugin System** - Trait-based extensibility for custom behavior
🎯 **Built-in Dashboard** - Real-time WebUI for monitoring

---

## Version Strategy

### Stable

- **v0.1.0** - Current stable release ✅
  - Protocol, tunnel, HTTP/TCP ingress, plugin system, observability, dashboard, unified CLI
  - Published to crates.io

### Future (post v0.1.0)

- **v0.2.0** - HTTP/2 support (ingress & client)
- **v0.3.0** - gRPC support
- **v0.4.0** - QUIC transport (HTTP/3)
- **v0.5.0** - Connection pooling (upstream/client)
- **v0.6.0** - WebSocket tunneling
- **v0.7.0** - Multi-region support
- **v0.8.0** - Custom domains
- **v1.0.0** - Breaking changes if needed

---

## Comparison with Alternatives

| Feature | Rathole | frp | FerroTunnel |
|---------|---------|-----|-------------|
| Language | Rust | Go | Rust |
| Embeddable | ❌ | ❌ | ✅ crates.io library |
| Plugin System | ❌ | Limited | ✅ Trait-based |
| Dashboard | ❌ | Basic | ✅ Built-in WebUI |
| Request Inspector | ❌ | ❌ | ✅ Built-in |
| OpenTelemetry | ❌ | ❌ | ✅ Built-in |
| Memory Efficiency | — | ~300MB/1k tunnels | ~100MB/1k tunnels |
| License | Apache-2.0 | Apache-2.0 | MIT OR Apache-2.0 |

---

## Success Metrics

### Technical Targets

- **Performance**: < 5ms latency overhead vs raw TCP
- **Scalability**: 10k concurrent streams per server
- **Efficiency**: < 100MB memory for 1000 tunnels
- **Reliability**: Zero crashes in 7-day soak test

### Differentiation Validation

- ✅ **Only embeddable** Rust tunnel (crates.io)
- ✅ **Most extensible** via plugin system
- ✅ **Best observability** with built-in dashboard

---

## Development Workflow

### Branch Strategy

- `main` - Stable, tagged releases
- `develop` - Integration branch
- `feature/*` - Feature branches
- `fix/*` - Bug fix branches

### Release Process

1. Development on `feature/*` branches
2. Merge to `develop` via PR
3. Integration testing on `develop`
4. Tag release from `develop` → `main`
5. Publish to crates.io
6. Create GitHub release

### CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
- Cargo check
- Cargo test (all features)
- Cargo clippy (deny warnings)
- Cargo fmt --check
- Cargo audit (dependency security)
- Cargo doc (documentation build)
- Coverage report (codecov)
```
