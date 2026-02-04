//! Thread-local byte buffer pooling for zero-copy data transfers
//!
//! Pools `BytesMut` buffers to avoid allocations and eliminate the
//! `Bytes::copy_from_slice()` overhead in the hot data path.
//!
//! Pool limits are configurable via environment variables (read once at first use):
//! - `FERROTUNNEL_POOL_MAX_SIZE`: max buffers per thread (default: 32)
//! - `FERROTUNNEL_POOL_MAX_CAPACITY_BYTES`: max capacity of a pooled buffer in bytes (default: 65536)
//! - `FERROTUNNEL_POOL_DEFAULT_CAPACITY_BYTES`: default capacity for new buffers (default: 4096)

use bytes::BytesMut;
use std::cell::RefCell;
use std::sync::OnceLock;

/// Default maximum number of buffers per thread
const DEFAULT_MAX_POOL_SIZE: usize = 32;

/// Default maximum capacity of a buffer that can be pooled (64KB)
const DEFAULT_MAX_POOLED_CAPACITY: usize = 64 * 1024;

/// Default buffer capacity for new allocations
const DEFAULT_BUFFER_CAPACITY: usize = 4096;

/// Pool configuration (loaded from env on first use).
struct PoolConfig {
    max_pool_size: usize,
    max_pooled_capacity: usize,
    default_buffer_capacity: usize,
}

fn load_config() -> PoolConfig {
    let max_pool_size = std::env::var("FERROTUNNEL_POOL_MAX_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n: &usize| n > 0)
        .unwrap_or(DEFAULT_MAX_POOL_SIZE);

    let max_pooled_capacity = std::env::var("FERROTUNNEL_POOL_MAX_CAPACITY_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n: &usize| n > 0)
        .unwrap_or(DEFAULT_MAX_POOLED_CAPACITY);

    let default_buffer_capacity = std::env::var("FERROTUNNEL_POOL_DEFAULT_CAPACITY_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n: &usize| n > 0)
        .unwrap_or(DEFAULT_BUFFER_CAPACITY);

    PoolConfig {
        max_pool_size,
        max_pooled_capacity,
        default_buffer_capacity,
    }
}

static CONFIG: OnceLock<PoolConfig> = OnceLock::new();

fn config() -> &'static PoolConfig {
    CONFIG.get_or_init(load_config)
}

thread_local! {
    /// Thread-local pool of reusable byte buffers
    static BYTES_POOL: RefCell<Vec<BytesMut>> = const { RefCell::new(Vec::new()) };
}

/// Acquire a `BytesMut` buffer from the thread-local pool
///
/// If the pool is empty, allocates a new buffer with the requested capacity.
/// Otherwise, returns a pooled buffer (cleared) with at least the requested capacity.
///
/// # Arguments
/// * `capacity` - Minimum capacity required for the buffer
///
/// # Returns
/// A `BytesMut` buffer with at least the requested capacity
///
/// # Examples
/// ```ignore
/// let mut buf = acquire_bytes(1024);
/// buf.extend_from_slice(b"hello");
/// let data = buf.freeze();
/// ```
pub fn acquire_bytes(capacity: usize) -> BytesMut {
    let min_cap = capacity.max(config().default_buffer_capacity);
    BYTES_POOL.with(|pool| {
        pool.borrow_mut().pop().map_or_else(
            || {
                // Pool empty, allocate new buffer
                BytesMut::with_capacity(min_cap)
            },
            |mut b| {
                // Clear the buffer for reuse
                b.clear();

                // Ensure it has enough capacity
                if b.capacity() < min_cap {
                    b.reserve(min_cap - b.capacity());
                }

                b
            },
        )
    })
}

/// Release a `BytesMut` buffer back to the thread-local pool
///
/// Buffers larger than `MAX_POOLED_CAPACITY` are dropped instead of pooled
/// to prevent memory bloat. If the pool is full, the buffer is dropped.
///
/// # Arguments
/// * `bytes` - The buffer to return to the pool
///
/// # Examples
/// ```ignore
/// let buf = acquire_bytes(1024);
/// // ... use buffer ...
/// release_bytes(buf);
/// ```
pub fn release_bytes(bytes: BytesMut) {
    let cfg = config();
    if bytes.capacity() <= cfg.max_pooled_capacity {
        BYTES_POOL.with(|pool| {
            let mut p = pool.borrow_mut();
            if p.len() < cfg.max_pool_size {
                p.push(bytes);
            }
        });
    }
}

/// Get current pool statistics (for monitoring/debugging)
///
/// Returns (`pool_size`, `total_capacity`)
pub fn pool_stats() -> (usize, usize) {
    BYTES_POOL.with(|pool| {
        let p = pool.borrow();
        let size = p.len();
        let total_capacity = p.iter().map(BytesMut::capacity).sum();
        (size, total_capacity)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_release() {
        // Acquire a buffer
        let mut buf = acquire_bytes(1024);
        assert!(buf.capacity() >= 1024);

        // Use it
        buf.extend_from_slice(b"test data");
        assert_eq!(&buf[..], b"test data");

        // Release it
        release_bytes(buf);

        // Acquire again - should get the same buffer (cleared)
        let buf2 = acquire_bytes(512);
        assert!(buf2.is_empty()); // Should be cleared
        assert!(buf2.capacity() >= 1024); // Should have previous capacity
    }

    #[test]
    fn test_pool_size_limit() {
        let _ = pool_stats();
        let limit = std::env::var("FERROTUNNEL_POOL_MAX_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_MAX_POOL_SIZE);

        for _ in 0..limit + 10 {
            let buf = BytesMut::with_capacity(1024);
            release_bytes(buf);
        }

        let (size, _) = pool_stats();
        assert_eq!(size, limit);
    }

    #[test]
    fn test_large_buffer_not_pooled() {
        let max_cap = std::env::var("FERROTUNNEL_POOL_MAX_CAPACITY_BYTES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_MAX_POOLED_CAPACITY);

        let large_buf = BytesMut::with_capacity(max_cap + 1);
        let initial_stats = pool_stats();

        release_bytes(large_buf);

        let final_stats = pool_stats();
        assert_eq!(initial_stats.0, final_stats.0);
    }

    #[test]
    fn test_capacity_expansion() {
        let buf = acquire_bytes(100);
        release_bytes(buf);

        // Request larger capacity
        let buf2 = acquire_bytes(2048);
        assert!(buf2.capacity() >= 2048);
    }

    #[test]
    fn test_concurrent_usage() {
        // Test that thread-local works correctly
        let buf1 = acquire_bytes(1024);
        let buf2 = acquire_bytes(1024);

        // Both should be valid buffers
        assert!(buf1.capacity() >= 1024);
        assert!(buf2.capacity() >= 1024);

        release_bytes(buf1);
        release_bytes(buf2);

        let (size, _) = pool_stats();
        assert_eq!(size, 2);
    }
}
