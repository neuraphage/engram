//! Engram: A minimal git-backed task graph library.
//!
//! Engram provides persistent, git-backed task tracking with SQLite caching.
//! It is designed for use by AI orchestration systems like Neuraphage.
//!
//! # Example
//!
//! ```no_run
//! use engram::{Store, Status, EdgeKind};
//! use std::path::Path;
//!
//! // Initialize a new store
//! let mut store = Store::init(Path::new(".")).unwrap();
//!
//! // Create tasks
//! let task1 = store.create("Implement login", 1, &["auth"], None).unwrap();
//! let task2 = store.create("Write tests", 2, &["auth", "test"], None).unwrap();
//!
//! // Add a dependency
//! store.add_edge(&task2.id, &task1.id, EdgeKind::Blocks).unwrap();
//!
//! // Query ready work
//! let ready = store.ready().unwrap();
//! assert_eq!(ready.len(), 1);
//! assert_eq!(ready[0].id, task1.id);
//!
//! // Close a task
//! store.close(&task1.id, Some("Implemented OAuth")).unwrap();
//! ```

mod id;
mod storage;
mod store;
mod types;

// Re-export public API
pub use store::{Store, StoreError};
pub use types::{Edge, EdgeKind, Item, Status, ValidationError};
