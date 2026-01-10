//! Batch operations for efficient bulk updates.

use crate::store::Store;
use crate::types::{Item, Status};
use eyre::{Context, Result};

/// Specification for creating an item in a batch.
#[derive(Debug, Clone)]
pub struct CreateSpec {
    pub title: String,
    pub priority: u8,
    pub labels: Vec<String>,
    pub description: Option<String>,
}

impl CreateSpec {
    /// Create a new spec with just a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            priority: 2,
            labels: Vec::new(),
            description: None,
        }
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set the labels.
    pub fn with_labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.labels = labels.into_iter().map(|l| l.into()).collect();
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Result of a batch create operation.
#[derive(Debug)]
pub struct BatchCreateResult {
    /// Successfully created items.
    pub created: Vec<Item>,
    /// Errors that occurred (index, error message).
    pub errors: Vec<(usize, String)>,
}

/// Result of a batch close operation.
#[derive(Debug)]
pub struct BatchCloseResult {
    /// Successfully closed items.
    pub closed: Vec<Item>,
    /// IDs that were not found.
    pub not_found: Vec<String>,
    /// Errors that occurred (id, error message).
    pub errors: Vec<(String, String)>,
}

/// Extension trait for batch operations on Store.
pub trait StoreBatchExt {
    /// Create multiple items in a batch.
    fn batch_create(&mut self, specs: Vec<CreateSpec>) -> Result<BatchCreateResult>;

    /// Close multiple items in a batch.
    fn batch_close(&mut self, ids: &[&str], reason: Option<&str>) -> Result<BatchCloseResult>;

    /// Set status on multiple items.
    fn batch_set_status(&mut self, ids: &[&str], status: Status) -> Result<Vec<Item>>;
}

impl StoreBatchExt for Store {
    fn batch_create(&mut self, specs: Vec<CreateSpec>) -> Result<BatchCreateResult> {
        let mut created = Vec::new();
        let mut errors = Vec::new();

        for (i, spec) in specs.into_iter().enumerate() {
            let label_refs: Vec<&str> = spec.labels.iter().map(|s| s.as_str()).collect();
            match self.create(&spec.title, spec.priority, &label_refs, spec.description.as_deref()) {
                Ok(item) => created.push(item),
                Err(e) => errors.push((i, e.to_string())),
            }
        }

        Ok(BatchCreateResult { created, errors })
    }

    fn batch_close(&mut self, ids: &[&str], reason: Option<&str>) -> Result<BatchCloseResult> {
        let mut closed = Vec::new();
        let mut not_found = Vec::new();
        let mut errors = Vec::new();

        for id in ids {
            match self.get(id).context("Failed to check item existence")? {
                None => not_found.push(id.to_string()),
                Some(_) => match self.close(id, reason) {
                    Ok(item) => closed.push(item),
                    Err(e) => errors.push((id.to_string(), e.to_string())),
                },
            }
        }

        Ok(BatchCloseResult {
            closed,
            not_found,
            errors,
        })
    }

    fn batch_set_status(&mut self, ids: &[&str], status: Status) -> Result<Vec<Item>> {
        let mut updated = Vec::new();

        for id in ids {
            let item = self.set_status(id, status)?;
            updated.push(item);
        }

        Ok(updated)
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
    fn test_batch_create() {
        let (_temp_dir, mut store) = setup_test_store();

        let specs = vec![
            CreateSpec::new("Task 1").with_priority(1),
            CreateSpec::new("Task 2").with_labels(["test"]),
            CreateSpec::new("Task 3").with_description("A description"),
        ];

        let result = store.batch_create(specs).unwrap();

        assert_eq!(result.created.len(), 3);
        assert!(result.errors.is_empty());
        assert_eq!(result.created[0].title, "Task 1");
        assert_eq!(result.created[0].priority, 1);
        assert_eq!(result.created[1].labels, vec!["test"]);
        assert_eq!(result.created[2].description, Some("A description".to_string()));
    }

    #[test]
    fn test_batch_close() {
        let (_temp_dir, mut store) = setup_test_store();

        // Create some items
        let item1 = store.create("Task 1", 2, &[], None).unwrap();
        let item2 = store.create("Task 2", 2, &[], None).unwrap();

        let result = store
            .batch_close(&[&item1.id, &item2.id, "nonexistent"], Some("Done"))
            .unwrap();

        assert_eq!(result.closed.len(), 2);
        assert_eq!(result.not_found, vec!["nonexistent"]);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_batch_set_status() {
        let (_temp_dir, mut store) = setup_test_store();

        let item1 = store.create("Task 1", 2, &[], None).unwrap();
        let item2 = store.create("Task 2", 2, &[], None).unwrap();

        let updated = store
            .batch_set_status(&[&item1.id, &item2.id], Status::InProgress)
            .unwrap();

        assert_eq!(updated.len(), 2);
        assert_eq!(updated[0].status, Status::InProgress);
        assert_eq!(updated[1].status, Status::InProgress);
    }
}
