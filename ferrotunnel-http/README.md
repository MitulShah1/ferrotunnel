# ferrotunnel-http

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel-http)](https://crates.io/crates/ferrotunnel-http)
[![Documentation](https://docs.rs/ferrotunnel-http/badge.svg)](https://docs.rs/ferrotunnel-http)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](../LICENSE)

HTTP ingress and proxy implementation for [FerroTunnel](https://github.com/MitulShah1/ferrotunnel).

## Overview

This crate handles the HTTP layer for FerroTunnel:
- HTTP Ingress: Receives public Internet traffic and routes it to the correct session.
- HTTP Proxy: Receives forwarded requests on the client side and proxies them to the local service.

## Usage

Internal use for HTTP handling in FerroTunnel.

```toml
[dependencies]
ferrotunnel-http = "0.4.0"
```
