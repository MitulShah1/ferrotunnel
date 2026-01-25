# ferrotunnel-protocol

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel-protocol)](https://crates.io/crates/ferrotunnel-protocol)
[![Documentation](https://docs.rs/ferrotunnel-protocol/badge.svg)](https://docs.rs/ferrotunnel-protocol)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](../LICENSE)

Wire protocol definitions and codec for [FerroTunnel](https://github.com/MitulShah1/ferrotunnel).

## Overview

This crate defines the binary protocol used by FerroTunnel components to communicate, including:
- Frame definitions (Control, Data)
- Serialization logic
- Tokio codecs (`TunnelCodec`)

## Usage

Internal use for building FerroTunnel compatible clients or servers.

```toml
[dependencies]
ferrotunnel-protocol = "0.4.0"
```
