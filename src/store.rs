//! High-level store API for Engram.

use crate::id::generate_id;
use crate::storage::Storage;
use crate::types::{Edge, EdgeKind, Item, Status, ValidationError};
use chrono::Utc;
use eyre::{Context, Result};
use std::collections::HashSet;
use std::path::Path;

/// Errors that can occur during store operations.
#[derive(Debug)]
pub enum StoreError {
    /// Item not found.
    ItemNotFound(String),
    /// Self-referential edge.
    SelfReferentialEdge,
    /// Adding this edge would create a cycle.
    CycleDetected,
    /// Invalid status transition.
    InvalidStatusTransition { from: Status, to: Status },
    /// Validation error.
    Validation(ValidationError),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::ItemNotFound(id) => write!(f, "item not found: {}", id),
            StoreError::SelfReferentialEdge => write!(f, "cannot create edge to self"),
            StoreError::CycleDetected => write!(f, "adding this edge would create a cycle"),
            StoreError::InvalidStatusTransition { from, to } => {
                write!(f, "invalid status transition from {:?} to {:?}", from, to)
            }
            StoreError::Validation(e) => write!(f, "validation error: {}", e),
        }
    }
}

impl std::error::Error for StoreError {}

/// The main Engram store.
pub struct Store {
    storage: Storage,
}

impl Store {
    /// Initialize a new store in the given directory.
    pub fn init(root: &Path) -> Result<Self> {
        let storage = Storage::init(root)?;
        Ok(Self { storage })
    }

    /// Open an existing store.
    pub fn open(root: &Path) -> Result<Self> {
        let storage = Storage::open(root)?;
        Ok(Self { storage })
    }

    /// Create a new item.
    pub fn create(&mut self, title: &str, priority: u8, labels: &[&str], description: Option<&str>) -> Result<Item> {
        let now = Utc::now();
        let id = generate_id(title, now);

        let item = Item {
            id,
            title: title.to_string(),
            description: description.map(String::from),
            status: Status::Open,
            priority,
            labels: labels.iter().map(|s| s.to_string()).collect(),
            created_at: now,
            updated_at: now,
            closed_at: None,
            close_reason: None,
        };

        // Validate before persisting
        item.validate().map_err(|e| eyre::eyre!(StoreError::Validation(e)))?;

        self.storage.append_item(&item).context("Failed to persist item")?;

        Ok(item)
    }

    /// Get an item by ID.
    pub fn get(&self, id: &str) -> Result<Option<Item>> {
        self.storage.get_item(id)
    }

    /// Update an item's fields.
    pub fn update(
        &mut self,
        id: &str,
        title: Option<&str>,
        description: Option<Option<&str>>,
        priority: Option<u8>,
        labels: Option<&[&str]>,
    ) -> Result<Item> {
        let existing = self
            .storage
            .get_item(id)?
            .ok_or_else(|| eyre::eyre!(StoreError::ItemNotFound(id.to_string())))?;

        let now = Utc::now();
        let updated = Item {
            id: existing.id,
            title: title.map(String::from).unwrap_or(existing.title),
            description: match description {
                Some(d) => d.map(String::from),
                None => existing.description,
            },
            status: existing.status,
            priority: priority.unwrap_or(existing.priority),
            labels: labels
                .map(|l| l.iter().map(|s| s.to_string()).collect())
                .unwrap_or(existing.labels),
            created_at: existing.created_at,
            updated_at: now,
            closed_at: existing.closed_at,
            close_reason: existing.close_reason,
        };

        // Validate before persisting
        updated.validate().map_err(|e| eyre::eyre!(StoreError::Validation(e)))?;

        self.storage
            .append_item(&updated)
            .context("Failed to persist updated item")?;

        Ok(updated)
    }

    /// Change an item's status.
    pub fn set_status(&mut self, id: &str, status: Status) -> Result<Item> {
        let existing = self
            .storage
            .get_item(id)?
            .ok_or_else(|| eyre::eyre!(StoreError::ItemNotFound(id.to_string())))?;

        if !existing.status.can_transition_to(&status) {
            return Err(eyre::eyre!(StoreError::InvalidStatusTransition {
                from: existing.status,
                to: status
            }));
        }

        let now = Utc::now();
        let updated = Item {
            status,
            updated_at: now,
            ..existing
        };

        self.storage
            .append_item(&updated)
            .context("Failed to persist status change")?;

        Ok(updated)
    }

    /// Close an item with an optional reason.
    pub fn close(&mut self, id: &str, reason: Option<&str>) -> Result<Item> {
        let existing = self
            .storage
            .get_item(id)?
            .ok_or_else(|| eyre::eyre!(StoreError::ItemNotFound(id.to_string())))?;

        if !existing.status.can_transition_to(&Status::Closed) {
            return Err(eyre::eyre!(StoreError::InvalidStatusTransition {
                from: existing.status,
                to: Status::Closed
            }));
        }

        let now = Utc::now();
        let updated = Item {
            status: Status::Closed,
            updated_at: now,
            closed_at: Some(now),
            close_reason: reason.map(String::from),
            ..existing
        };

        self.storage.append_item(&updated).context("Failed to persist close")?;

        Ok(updated)
    }

    /// List items with optional status filter.
    pub fn list(&self, status_filter: Option<Status>) -> Result<Vec<Item>> {
        self.storage.list_items(status_filter)
    }

    /// Get items that are ready to work on.
    pub fn ready(&self) -> Result<Vec<Item>> {
        self.storage.ready()
    }

    /// Add an edge between items.
    pub fn add_edge(&mut self, from_id: &str, to_id: &str, kind: EdgeKind) -> Result<Edge> {
        // No self-referential edges
        if from_id == to_id {
            return Err(eyre::eyre!(StoreError::SelfReferentialEdge));
        }

        // Both items must exist
        if self.storage.get_item(from_id)?.is_none() {
            return Err(eyre::eyre!(StoreError::ItemNotFound(from_id.to_string())));
        }
        if self.storage.get_item(to_id)?.is_none() {
            return Err(eyre::eyre!(StoreError::ItemNotFound(to_id.to_string())));
        }

        // Check for existing edge (idempotent)
        if self.storage.edge_exists(from_id, to_id, kind)? {
            let now = Utc::now();
            return Ok(Edge {
                from_id: from_id.to_string(),
                to_id: to_id.to_string(),
                kind,
                created_at: now,
                deleted: false,
            });
        }

        // For blocking edges, check for cycles
        if kind.is_blocking() && self.would_create_cycle(from_id, to_id)? {
            return Err(eyre::eyre!(StoreError::CycleDetected));
        }

        let now = Utc::now();
        let edge = Edge {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            kind,
            created_at: now,
            deleted: false,
        };

        self.storage.append_edge(&edge).context("Failed to persist edge")?;

        Ok(edge)
    }

    /// Remove an edge between items.
    pub fn remove_edge(&mut self, from_id: &str, to_id: &str, kind: EdgeKind) -> Result<()> {
        let now = Utc::now();
        let edge = Edge {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            kind,
            created_at: now,
            deleted: true,
        };

        self.storage
            .append_edge(&edge)
            .context("Failed to persist edge removal")?;

        Ok(())
    }

    /// Check if adding an edge would create a cycle in the blocking graph.
    fn would_create_cycle(&self, from_id: &str, to_id: &str) -> Result<bool> {
        // DFS from 'to_id' to see if we can reach 'from_id'
        // If yes, adding from->to would create a cycle
        let mut visited = HashSet::new();
        let mut stack = vec![to_id.to_string()];

        while let Some(node) = stack.pop() {
            if node == from_id {
                return Ok(true);
            }
            if visited.insert(node.clone()) {
                for edge in self.storage.get_blocking_edges_from(&node)? {
                    stack.push(edge.to_id);
                }
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_store() -> (TempDir, Store) {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::init(temp_dir.path()).unwrap();
        (temp_dir, store)
    }

    #[test]
    fn test_create_and_get() {
        let (_temp_dir, mut store) = setup_test_store();

        let item = store
            .create("Test task", 2, &["test", "example"], Some("A description"))
            .unwrap();

        assert!(item.id.starts_with("eg-"));
        assert_eq!(item.title, "Test task");
        assert_eq!(item.priority, 2);
        assert_eq!(item.labels, vec!["test", "example"]);
        assert_eq!(item.status, Status::Open);

        let retrieved = store.get(&item.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test task");
    }

    #[test]
    fn test_update() {
        let (_temp_dir, mut store) = setup_test_store();

        let item = store.create("Original", 2, &[], None).unwrap();
        let updated = store
            .update(
                &item.id,
                Some("Updated title"),
                Some(Some("New description")),
                Some(1),
                Some(&["new-label"]),
            )
            .unwrap();

        assert_eq!(updated.title, "Updated title");
        assert_eq!(updated.description, Some("New description".to_string()));
        assert_eq!(updated.priority, 1);
        assert_eq!(updated.labels, vec!["new-label"]);
    }

    #[test]
    fn test_close() {
        let (_temp_dir, mut store) = setup_test_store();

        let item = store.create("Task to close", 2, &[], None).unwrap();
        let closed = store.close(&item.id, Some("Completed")).unwrap();

        assert_eq!(closed.status, Status::Closed);
        assert!(closed.closed_at.is_some());
        assert_eq!(closed.close_reason, Some("Completed".to_string()));
    }

    #[test]
    fn test_invalid_status_transition() {
        let (_temp_dir, mut store) = setup_test_store();

        let item = store.create("Task", 2, &[], None).unwrap();
        let closed = store.close(&item.id, None).unwrap();

        // Closed -> InProgress is invalid
        let result = store.set_status(&closed.id, Status::InProgress);
        assert!(result.is_err());
    }

    #[test]
    fn test_ready_with_blocking() {
        let (_temp_dir, mut store) = setup_test_store();

        let blocker = store.create("Blocker task", 0, &[], None).unwrap();
        let blocked = store.create("Blocked task", 1, &[], None).unwrap();

        store.add_edge(&blocked.id, &blocker.id, EdgeKind::Blocks).unwrap();

        let ready = store.ready().unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, blocker.id);

        // Close the blocker
        store.close(&blocker.id, None).unwrap();

        // Now blocked should be ready
        let ready = store.ready().unwrap();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id, blocked.id);
    }

    #[test]
    fn test_self_referential_edge_rejected() {
        let (_temp_dir, mut store) = setup_test_store();

        let item = store.create("Task", 2, &[], None).unwrap();
        let result = store.add_edge(&item.id, &item.id, EdgeKind::Blocks);

        assert!(result.is_err());
    }

    #[test]
    fn test_cycle_detection() {
        let (_temp_dir, mut store) = setup_test_store();

        let a = store.create("Task A", 2, &[], None).unwrap();
        let b = store.create("Task B", 2, &[], None).unwrap();
        let c = store.create("Task C", 2, &[], None).unwrap();

        // A -> B -> C
        store.add_edge(&a.id, &b.id, EdgeKind::Blocks).unwrap();
        store.add_edge(&b.id, &c.id, EdgeKind::Blocks).unwrap();

        // C -> A would create a cycle
        let result = store.add_edge(&c.id, &a.id, EdgeKind::Blocks);
        assert!(result.is_err());
    }

    #[test]
    fn test_edge_idempotent() {
        let (_temp_dir, mut store) = setup_test_store();

        let a = store.create("Task A", 2, &[], None).unwrap();
        let b = store.create("Task B", 2, &[], None).unwrap();

        // Add same edge twice
        store.add_edge(&a.id, &b.id, EdgeKind::Blocks).unwrap();
        store.add_edge(&a.id, &b.id, EdgeKind::Blocks).unwrap();

        // Should not cause issues
        let ready = store.ready().unwrap();
        assert_eq!(ready.len(), 1);
    }
}
