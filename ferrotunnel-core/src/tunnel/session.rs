use crate::rate_limit::SessionRateLimiter;
use crate::stream::multiplexer::Multiplexer;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Represents an active client session
#[derive(Debug)]
pub struct Session {
    pub id: Uuid,
    pub tunnel_id: String,
    pub client_addr: SocketAddr,
    pub token: String,
    pub connected_at: Instant,
    pub last_heartbeat: Instant,
    pub capabilities: Vec<String>,
    pub multiplexer: Option<Multiplexer>,
    pub rate_limiter: Option<SessionRateLimiter>,
}

impl Session {
    pub fn new(
        id: Uuid,
        tunnel_id: String,
        client_addr: SocketAddr,
        token: String,
        capabilities: Vec<String>,
        multiplexer: Option<Multiplexer>,
    ) -> Self {
        let now = Instant::now();
        Self {
            id,
            tunnel_id,
            client_addr,
            token,
            connected_at: now,
            last_heartbeat: now,
            capabilities,
            multiplexer,
            rate_limiter: None,
        }
    }

    #[must_use]
    pub fn with_rate_limiter(mut self, rate_limiter: SessionRateLimiter) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }
}

/// Error type for session store operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum SessionStoreError {
    #[error("tunnel ID '{0}' is already registered by another session")]
    TunnelIdAlreadyExists(String),
}

impl From<SessionStoreError> for ferrotunnel_common::TunnelError {
    fn from(err: SessionStoreError) -> Self {
        ferrotunnel_common::TunnelError::InvalidState(err.to_string())
    }
}

/// Thread-safe session store
#[derive(Debug, Clone, Default)]
pub struct SessionStore {
    sessions: Arc<DashMap<Uuid, Session>>,
    tunnel_index: Arc<DashMap<String, Uuid>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            tunnel_index: Arc::new(DashMap::new()),
        }
    }

    /// Add a new session.
    /// Returns error if `tunnel_id` is already registered by a different session.
    pub fn add(&self, session: Session) -> Result<(), SessionStoreError> {
        let tunnel_id = session.tunnel_id.clone();
        let session_id = session.id;

        // Check if tunnel_id already exists and belongs to a different session
        if let Some(existing_id) = self.tunnel_index.get(&tunnel_id) {
            if *existing_id != session_id {
                return Err(SessionStoreError::TunnelIdAlreadyExists(tunnel_id));
            }
        }

        self.tunnel_index.insert(tunnel_id, session_id);
        self.sessions.insert(session_id, session);
        Ok(())
    }

    /// Add or replace a session, removing any existing session with the same `tunnel_id`.
    /// Use this for explicit session replacement (e.g., reconnection).
    pub fn add_or_replace(&self, session: Session) {
        let tunnel_id = session.tunnel_id.clone();
        let session_id = session.id;

        // Remove existing session with the same tunnel_id if it exists
        if let Some((_, old_session_id)) = self.tunnel_index.remove(&tunnel_id) {
            if old_session_id != session_id {
                self.sessions.remove(&old_session_id);
            }
        }

        self.tunnel_index.insert(tunnel_id, session_id);
        self.sessions.insert(session_id, session);
    }

    /// Get a session by ID
    pub fn get(&self, id: &Uuid) -> Option<dashmap::mapref::one::Ref<'_, Uuid, Session>> {
        self.sessions.get(id)
    }

    /// Get a session by `tunnel_id`
    pub fn get_by_tunnel_id(
        &self,
        tunnel_id: &str,
    ) -> Option<dashmap::mapref::one::Ref<'_, Uuid, Session>> {
        let id = self.tunnel_index.get(tunnel_id)?;
        self.sessions.get(&id)
    }

    /// Get a mutable session by ID (e.g. to update heartbeat)
    pub fn get_mut(&self, id: &Uuid) -> Option<dashmap::mapref::one::RefMut<'_, Uuid, Session>> {
        self.sessions.get_mut(id)
    }

    /// Remove a session
    pub fn remove(&self, id: &Uuid) -> Option<Session> {
        if let Some((_, session)) = self.sessions.remove(id) {
            self.tunnel_index.remove(&session.tunnel_id);
            Some(session)
        } else {
            None
        }
    }

    /// Count active sessions
    pub fn count(&self) -> usize {
        self.sessions.len()
    }

    /// Clean up stale sessions that haven't sent a heartbeat within the timeout
    /// Returns the number of removed sessions
    pub fn cleanup_stale_sessions(&self, timeout: Duration) -> usize {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        // Identify stale sessions
        // We use an explicit scope or just collect keys to avoid deadlocks strictly speaking,
        // though dashmap handles iteration safely.
        for r in self.sessions.iter() {
            if now.duration_since(r.last_heartbeat) > timeout {
                to_remove.push(*r.key());
            }
        }

        let count = to_remove.len();
        for id in to_remove {
            self.remove(&id);
        }

        count
    }

    pub fn find_multiplexer(&self) -> Option<Multiplexer> {
        for r in self.sessions.iter() {
            if let Some(m) = &r.multiplexer {
                return Some(m.clone());
            }
        }
        None
    }

    /// Find a multiplexer for a session that has the specified capability
    pub fn find_multiplexer_with_capability(&self, capability: &str) -> Option<Multiplexer> {
        for r in self.sessions.iter() {
            if r.capabilities.contains(&capability.to_string()) {
                if let Some(m) = &r.multiplexer {
                    return Some(m.clone());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_session_management() {
        let store = SessionStore::new();
        let id = Uuid::new_v4();
        let addr = "127.0.0.1:1234".parse().unwrap();
        let session = Session::new(id, "test-tunnel".into(), addr, "token".into(), vec![], None);

        store.add(session).unwrap();
        assert_eq!(store.count(), 1);

        assert!(store.get(&id).is_some());
        assert!(store.get_by_tunnel_id("test-tunnel").is_some());

        let removed = store.remove(&id);
        assert!(removed.is_some());
        assert_eq!(store.count(), 0);
        assert!(store.get_by_tunnel_id("test-tunnel").is_none());
    }

    #[test]
    fn test_stale_cleanup() {
        let store = SessionStore::new();
        let id = Uuid::new_v4();
        let addr = "127.0.0.1:1234".parse().unwrap();
        let mut session =
            Session::new(id, "test-tunnel".into(), addr, "token".into(), vec![], None);

        // Artificially age the session
        session.last_heartbeat = Instant::now()
            .checked_sub(Duration::from_secs(100))
            .unwrap();
        store.add(session).unwrap();

        let count = store.cleanup_stale_sessions(Duration::from_secs(90));
        assert_eq!(count, 1);
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_tunnel_id_uniqueness() {
        let store = SessionStore::new();
        let addr = "127.0.0.1:1234".parse().unwrap();

        // Add first session
        let id1 = Uuid::new_v4();
        let session1 = Session::new(id1, "my-tunnel".into(), addr, "token1".into(), vec![], None);
        assert!(store.add(session1).is_ok());

        // Try to add another session with the same tunnel_id - should fail
        let id2 = Uuid::new_v4();
        let session2 = Session::new(id2, "my-tunnel".into(), addr, "token2".into(), vec![], None);
        let result = store.add(session2);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SessionStoreError::TunnelIdAlreadyExists(_)
        ));

        // Only one session should exist
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn test_add_or_replace() {
        let store = SessionStore::new();
        let addr = "127.0.0.1:1234".parse().unwrap();

        // Add first session
        let id1 = Uuid::new_v4();
        let session1 = Session::new(id1, "my-tunnel".into(), addr, "token1".into(), vec![], None);
        store.add_or_replace(session1);
        assert_eq!(store.count(), 1);

        // Replace with new session with same tunnel_id
        let id2 = Uuid::new_v4();
        let session2 = Session::new(id2, "my-tunnel".into(), addr, "token2".into(), vec![], None);
        store.add_or_replace(session2);

        // Should still have only one session
        assert_eq!(store.count(), 1);

        // The old session should be gone, new one should exist
        assert!(store.get(&id1).is_none());
        assert!(store.get(&id2).is_some());
        assert!(store.get_by_tunnel_id("my-tunnel").is_some());
    }
}
