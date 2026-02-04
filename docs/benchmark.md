# FerroTunnel Benchmark & Performance

This document covers FerroTunnel's performance characteristics, comparison benchmarks against alternatives, and explains the architectural trade-offs that inform these results.

---

## TL;DR

| Metric | FerroTunnel | Rathole | frp |
|--------|-------------|---------|-----|
| **Throughput** | 382 MB/s | 1349 MB/s | 690 MB/s |
| **Latency (P99)** | 0.114ms | 0.075ms | 0.131ms |
| **Memory/conn** | 47.3 KB | 35.8 KB | 113.7 KB |
| **Architecture** | Multiplexed | 1:1 TCP | 1:1 TCP |

FerroTunnel is **slower in raw throughput** but provides **ngrok-like capabilities** (multiplexing, HTTP routing, plugins, dashboard) that rathole and frp don't offer.

---

## 1. Why FerroTunnel is Different

### 1.1 The Architecture Spectrum

```
Simple & Fast                                    Feature-Rich
     │                                                │
     ▼                                                ▼
 ┌────────┐      ┌─────────┐      ┌──────────────────────────────┐
 │ rathole│      │   frp   │      │ ngrok / Cloudflare / FerroTunnel │
 └────────┘      └─────────┘      └──────────────────────────────┘
     │                │                        │
   1:1 TCP         1:1 TCP            Multiplexed Streams
   Minimal         Some features      HTTP routing, plugins,
   overhead        (dashboard)        dashboard, observability
```

### 1.2 What rathole/frp Do

**rathole** and **frp** use a **1:1 TCP forwarding model**:

```
[Client Request] ──TCP──▶ [Tunnel Server] ──NEW TCP──▶ [Tunnel Client] ──TCP──▶ [Local Service]
```

- Each incoming connection spawns a **dedicated TCP connection** through the tunnel
- Minimal protocol overhead nearly raw TCP passthrough
- **Strength**: Maximum throughput, lowest latency
- **Limitation**: No stream multiplexing, limited HTTP awareness, no plugins

### 1.3 What FerroTunnel Does (ngrok/Cloudflare Tunnel Model)

FerroTunnel uses a **multiplexed stream model** over a single control connection the same approach used by [ngrok](https://ngrok.com/docs/http/) (persistent TLS connection routing multiple concurrent requests) and [Cloudflare Tunnel](https://developers.cloudflare.com/speed/optimization/protocol/http2-to-origin/) (HTTP/2 multiplexing with up to 200 concurrent streams per connection):

```
                        ┌─────────────────────────────────────┐
[Request 1] ──┐         │        Single TCP Connection        │         ┌──▶ [Service A]
[Request 2] ──┼──MUX───▶│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐   │──DEMUX──┼──▶ [Service B]
[Request 3] ──┘         │  │ S:1 │ │ S:2 │ │ S:3 │ │ S:n │   │         └──▶ [Service C]
                        │  └─────┘ └─────┘ └─────┘ └─────┘   │
                        └─────────────────────────────────────┘
```

**Why this matters:**

| Capability | rathole/frp | FerroTunnel |
|------------|-------------|-------------|
| Multiple services on one tunnel | ❌ Need separate tunnels | ✅ Single connection |
| HTTP Host-based routing | ❌ | ✅ Route by hostname |
| Request/response plugins | ❌ | ✅ Auth, rate-limit, logging |
| Real-time dashboard | ❌ (frp has basic) | ✅ Full WebUI |
| Embeddable library | ❌ | ✅ `Client::builder()` API |
| NAT with many services | ❌ Opens many ports | ✅ One port, many services |

### 1.4 The Trade-off

Multiplexing adds overhead:

1. **Frame encoding/decoding**: Each data chunk is wrapped in a frame with stream ID, length, flags
2. **Stream demultiplexing**: DashMap lookups, channel dispatch per frame
3. **Ordering guarantees**: Virtual streams maintain order within the multiplexed connection
4. **Memory buffers**: Per-stream channels (128-depth) for flow control

This overhead is the **cost of features**. For users who need:
- Just TCP forwarding → **Use rathole** (fastest)
- HTTP routing + plugins + dashboard → **Use FerroTunnel** (feature-rich)

---

## 2. Comparison Benchmarks

### 2.1 Test Environment

| Parameter | Value |
|-----------|-------|
| **OS** | Ubuntu (Kernel 6.14.0-37-generic) |
| **CPU** | Intel Core i5-10400H @ 2.60GHz (8 cores) |
| **Memory** | 31 GB |
| **Test Mode** | Loopback (127.0.0.1) CPU-bound |
| **Encryption** | Disabled (fair comparison) |
| **Compression** | Disabled |

### 2.2 HTTP Throughput

Transfer of 100MB payload through the tunnel:

| Tunnel | Duration | Throughput | Relative | Notes |
|--------|----------|------------|----------|-------|
| Rathole 0.5.0 | 0.07s | **1349 MB/s** | 1.00× | 1:1 TCP, minimal overhead |
| frp 0.66.0 | 0.14s | 690 MB/s | 0.51× | 1:1 TCP, Go runtime |
| FerroTunnel 0.1.0 | 0.26s | 382 MB/s | 0.28× | Multiplexed, frame overhead |

**Analysis**: FerroTunnel's throughput gap is primarily due to:
- Frame encode/decode per chunk (length-prefixed protocol)
- Per-stream channel dispatch through `DashMap`
- Larger per-frame memory allocations

### 2.3 TCP Bitrate (Multiplexed Streams)

10 concurrent streams, 10MB each:

| Tunnel | Aggregate Throughput | Per-Stream | Notes |
|--------|---------------------|------------|-------|
| Rathole | **3106 MB/s** | 310.6 MB/s | Parallel TCP connections |
| frp | 1087 MB/s | 108.7 MB/s | Parallel TCP connections |
| FerroTunnel | 451 MB/s | 45.1 MB/s | True multiplexing over 1 conn |

**Key insight**: Rathole/frp spawn 10 separate TCP connections. FerroTunnel multiplexes all 10 streams over a **single connection** different trade-off, not apples-to-apples.

### 2.4 Latency (10,000 requests, 64-byte payload)

| Tunnel | P50 | P90 | P99 | P99.9 | Mean |
|--------|-----|-----|-----|-------|------|
| Rathole | **0.050ms** | 0.061ms | 0.075ms | 0.092ms | 0.051ms |
| FerroTunnel | 0.078ms | 0.094ms | 0.114ms | 0.261ms | 0.080ms |
| frp | 0.096ms | 0.110ms | 0.131ms | 0.170ms | 0.098ms |

**Analysis**: FerroTunnel is 56% slower than rathole but **18% faster than frp**. The gap vs rathole is frame overhead; the win vs frp is Rust vs Go runtime efficiency.

### 2.5 Memory Efficiency (1000 concurrent connections)

| Tunnel | Peak Memory | Per Connection | Success Rate |
|--------|-------------|----------------|--------------|
| Rathole | **35.8 MB** | 35.8 KB | 100% |
| FerroTunnel | 47.3 MB | 47.3 KB | 100% |
| frp | 113.7 MB | 113.7 KB | 100% |

**Key wins**:
- FerroTunnel uses **58% less memory** than frp
- Competitive with rathole despite multiplexer overhead
- Excellent for resource-constrained devices (Raspberry Pi, routers)

---

## 3. Internal Profiling

### 3.1 Startup Performance

| Metric | Result | Status |
|--------|--------|--------|
| **Startup Time** | < 50ms | ✅ Optimized |
| **Time to Accept Connections** | < 10ms | ✅ |

Server binds TCP/HTTP listeners **before** initializing plugins for instant readiness.

### 3.2 Memory Profile (Heaptrack)

| Component | Peak Heap | Context |
|-----------|-----------|---------|
| **Server** | 78.66 KB | 20 concurrent connections |
| **Client** | 77.89 KB | Forwarding traffic |

![Server Heap Consumption](static/server_heap_graph.png)
*Server memory usage over time note the flat line indicating no memory accumulation.*

Dominant allocations:
1. Thread Local Storage (glibc): ~256B per thread
2. Tokio Runtime Init: ~256B (once)
3. HTTP Connection Buffer: ~128B (pooled)

![Top Allocations](static/top_allocations.png)
*Top allocators showing minimal heavy objects.*

### 3.3 System Call Distribution (strace, 50 connections)

| Syscall | % Time | Description |
|---------|--------|-------------|
| `write` | ~51% | Sending data (expected for proxy) |
| `futex` | ~43% | Thread sync (Tokio runtime) |
| `read` | ~5% | Reading from sockets |

---

## 4. Optimization Roadmap

### Completed
- ✅ Lock-free stream dispatch (`DashMap`)
- ✅ Buffer pooling (`ObjectPool<Vec<u8>>`)
- ✅ Larger channel capacity (128) to reduce backpressure
- ✅ Fast startup (listeners bind before plugins)

### In Progress
- 🔄 Vectorized I/O (`writev`) batch multiple frames per syscall
- 🔄 Zero-copy frame paths for large payloads

### Planned
- 📋 `mimalloc` allocator for reduced fragmentation
- 📋 io_uring on Linux for reduced syscall overhead
- 📋 Connection pooling for upstream services

---

## 5. Benchmark Methodology

All benchmarks follow these principles:

| Principle | Implementation |
|-----------|----------------|
| **Accuracy** | HDR Histogram (3 sig figs), 1µs–60s range |
| **Fairness** | Identical conditions, encryption/compression disabled |
| **Reproducibility** | Docker-based, version-pinned environment |
| **Warmup** | 1000 requests excluded from latency measurements |
| **TCP_NODELAY** | Enabled on all sockets |


---

*Benchmarks run: 2026-02-03 | FerroTunnel 0.1.0, Rathole 0.5.0, frp 0.66.0*
