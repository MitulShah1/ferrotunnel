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

- **v1.0.0** - Current stable release ✅
  - Protocol, tunnel, HTTP/TCP ingress, plugin system, observability, dashboard, unified CLI
  - Published to crates.io

### Planned

> **Strategy**: Prioritize features that maximize user adoption and "time to first success"

- **v1.0.1** - Stability & Developer Experience
  - Enhanced documentation with real-world integration examples
  - Simplified deployment: Docker images, Homebrew formula
  - Performance benchmarks vs. alternatives (rathole, frp)
  - Bug fixes and polish from early adopter feedback
  - **Goal**: Convert evaluators → users → advocates

- **v1.0.2** - WebSocket Tunneling
  - Full WebSocket tunnel support
  - Real-time application compatibility (chat, dashboards, gaming)
  - **Market Impact**: Opens to entire real-time application developer segment

- **v1.0.3** - HTTP/2 Support
  - HTTP/2 ingress and client proxy
  - Multiplexing and header compression
  - **Value**: Modern web baseline, enterprise credibility

- **v1.0.4** - gRPC Support
  - Native gRPC tunneling
  - **Target Audience**: Enterprise and microservices developers

- **v1.0.5** - Connection Pooling
  - Upstream and client connection pooling
  - Performance optimization for high-throughput scenarios

- **v1.0.6** - QUIC Transport (HTTP/3)
  - QUIC protocol support for reduced latency
  - **Differentiator**: Next-gen transport for competitive advantage

- **v1.0.7** - Multi-region Support
  - Geographic load balancing
  - Regional failover capabilities

- **v1.0.8** - Custom Domains
  - Custom domain mapping for white-label deployments

- **v2.0.0** - Breaking Changes (if needed)
  - Protocol improvements based on v1.x learnings

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
