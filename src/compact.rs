//! Compaction for reducing storage size of old closed items.
//!
//! Compaction truncates or removes descriptions from items that have been
//! closed for a specified number of days.

use crate::store::Store;
use crate::types::{Item, Status};
use chrono::{Duration, Utc};
use eyre::{Context, Result};

/// Configuration for compaction.
#[derive(Debug, Clone)]
pub struct CompactConfig {
    /// Items closed longer than this are eligible for compaction.
    pub older_than_days: u32,
    /// Maximum description length after compaction (None = remove entirely).
    pub max_description_len: Option<usize>,
}

impl Default for CompactConfig {
    fn default() -> Self {
        Self {
            older_than_days: 7,
            max_description_len: None, // Remove descriptions entirely
        }
    }
}

impl CompactConfig {
    /// Create a new config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the age threshold in days.
    pub fn older_than_days(mut self, days: u32) -> Self {
        self.older_than_days = days;
        self
    }

    /// Set max description length (None = remove entirely).
    pub fn max_description_len(mut self, len: Option<usize>) -> Self {
        self.max_description_len = len;
        self
    }
}

/// Result of a compaction operation.
#[derive(Debug)]
pub struct CompactResult {
    /// Number of items that were compacted.
    pub compacted_count: usize,
    /// Total bytes saved (approximate).
    pub bytes_saved: usize,
    /// IDs of compacted items.
    pub compacted_ids: Vec<String>,
}

/// Extension trait for compaction operations on Store.
pub trait StoreCompactExt {
    /// Run compaction with the given configuration.
    fn compact(&mut self, config: &CompactConfig) -> Result<CompactResult>;

    /// Get items eligible for compaction.
    fn get_compactable_items(&self, older_than_days: u32) -> Result<Vec<Item>>;
}

impl StoreCompactExt for Store {
    fn compact(&mut self, config: &CompactConfig) -> Result<CompactResult> {
        let cutoff = Utc::now() - Duration::days(config.older_than_days as i64);

        // Get all closed items
        let items = self.list(Some(Status::Closed)).context("Failed to list closed items")?;

        let mut compacted_count = 0;
        let mut bytes_saved = 0;
        let mut compacted_ids = Vec::new();

        for item in items {
            // Check if item is old enough and has a description
            let dominated = item.closed_at.filter(|&t| t < cutoff).is_some();
            if !dominated {
                continue;
            }

            // Only compact items with descriptions
            if let Some(ref desc) = item.description {
                let original_len = desc.len();

                // Calculate new description
                let new_desc = match config.max_description_len {
                    None => None,
                    Some(max_len) if desc.len() > max_len => Some(format!("{}...", &desc[..max_len.saturating_sub(3)])),
                    Some(_) => continue, // Already short enough
                };

                // Update the item
                self.update(&item.id, None, Some(new_desc.as_deref()), None, None)
                    .context("Failed to update item during compaction")?;

                bytes_saved += original_len - new_desc.as_ref().map(|s| s.len()).unwrap_or(0);
                compacted_count += 1;
                compacted_ids.push(item.id.clone());
            }
        }

        Ok(CompactResult {
            compacted_count,
            bytes_saved,
            compacted_ids,
        })
    }

    fn get_compactable_items(&self, older_than_days: u32) -> Result<Vec<Item>> {
        let cutoff = Utc::now() - Duration::days(older_than_days as i64);

        let items = self.list(Some(Status::Closed)).context("Failed to list closed items")?;

        Ok(items
            .into_iter()
            .filter(|item| {
                item.closed_at.map(|closed_at| closed_at < cutoff).unwrap_or(false) && item.description.is_some()
            })
            .collect())
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
    fn test_compact_config_builder() {
        let config = CompactConfig::new().older_than_days(14).max_description_len(Some(100));

        assert_eq!(config.older_than_days, 14);
        assert_eq!(config.max_description_len, Some(100));
    }

    #[test]
    fn test_compact_no_eligible_items() {
        let (_temp_dir, mut store) = setup_test_store();

        // Create and close an item (it won't be old enough)
        let item = store.create("Test task", 2, &[], Some("A description")).unwrap();
        store.close(&item.id, None).unwrap();

        let config = CompactConfig::new().older_than_days(1);
        let result = store.compact(&config).unwrap();

        assert_eq!(result.compacted_count, 0);
    }

    #[test]
    fn test_get_compactable_items_empty() {
        let (_temp_dir, store) = setup_test_store();

        let items = store.get_compactable_items(7).unwrap();
        assert!(items.is_empty());
    }
}
