//! `FerroTunnel` End-to-End Benchmarks
//!
//! This crate contains benchmarks that test the complete tunnel stack,
//! measuring real-world performance characteristics.
//!
//! ## Benchmarks
//!
//! - **`e2e_tunnel`** - Complete tunnel setup/teardown and request handling
//! - **`throughput`** - Raw throughput measurements for data transfer
//!
//! ## Running
//!
//! ```bash
//! # Run all benchmarks
//! cargo bench -p ferrotunnel-benches
//!
//! # Run specific benchmark
//! cargo bench -p ferrotunnel-benches --bench e2e_tunnel
//!
//! # Save baseline
//! cargo bench -p ferrotunnel-benches -- --save-baseline main
//!
//! # Compare to baseline
//! cargo bench -p ferrotunnel-benches -- --baseline main
//! ```
