//! Server-Sent Events (SSE) support for real-time dashboard updates.
//!
//! This module provides event broadcasting infrastructure for pushing
//! real-time updates to connected dashboard clients.
//!
//! # Example
//!
//! ```ignore
//! use ferrotunnel_observability::dashboard::events::{EventBroadcaster, DashboardEvent};
//! use std::sync::Arc;
//!
//! let broadcaster = Arc::new(EventBroadcaster::new(100));
//!
//! // Send events from anywhere
//! broadcaster.send(DashboardEvent::TunnelDisconnected {
//!     tunnel_id: some_uuid,
//! });
//!
//! // Mount the SSE handler on your router
//! let app = Router::new()
//!     .route("/api/v1/events", get(events_handler))
//!     .with_state(broadcaster);
//! ```

use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use uuid::Uuid;

use super::models::{DashboardTunnelInfo, RequestLogEntry};

/// Events that can be broadcast to dashboard clients via SSE.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DashboardEvent {
    /// A new HTTP request was logged.
    NewRequest(RequestLogEntry),
    /// A tunnel has connected.
    TunnelConnected(DashboardTunnelInfo),
    /// A tunnel has disconnected.
    TunnelDisconnected { tunnel_id: Uuid },
}

impl DashboardEvent {
    /// Returns the tunnel ID associated with this event, if any.
    pub fn tunnel_id(&self) -> Option<Uuid> {
        match self {
            DashboardEvent::NewRequest(entry) => Some(entry.tunnel_id),
            DashboardEvent::TunnelConnected(info) => Some(info.id),
            DashboardEvent::TunnelDisconnected { tunnel_id } => Some(*tunnel_id),
        }
    }

    /// Returns the SSE event type name for this event.
    fn event_type(&self) -> &'static str {
        match self {
            DashboardEvent::NewRequest(_) => "new_request",
            DashboardEvent::TunnelConnected(_) => "tunnel_connected",
            DashboardEvent::TunnelDisconnected { .. } => "tunnel_disconnected",
        }
    }
}

/// Broadcasts dashboard events to connected SSE clients.
///
/// Uses a tokio broadcast channel internally to support multiple subscribers.
/// Events are dropped if no clients are connected.
pub struct EventBroadcaster {
    sender: broadcast::Sender<DashboardEvent>,
}

impl EventBroadcaster {
    /// Creates a new event broadcaster with the specified channel capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of events to buffer. Defaults to 100 if 0 is passed.
    pub fn new(capacity: usize) -> Self {
        let capacity = if capacity == 0 { 100 } else { capacity };
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Creates a new subscriber to receive broadcast events.
    pub fn subscribe(&self) -> broadcast::Receiver<DashboardEvent> {
        self.sender.subscribe()
    }

    /// Sends an event to all connected subscribers.
    ///
    /// If there are no subscribers, the event is silently dropped.
    pub fn send(&self, event: DashboardEvent) {
        let _ = self.sender.send(event);
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Query parameters for the events endpoint.
#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    /// Optional tunnel ID to filter events.
    pub tunnel_id: Option<Uuid>,
}

/// Axum handler for SSE events endpoint.
///
/// Streams real-time dashboard events to connected clients.
/// Events can be filtered by tunnel_id using query parameter.
///
/// # Endpoint
///
/// `GET /api/v1/events?tunnel_id=<uuid>`
///
/// # SSE Format
///
/// ```text
/// event: new_request
/// data: {"type":"new_request","id":"...","tunnel_id":"..."}
///
/// event: tunnel_connected
/// data: {"type":"tunnel_connected","id":"...","subdomain":"..."}
/// ```
pub async fn events_handler(
    State(broadcaster): State<Arc<EventBroadcaster>>,
    Query(query): Query<EventsQuery>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let receiver = broadcaster.subscribe();
    let stream = BroadcastStream::new(receiver);

    let filter_tunnel_id = query.tunnel_id;

    let event_stream = stream.filter_map(move |result: Result<DashboardEvent, _>| match result {
        Ok(event) => {
            if let Some(filter_id) = filter_tunnel_id {
                if event.tunnel_id() != Some(filter_id) {
                    return None;
                }
            }

            let event_type = event.event_type();
            match serde_json::to_string(&event) {
                Ok(data) => Some(Ok(Event::default().event(event_type).data(data))),
                Err(_) => None,
            }
        }
        Err(_) => None,
    });

    Sse::new(event_stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcaster_new() {
        let broadcaster = EventBroadcaster::new(50);
        let _receiver = broadcaster.subscribe();
    }

    #[test]
    fn test_broadcaster_default() {
        let broadcaster = EventBroadcaster::default();
        let _receiver = broadcaster.subscribe();
    }

    #[test]
    fn test_event_type() {
        let event = DashboardEvent::TunnelDisconnected {
            tunnel_id: Uuid::new_v4(),
        };
        assert_eq!(event.event_type(), "tunnel_disconnected");
    }

    #[test]
    fn test_event_tunnel_id() {
        let tunnel_id = Uuid::new_v4();
        let event = DashboardEvent::TunnelDisconnected { tunnel_id };
        assert_eq!(event.tunnel_id(), Some(tunnel_id));
    }

    #[tokio::test]
    async fn test_broadcast_send_receive() {
        let broadcaster = EventBroadcaster::new(10);
        let mut receiver = broadcaster.subscribe();

        let tunnel_id = Uuid::new_v4();
        broadcaster.send(DashboardEvent::TunnelDisconnected { tunnel_id });

        let received = receiver.recv().await.unwrap();
        assert_eq!(received.tunnel_id(), Some(tunnel_id));
    }
}
