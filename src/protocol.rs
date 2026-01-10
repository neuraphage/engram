//! IPC protocol types for daemon communication.

use crate::types::{Edge, EdgeKind, Item, Status};
use serde::{Deserialize, Serialize};

/// Request sent from client to daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    /// Create a new item.
    Create {
        title: String,
        priority: u8,
        labels: Vec<String>,
        description: Option<String>,
    },

    /// Update an existing item.
    Update {
        id: String,
        title: Option<String>,
        description: Option<Option<String>>,
        priority: Option<u8>,
        labels: Option<Vec<String>>,
    },

    /// Set item status.
    SetStatus { id: String, status: Status },

    /// Close an item.
    Close { id: String, reason: Option<String> },

    /// Add an edge between items.
    AddEdge {
        from_id: String,
        to_id: String,
        kind: EdgeKind,
    },

    /// Remove an edge between items.
    RemoveEdge {
        from_id: String,
        to_id: String,
        kind: EdgeKind,
    },

    /// Get an item by ID.
    Get { id: String },

    /// List items with optional status filter.
    List { status: Option<Status> },

    /// Get ready items (unblocked, open).
    Ready,

    /// Get blocked items (have open blockers).
    Blocked,

    /// Force flush pending writes to disk.
    Flush,

    /// Shutdown the daemon.
    Shutdown,

    /// Ping to check if daemon is alive.
    Ping,
}

/// Response sent from daemon to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    /// Single item response.
    Item { item: Item },

    /// Multiple items response.
    Items { items: Vec<Item> },

    /// Single edge response.
    Edge { edge: Edge },

    /// Item not found.
    NotFound { id: String },

    /// Operation succeeded.
    Ok,

    /// Pong response to ping.
    Pong,

    /// Error response.
    Error { message: String },
}

impl Response {
    /// Create an error response.
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = Request::Create {
            title: "Test".to_string(),
            priority: 2,
            labels: vec!["test".to_string()],
            description: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        let parsed: Request = serde_json::from_str(&json).unwrap();

        if let Request::Create { title, priority, .. } = parsed {
            assert_eq!(title, "Test");
            assert_eq!(priority, 2);
        } else {
            panic!("Wrong request type");
        }
    }

    #[test]
    fn test_response_serialization() {
        let resp = Response::error("test error");
        let json = serde_json::to_string(&resp).unwrap();

        assert!(json.contains("Error"));
        assert!(json.contains("test error"));
    }
}
