# ferrotunnel-server

[![Crates.io](https://img.shields.io/crates/v/ferrotunnel-server)](https://crates.io/crates/ferrotunnel-server)
[![Documentation](https://docs.rs/ferrotunnel-server/badge.svg)](https://docs.rs/ferrotunnel-server)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](../LICENSE)

The official server binary for [FerroTunnel](https://github.com/MitulShah1/ferrotunnel).

## Overview

This binary runs the FerroTunnel control plane and HTTP ingress, accepting connections from clients and routing Internet traffic to them.

## Installation

```bash
cargo install ferrotunnel-server
```

## Usage

```bash
ferrotunnel-server --bind 0.0.0.0:7835 --http-bind 0.0.0.0:8080 --token my-secret
```
