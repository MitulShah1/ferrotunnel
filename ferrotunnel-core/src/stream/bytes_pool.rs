//! Thread-local byte buffer pooling for zero-copy data transfers
//!
//! Pools `BytesMut` buffers to avoid allocations and eliminate the
//! `Bytes::copy_from_slice()` overhead in the hot data path.

use bytes::BytesMut;
use std::cell::RefCell;

/// Maximum number of buffers to keep in the pool per thread
const MAX_POOL_SIZE: usize = 32;

/// Maximum capacity of a buffer that can be pooled (64KB)
/// Larger buffers are not pooled to avoid memory bloat
const MAX_POOLED_CAPACITY: usize = 64 * 1024;

/// Default buffer capacity for new allocations
const DEFAULT_BUFFER_CAPACITY: usize = 4096;

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
    BYTES_POOL.with(|pool| {
        pool.borrow_mut().pop().map_or_else(
            || {
                // Pool empty, allocate new buffer
                let capacity = capacity.max(DEFAULT_BUFFER_CAPACITY);
                BytesMut::with_capacity(capacity)
            },
            |mut b| {
                // Clear the buffer for reuse
                b.clear();

                // Ensure it has enough capacity
                if b.capacity() < capacity {
                    b.reserve(capacity - b.capacity());
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
    // Only pool buffers within reasonable size limits
    if bytes.capacity() <= MAX_POOLED_CAPACITY {
        BYTES_POOL.with(|pool| {
            let mut p = pool.borrow_mut();
            // Only keep up to MAX_POOL_SIZE buffers
            if p.len() < MAX_POOL_SIZE {
                p.push(bytes);
            }
            // Otherwise drop the buffer
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
        // Clear pool first
        let _ = pool_stats();

        // Fill pool beyond limit
        for _ in 0..MAX_POOL_SIZE + 10 {
            let buf = BytesMut::with_capacity(1024);
            release_bytes(buf);
        }

        let (size, _) = pool_stats();
        assert_eq!(size, MAX_POOL_SIZE); // Should not exceed limit
    }

    #[test]
    fn test_large_buffer_not_pooled() {
        // Acquire and release a large buffer
        let large_buf = BytesMut::with_capacity(MAX_POOLED_CAPACITY + 1);
        let initial_stats = pool_stats();

        release_bytes(large_buf);

        let final_stats = pool_stats();
        assert_eq!(initial_stats.0, final_stats.0); // Pool size unchanged
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
