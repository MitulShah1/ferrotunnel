# AGENTS.md - FerroTunnel

## Commands
- **Build**: `cargo build --workspace` or `make build`
- **Test all**: `cargo test --workspace --all-features` or `make test`
- **Test single**: `cargo test -p <crate> <test_name>` (e.g., `cargo test -p ferrotunnel-protocol codec_tests`)
- **Lint**: `cargo clippy --workspace --all-targets --all-features -- -D warnings` or `make lint`
- **Format**: `cargo fmt --all` or `make fmt`
- **Check**: `make check` (runs fmt check + clippy)

## Architecture
Rust workspace (tokio-style) with crates: `ferrotunnel` (main API), `ferrotunnel-protocol` (wire protocol), `ferrotunnel-core` (tunnel logic), `ferrotunnel-http` (HTTP ingress/proxy), `ferrotunnel-client`/`ferrotunnel-server` (binaries), `ferrotunnel-plugin`, `ferrotunnel-observability`, `ferrotunnel-common` (shared errors). Tools in `tools/loadgen` and `tools/soak`.

## Code Style
- Edition 2021, MSRV 1.75, max line width 100, 4-space indent
- `unsafe_code = "forbid"` - no unsafe code allowed
- Use `thiserror` for error types, `anyhow` for application errors
- Avoid `.unwrap()` and `.expect()` (allowed in tests only)
- Use `?` operator (`use_try_shorthand`), field init shorthand
- Prefer `Bytes` for zero-copy buffers, `tokio` for async runtime
- Clippy pedantic enabled; `dbg!`, `todo!`, `unimplemented!` emit warnings
