use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Represents an active client session
#[derive(Debug)]
pub struct Session {
    pub id: Uuid,
    pub client_addr: SocketAddr,
    pub token: String,
    pub connected_at: Instant,
    pub last_heartbeat: Instant,
    pub capabilities: Vec<String>,
}

impl Session {
    pub fn new(
        id: Uuid,
        client_addr: SocketAddr,
        token: String,
        capabilities: Vec<String>,
    ) -> Self {
        let now = Instant::now();
        Self {
            id,
            client_addr,
            token,
            connected_at: now,
            last_heartbeat: now,
            capabilities,
        }
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }
}

/// Thread-safe session store
#[derive(Debug, Clone, Default)]
pub struct SessionStore {
    sessions: Arc<DashMap<Uuid, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// Add a new session
    pub fn add(&self, session: Session) {
        self.sessions.insert(session.id, session);
    }

    /// Get a session by ID
    pub fn get(&self, id: &Uuid) -> Option<dashmap::mapref::one::Ref<'_, Uuid, Session>> {
        self.sessions.get(id)
    }

    /// Get a mutable session by ID (e.g. to update heartbeat)
    pub fn get_mut(&self, id: &Uuid) -> Option<dashmap::mapref::one::RefMut<'_, Uuid, Session>> {
        self.sessions.get_mut(id)
    }

    /// Remove a session
    pub fn remove(&self, id: &Uuid) -> Option<Session> {
        self.sessions.remove(id).map(|(_, s)| s)
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
            self.sessions.remove(&id);
        }

        count
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
        let session = Session::new(id, addr, "token".into(), vec![]);

        store.add(session);
        assert_eq!(store.count(), 1);

        assert!(store.get(&id).is_some());

        let removed = store.remove(&id);
        assert!(removed.is_some());
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_stale_cleanup() {
        let store = SessionStore::new();
        let id = Uuid::new_v4();
        let addr = "127.0.0.1:1234".parse().unwrap();
        let mut session = Session::new(id, addr, "token".into(), vec![]);

        // Artificially age the session
        session.last_heartbeat = Instant::now()
            .checked_sub(Duration::from_secs(100))
            .unwrap();
        store.add(session);

        let count = store.cleanup_stale_sessions(Duration::from_secs(90));
        assert_eq!(count, 1);
        assert_eq!(store.count(), 0);
    }
}
