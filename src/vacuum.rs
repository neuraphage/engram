//! Vacuum operations for database maintenance.
//!
//! Vacuum rebuilds the SQLite cache from JSONL source and runs SQLite's
//! built-in vacuum to reclaim space.

use crate::storage::Storage;
use eyre::{Context, Result};
use std::path::Path;

/// Result of a vacuum operation.
#[derive(Debug)]
pub struct VacuumResult {
    /// Size of database before vacuum (bytes).
    pub size_before: u64,
    /// Size of database after vacuum (bytes).
    pub size_after: u64,
    /// Number of items in the store.
    pub item_count: usize,
    /// Number of edges in the store.
    pub edge_count: usize,
}

/// Vacuum the engram store at the given path.
///
/// This operation:
/// 1. Rebuilds the SQLite cache from JSONL source files
/// 2. Runs SQLite VACUUM to reclaim space
/// 3. Returns statistics about the operation
pub fn vacuum(root: &Path) -> Result<VacuumResult> {
    // Get size before
    let db_path = root.join(".engram").join("cache.db");
    let size_before = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

    // Reopen storage (this triggers rebuild)
    let storage = Storage::open(root).context("Failed to open storage for vacuum")?;

    // Run SQLite vacuum
    storage.vacuum().context("Failed to run SQLite vacuum")?;

    // Get counts
    let item_count = storage.count_all_items()?;
    let edge_count = storage.count_all_edges()?;

    // Get size after
    let size_after = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

    Ok(VacuumResult {
        size_before,
        size_after,
        item_count,
        edge_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
    use tempfile::TempDir;

    #[test]
    fn test_vacuum_empty_store() {
        let temp_dir = TempDir::new().unwrap();
        Store::init(temp_dir.path()).unwrap();

        let result = vacuum(temp_dir.path()).unwrap();

        assert_eq!(result.item_count, 0);
        assert_eq!(result.edge_count, 0);
    }

    #[test]
    fn test_vacuum_with_items() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = Store::init(temp_dir.path()).unwrap();

        store.create("Task 1", 2, &[], None).unwrap();
        store.create("Task 2", 2, &[], None).unwrap();

        drop(store); // Close store

        let result = vacuum(temp_dir.path()).unwrap();

        assert_eq!(result.item_count, 2);
        assert_eq!(result.edge_count, 0);
    }
}
