# ferrotunnel-common

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel-common)](https://crates.io/crates/ferrotunnel-common)
[![Documentation](https://docs.rs/ferrotunnel-common/badge.svg)](https://docs.rs/ferrotunnel-common)

Shared types and error handling for [FerroTunnel](https://github.com/MitulShah1/ferrotunnel).

## Overview

This crate provides common utilities shared across all FerroTunnel crates:

- `TunnelError` - Comprehensive error type for tunnel operations
- `Result<T>` - Convenient result type alias

## Usage

```rust
use ferrotunnel_common::{Result, TunnelError};

fn example() -> Result<()> {
    // Your tunnel logic here
    Ok(())
}
```

## Error Types

- `Io` - I/O errors
- `Protocol` - Protocol violations
- `Authentication` - Auth failures
- `SessionNotFound` - Invalid session
- `StreamNotFound` - Invalid stream
- `Timeout` - Operation timeout
- `Config` - Configuration errors
- `Connection` - Connection failures
- `Tls` - TLS handshake or certificate errors

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
