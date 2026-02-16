//! Example: Custom Connection Pool Configuration
//!
//! This example demonstrates how to use the connection pooling API directly
//! with custom configuration for optimal performance tuning.
//!
//! # What This Shows
//!
//! - Creating an `HttpProxy` with custom `PoolConfig`
//! - Configuring pool size, timeouts, and HTTP/2 preferences
//! - Understanding the trade-offs of different pool settings
//!
//! # Usage
//!
//! This is a library API example. See the code below for how to configure
//! connection pooling in your own application.
//!
//! ```bash
//! # View the example code
//! cat examples/advanced/custom_pool_config.rs
//! ```
//!
//! # Performance Tuning Guidelines
//!
//! - **max_idle_per_host**: Higher values reduce connection setup overhead but use more memory
//!   - Default: 32 (good for most applications)
//!   - High-traffic: 64-128 (reduce latency spikes)
//!   - Memory-constrained: 8-16 (conserve resources)
//!
//! - **idle_timeout**: Balance between connection reuse and resource cleanup
//!   - Default: 90s (good balance)
//!   - Long-lived connections: 120-300s (reduce reconnection overhead)
//!   - Frequent connection changes: 30-60s (faster cleanup)
//!
//! - **prefer_h2**: Use HTTP/2 when the local service supports it
//!   - Default: false (HTTP/1.1 is more compatible)
//!   - Enable for: gRPC services, modern APIs with HTTP/2 support
//!   - Disable for: Legacy services, simple HTTP/1.1 endpoints

use ferrotunnel_http::{HttpProxy, PoolConfig};
use std::time::Duration;

fn main() {
    println!("Connection Pool Configuration Examples");
    println!("=====================================");
    println!();

    // Example 1: Default configuration (recommended for most applications)
    println!("1. Default Configuration:");
    println!("   ----------------------");
    let _default_proxy = HttpProxy::new("127.0.0.1:8080".into());
    println!("   • Max idle connections: 32");
    println!("   • Idle timeout: 90 seconds");
    println!("   • HTTP/2 preference: disabled");
    println!("   • Use case: General-purpose web applications");
    println!();

    // Example 2: High-throughput configuration
    println!("2. High-Throughput Configuration:");
    println!("   ------------------------------");
    let high_throughput_config = PoolConfig {
        max_idle_per_host: 128,
        idle_timeout: Duration::from_secs(120),
        prefer_h2: false,
    };
    let _high_throughput_proxy =
        HttpProxy::with_pool_config("127.0.0.1:8080".into(), high_throughput_config);
    println!("   • Max idle connections: 128");
    println!("   • Idle timeout: 120 seconds");
    println!("   • HTTP/2 preference: disabled");
    println!("   • Use case: High-traffic APIs, webhooks, CI/CD pipelines");
    println!("   • Benefits: Minimal latency, handles traffic spikes");
    println!("   • Trade-offs: Higher memory usage");
    println!();

    // Example 3: Memory-constrained configuration
    println!("3. Memory-Constrained Configuration:");
    println!("   ----------------------------------");
    let memory_constrained_config = PoolConfig {
        max_idle_per_host: 8,
        idle_timeout: Duration::from_secs(60),
        prefer_h2: false,
    };
    let _memory_constrained_proxy =
        HttpProxy::with_pool_config("127.0.0.1:8080".into(), memory_constrained_config);
    println!("   • Max idle connections: 8");
    println!("   • Idle timeout: 60 seconds");
    println!("   • HTTP/2 preference: disabled");
    println!("   • Use case: IoT devices, embedded systems, edge computing");
    println!("   • Benefits: Low memory footprint, fast cleanup");
    println!("   • Trade-offs: More frequent connection establishment");
    println!();

    // Example 4: HTTP/2-optimized configuration
    println!("4. HTTP/2-Optimized Configuration:");
    println!("   --------------------------------");
    let http2_config = PoolConfig {
        max_idle_per_host: 16,
        idle_timeout: Duration::from_secs(300),
        prefer_h2: true,
    };
    let _http2_proxy = HttpProxy::with_pool_config("127.0.0.1:50051".into(), http2_config);
    println!("   • Max idle connections: 16");
    println!("   • Idle timeout: 300 seconds (5 minutes)");
    println!("   • HTTP/2 preference: enabled");
    println!("   • Use case: gRPC services, HTTP/2 APIs");
    println!("   • Benefits: Single multiplexed connection, efficient");
    println!("   • Note: Requires local service to support HTTP/2");
    println!();

    // Example 5: Short-lived connections
    println!("5. Short-Lived Connection Configuration:");
    println!("   --------------------------------------");
    let short_lived_config = PoolConfig {
        max_idle_per_host: 4,
        idle_timeout: Duration::from_secs(30),
        prefer_h2: false,
    };
    let _short_lived_proxy =
        HttpProxy::with_pool_config("127.0.0.1:3000".into(), short_lived_config);
    println!("   • Max idle connections: 4");
    println!("   • Idle timeout: 30 seconds");
    println!("   • HTTP/2 preference: disabled");
    println!("   • Use case: Development, testing, frequently changing backends");
    println!("   • Benefits: Fast resource cleanup, low overhead");
    println!();

    println!("Key Takeaways:");
    println!("-------------");
    println!("• Start with default settings for most applications");
    println!("• Increase pool size for high-traffic scenarios");
    println!("• Decrease pool size for memory-constrained environments");
    println!("• Enable HTTP/2 preference only when local service supports it");
    println!("• Monitor connection metrics to fine-tune your configuration");
    println!();
    println!("For more information, see:");
    println!("  cargo doc --open -p ferrotunnel-http");
}
