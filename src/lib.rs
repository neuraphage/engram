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

pub mod id;
mod storage;
mod store;
mod types;

pub mod batch;
pub mod builder;
pub mod client;
pub mod compact;
pub mod daemon;
pub mod eventquery;
pub mod protocol;
pub mod query;
pub mod vacuum;

// Re-export public API
pub use batch::{BatchCloseResult, BatchCreateResult, CreateSpec, StoreBatchExt};
pub use builder::{ItemBuilder, StoreBuilderExt};
pub use client::Client;
pub use compact::{CompactConfig, CompactResult, StoreCompactExt};
pub use daemon::{Daemon, DaemonConfig, is_daemon_running, start_daemon};
pub use eventquery::{EventCounts, EventQuery, StoreEventExt, TimelineEntry};
pub use id::generate_event_id;
pub use protocol::{Request, Response};
pub use query::{Query, StoreQueryExt};
pub use store::{Store, StoreError};
pub use types::{Edge, EdgeKind, Event, EventFilter, Filter, Item, Status, ValidationError};
pub use vacuum::{VacuumResult, vacuum};
