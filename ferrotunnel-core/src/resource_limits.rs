//! Resource limits for preventing resource exhaustion

use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Resource limit errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ResourceLimitError {
    #[error("maximum sessions reached ({max})")]
    MaxSessionsReached { max: usize },
    #[error("maximum streams per session reached ({max})")]
    MaxStreamsReached { max: usize },
    #[error("resource limit exceeded")]
    LimitExceeded,
}

/// Global server resource limits
#[derive(Debug)]
pub struct ServerResourceLimits {
    /// Semaphore for limiting concurrent sessions
    session_semaphore: Arc<Semaphore>,
    /// Maximum sessions (for error messages)
    max_sessions: usize,
    /// Maximum streams per session
    pub max_streams_per_session: usize,
    /// Maximum in-flight frames per session
    pub max_inflight_frames: usize,
}

impl ServerResourceLimits {
    /// Create new server resource limits
    #[must_use]
    pub fn new(
        max_sessions: usize,
        max_streams_per_session: usize,
        max_inflight_frames: usize,
    ) -> Self {
        Self {
            session_semaphore: Arc::new(Semaphore::new(max_sessions)),
            max_sessions,
            max_streams_per_session,
            max_inflight_frames,
        }
    }

    /// Try to acquire a session slot
    /// Returns a permit that must be held for the session's lifetime
    pub fn try_acquire_session(&self) -> Result<SessionPermit, ResourceLimitError> {
        match self.session_semaphore.clone().try_acquire_owned() {
            Ok(permit) => Ok(SessionPermit { _permit: permit }),
            Err(_) => Err(ResourceLimitError::MaxSessionsReached {
                max: self.max_sessions,
            }),
        }
    }

    /// Create stream limits for a new session
    #[must_use]
    pub fn create_stream_limits(&self) -> StreamLimits {
        StreamLimits::new(self.max_streams_per_session)
    }

    /// Get current available session slots
    #[must_use]
    pub fn available_sessions(&self) -> usize {
        self.session_semaphore.available_permits()
    }
}

impl Clone for ServerResourceLimits {
    fn clone(&self) -> Self {
        Self {
            session_semaphore: Arc::clone(&self.session_semaphore),
            max_sessions: self.max_sessions,
            max_streams_per_session: self.max_streams_per_session,
            max_inflight_frames: self.max_inflight_frames,
        }
    }
}

impl Default for ServerResourceLimits {
    fn default() -> Self {
        Self::new(1000, 100, 100)
    }
}

/// Permit for holding a session slot
#[derive(Debug)]
pub struct SessionPermit {
    _permit: OwnedSemaphorePermit,
}

/// Per-session stream limits
#[derive(Debug)]
pub struct StreamLimits {
    /// Semaphore for limiting concurrent streams
    stream_semaphore: Arc<Semaphore>,
    /// Maximum streams (for error messages)
    max_streams: usize,
}

impl StreamLimits {
    /// Create new stream limits
    #[must_use]
    pub fn new(max_streams: usize) -> Self {
        Self {
            stream_semaphore: Arc::new(Semaphore::new(max_streams)),
            max_streams,
        }
    }

    /// Try to acquire a stream slot
    pub fn try_acquire_stream(&self) -> Result<StreamPermit, ResourceLimitError> {
        match self.stream_semaphore.clone().try_acquire_owned() {
            Ok(permit) => Ok(StreamPermit { _permit: permit }),
            Err(_) => Err(ResourceLimitError::MaxStreamsReached {
                max: self.max_streams,
            }),
        }
    }

    /// Get current available stream slots
    #[must_use]
    pub fn available_streams(&self) -> usize {
        self.stream_semaphore.available_permits()
    }
}

impl Clone for StreamLimits {
    fn clone(&self) -> Self {
        Self {
            stream_semaphore: Arc::clone(&self.stream_semaphore),
            max_streams: self.max_streams,
        }
    }
}

/// Permit for holding a stream slot
#[derive(Debug)]
pub struct StreamPermit {
    _permit: OwnedSemaphorePermit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_limits() {
        let limits = ServerResourceLimits::new(2, 10, 100);

        let permit1 = limits.try_acquire_session();
        assert!(permit1.is_ok());
        assert_eq!(limits.available_sessions(), 1);

        let permit2 = limits.try_acquire_session();
        assert!(permit2.is_ok());
        assert_eq!(limits.available_sessions(), 0);

        // Should fail - no more slots
        let permit3 = limits.try_acquire_session();
        assert!(permit3.is_err());

        // Drop permit1, slot should be available again
        drop(permit1);
        assert_eq!(limits.available_sessions(), 1);
    }

    #[test]
    fn test_stream_limits() {
        let limits = StreamLimits::new(2);

        let _p1 = limits.try_acquire_stream().unwrap();
        let _p2 = limits.try_acquire_stream().unwrap();

        // Should fail
        assert!(limits.try_acquire_stream().is_err());
    }
}
