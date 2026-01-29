//! Lock-free object pool for reusing allocations
//!
//! Uses crossbeam's `ArrayQueue` for high-performance pooling.

use crossbeam_queue::ArrayQueue;
use std::sync::Arc;

/// Default pool capacity - enough for typical concurrent stream count
const DEFAULT_POOL_CAPACITY: usize = 256;

/// Poolable trait for objects that can be reset and reused
pub trait Poolable: Sized {
    /// Reset the object to a clean state for reuse
    fn reset(&mut self);
}

/// A high-performance object pool using lock-free queue
///
/// The pool attempts to reuse objects when possible, falling back to
/// creating new instances when the pool is empty. Objects are returned
/// to the pool when dropped via `PooledObject`.
#[derive(Debug)]
pub struct ObjectPool<T: Poolable> {
    queue: Arc<ArrayQueue<T>>,
    capacity: usize,
}

impl<T: Poolable> ObjectPool<T> {
    /// Create a new pool with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Arc::new(ArrayQueue::new(capacity)),
            capacity,
        }
    }

    /// Create a pool with default capacity (256)
    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_POOL_CAPACITY)
    }

    /// Acquire an object from the pool, or None if pool is empty
    pub fn try_acquire(&self) -> Option<T> {
        self.queue.pop()
    }

    /// Release an object back to the pool
    ///
    /// The object is reset before being added to the pool.
    /// If the pool is full, the object is dropped.
    pub fn release(&self, mut obj: T) {
        obj.reset();
        // Ignore push failure if pool is full - object will be dropped
        let _ = self.queue.push(obj);
    }

    /// Current number of pooled objects
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Check if pool is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Pool capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get a clone of the internal queue for sharing
    pub fn queue(&self) -> Arc<ArrayQueue<T>> {
        Arc::clone(&self.queue)
    }
}

impl<T: Poolable> Clone for ObjectPool<T> {
    fn clone(&self) -> Self {
        Self {
            queue: Arc::clone(&self.queue),
            capacity: self.capacity,
        }
    }
}

impl<T: Poolable> Default for ObjectPool<T> {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

/// A wrapper that returns the object to the pool when dropped
pub struct PooledObject<T: Poolable> {
    obj: Option<T>,
    pool: Arc<ArrayQueue<T>>,
}

impl<T: Poolable> PooledObject<T> {
    /// Create a new pooled object wrapper
    pub fn new(obj: T, pool: Arc<ArrayQueue<T>>) -> Self {
        Self {
            obj: Some(obj),
            pool,
        }
    }

    /// Take ownership of the inner object, preventing return to pool
    ///
    /// # Panics
    /// Panics if the object has already been taken.
    #[allow(clippy::expect_used)]
    pub fn take(mut self) -> T {
        self.obj.take().expect("Object already taken")
    }
}

impl<T: Poolable> std::ops::Deref for PooledObject<T> {
    type Target = T;

    #[allow(clippy::expect_used)]
    fn deref(&self) -> &Self::Target {
        self.obj.as_ref().expect("Object already taken")
    }
}

impl<T: Poolable> std::ops::DerefMut for PooledObject<T> {
    #[allow(clippy::expect_used)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.obj.as_mut().expect("Object already taken")
    }
}

impl<T: Poolable> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(mut obj) = self.obj.take() {
            obj.reset();
            // Return to pool, ignore if full
            let _ = self.pool.push(obj);
        }
    }
}

/// Pool for reusing byte buffers
pub type ByteBufferPool = ObjectPool<Vec<u8>>;

impl Poolable for Vec<u8> {
    fn reset(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct TestObject {
        value: i32,
        used: bool,
    }

    impl Poolable for TestObject {
        fn reset(&mut self) {
            self.value = 0;
            self.used = false;
        }
    }

    #[test]
    fn test_pool_acquire_release() {
        let pool: ObjectPool<TestObject> = ObjectPool::new(10);

        // Pool starts empty
        assert!(pool.is_empty());
        assert!(pool.try_acquire().is_none());

        // Release an object
        let obj = TestObject {
            value: 42,
            used: true,
        };
        pool.release(obj);

        assert_eq!(pool.len(), 1);

        // Acquire should return the reset object
        let acquired = pool.try_acquire().unwrap();
        assert_eq!(acquired.value, 0);
        assert!(!acquired.used);
        assert!(pool.is_empty());
    }

    #[test]
    fn test_pool_capacity_limit() {
        let pool: ObjectPool<TestObject> = ObjectPool::new(2);

        // Fill the pool
        pool.release(TestObject::default());
        pool.release(TestObject::default());
        assert_eq!(pool.len(), 2);

        // Third object should be dropped (pool full)
        pool.release(TestObject::default());
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn test_pooled_object_auto_return() {
        let pool: ObjectPool<TestObject> = ObjectPool::new(10);
        let queue = pool.queue();

        {
            let mut obj = TestObject {
                value: 100,
                used: true,
            };
            obj.value = 100;
            let _pooled = PooledObject::new(obj, queue.clone());
            // Object returned to pool when _pooled drops
        }

        assert_eq!(pool.len(), 1);
        let acquired = pool.try_acquire().unwrap();
        assert_eq!(acquired.value, 0); // Reset
    }

    #[test]
    fn test_byte_buffer_pool() {
        let pool: ByteBufferPool = ObjectPool::new(10);

        let mut buf = Vec::with_capacity(1024);
        buf.extend_from_slice(b"hello world");

        pool.release(buf);

        let acquired = pool.try_acquire().unwrap();
        assert!(acquired.is_empty()); // Cleared
        assert!(acquired.capacity() >= 1024); // Capacity preserved
    }
}
