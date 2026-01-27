# FerroTunnel Review & Gap Analysis

## Goal
To make FerroTunnel the "best" project among competitors like ngrok, Rathole, and Cloudflare Tunnel.

## Competitive Analysis

| Feature | FerroTunnel (Current) | ngrok | Rathole | Gap Severity |
|---------|-----------------------|-------|---------|--------------|
| **Routing** | ❌ **Unsafe/Random** | Subdomain-based | Token/Config based | 🚨 **CRITICAL** |
| **Performance** | ⚠️ **Buffered** | Streaming | High-perf Streaming | 🚨 **CRITICAL** |
| **Observability** | ❌ Missing | Real-time Dashboard | Basic Logging | 🟠 HIGH |
| **Tenancy** | Single-tenant (effective) | Multi-tenant | Multi-tenant | 🚨 **CRITICAL** |

## Critical Bugs & Issues Identified

### 1. Insecure Global Routing (Multi-tenancy Failure)
**Location:** `ferrotunnel-http/src/ingress.rs:136`

```rust
// 2. Identify Target Session
let Some(multiplexer) = sessions.find_multiplexer() else {
    return Ok(full_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "No active tunnels",
    ));
};
```

**Issue:**
The ingress server currently retrieves *any* available multiplexer using `sessions.find_multiplexer()`. It does **not** check if the incoming request's `Host` header matches the tunnel's assigned domain or ID.

**Impact:**
- If User A and User B both connect their tunnels to the server, a request intended for User A (e.g., `user-a.tunnel.com`) might be routed to User B's local server.
- This creates a **Cross-Tenant Data Leak** vulnerability.
- Only one tunnel can effectively operate on the server at a time without race conditions.

**Remediation:**
- Implement a `SessionMap` that maps `tunnel_id` (subdomain) to specific `Session/Multiplexer`.
- In `ingress.rs`, extract the subdomain from the `Host` header and look up the *specific* session.

### 2. Full Request Buffering (DoS Vector & Performance Bottleneck)
**Location:** `ferrotunnel-http/src/ingress.rs:90`

```rust
// Buffer Request Body for Plugins
let (parts, body) = req.into_parts();
let body_bytes = body.collect().await?.to_bytes();
let mut plugin_req = Request::from_parts(parts.clone(), body_bytes.to_vec());
```

**Issue:**
The server buffers the **entire HTTP request body** into RAM before sending it to plugins or forwarding it to the tunnel.

**Impact:**
- **Denial of Service (DoS):** An attacker can send a large request (e.g., 5GB upload) and exhaust the server's memory.
- **High Latency:** The byte transfer to the client doesn't start until the entire upload is received.
- **Incompatibility:** Cannot support large file transfers or streaming protocols efficiently.

**Remediation:**
- Refactor the Plugin API to support **Streaming Bodies**.
- Plugins should inspect headers first. If body inspection is required, only *then* buffer (with limits), or use a streaming inspection tap.
- Default path should stream bytes `Ingress -> Tunnel -> Client` without holding the full blob.

## Missing Key Features

### 3. Observability Dashboard (The "ngrok" Factor)
**Status:** Planned (Phase 6) but currently missing.

**Gap:**
One of ngrok's most beloved features is the web interface (`http://localhost:4040`) that allows developers to:
- Replay requests.
- Inspect JSON bodies and headers.
- See error rates in real-time.

**Remediation:**
- Prioritize the implementation of `ferrotunnel-observability`.
- Capture request/response metadata (not bodies, unless opted-in) and serve via a local API.

## Strategic Recommendation

To claim the title of "Best", the immediate priority must be fixing the **Critical Bugs** to ensure the system is secure and usable for more than one person.

1.  **Fix Routing:** Ensure `host -> tunnel` mapping is strict.
2.  **Fix Buffering:** Switch to streaming proxy logic.
3.  **Build Dashboard:** Implement the web UI for developer joy.
