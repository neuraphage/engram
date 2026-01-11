//! Shared test infrastructure for Engram integration tests.
//!
//! Provides TestEnv helper for consistent test setup/teardown.

#![allow(dead_code)]

use engram::{Edge, EdgeKind, Item, Status, Store};
use tempfile::TempDir;

/// Test environment with automatic cleanup.
pub struct TestEnv {
    pub temp_dir: TempDir,
    pub store: Store,
}

impl TestEnv {
    /// Create a new test environment with an initialized store.
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let store = Store::init(temp_dir.path()).expect("Failed to init store");
        Self { temp_dir, store }
    }

    /// Create an item with default priority and no labels.
    pub fn create_item(&mut self, title: &str) -> Item {
        self.store.create(title, 2, &[], None).expect("Failed to create item")
    }

    /// Create an item with specified priority.
    pub fn create_item_with_priority(&mut self, title: &str, priority: u8) -> Item {
        self.store
            .create(title, priority, &[], None)
            .expect("Failed to create item")
    }

    /// Create an item with labels.
    pub fn create_item_with_labels(&mut self, title: &str, labels: &[&str]) -> Item {
        self.store
            .create(title, 2, labels, None)
            .expect("Failed to create item")
    }

    /// Create an item with description.
    pub fn create_item_with_desc(&mut self, title: &str, description: &str) -> Item {
        self.store
            .create(title, 2, &[], Some(description))
            .expect("Failed to create item")
    }

    /// Add a blocking dependency (from blocks on to).
    pub fn add_blocking_dep(&mut self, from: &Item, to: &Item) -> Edge {
        self.store
            .add_edge(&from.id, &to.id, EdgeKind::Blocks)
            .expect("Failed to add edge")
    }

    /// Close an item.
    pub fn close_item(&mut self, item: &Item) -> Item {
        self.store.close(&item.id, None).expect("Failed to close item")
    }

    /// Close an item with a reason.
    pub fn close_item_with_reason(&mut self, item: &Item, reason: &str) -> Item {
        self.store.close(&item.id, Some(reason)).expect("Failed to close item")
    }

    /// Assert that an item is in the ready list.
    pub fn assert_ready(&self, item: &Item) {
        let ready = self.store.ready().expect("Failed to get ready items");
        assert!(
            ready.iter().any(|i| i.id == item.id),
            "Expected item {} to be ready, but it wasn't. Ready items: {:?}",
            item.id,
            ready.iter().map(|i| &i.id).collect::<Vec<_>>()
        );
    }

    /// Assert that an item is NOT in the ready list.
    pub fn assert_not_ready(&self, item: &Item) {
        let ready = self.store.ready().expect("Failed to get ready items");
        assert!(
            !ready.iter().any(|i| i.id == item.id),
            "Expected item {} to NOT be ready, but it was",
            item.id
        );
    }

    /// Assert that an item is in the blocked list.
    pub fn assert_blocked(&self, item: &Item) {
        let blocked = self.store.blocked().expect("Failed to get blocked items");
        assert!(
            blocked.iter().any(|i| i.id == item.id),
            "Expected item {} to be blocked, but it wasn't",
            item.id
        );
    }

    /// Get ready items count.
    pub fn ready_count(&self) -> usize {
        self.store.ready().expect("Failed to get ready items").len()
    }

    /// Get all items count.
    pub fn total_count(&self) -> usize {
        self.store.list(None).expect("Failed to list items").len()
    }

    /// Get items by status.
    pub fn count_by_status(&self, status: Status) -> usize {
        self.store.list(Some(status)).expect("Failed to list items").len()
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}
