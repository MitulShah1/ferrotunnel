# ferrotunnel-client

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel-client)](https://crates.io/crates/ferrotunnel-client)
[![Documentation](https://docs.rs/ferrotunnel-client/badge.svg)](https://docs.rs/ferrotunnel-client)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](../LICENSE)

The official CLI client for [FerroTunnel](https://github.com/MitulShah1/ferrotunnel).

## Overview

This binary connects a local service to a FerroTunnel server, exposing it to the Internet.

## Installation

```bash
cargo install ferrotunnel-client
```

## Usage

```bash
ferrotunnel-client --server tunnel.example.com --token my-secret --local-addr 127.0.0.1:8080
```
