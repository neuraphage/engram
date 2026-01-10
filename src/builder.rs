//! Builder pattern API for creating items.

use crate::store::Store;
use crate::types::Item;
use eyre::{Context, Result};

/// Builder for creating items with a fluent API.
///
/// # Example
///
/// ```ignore
/// let item = store.build("Implement feature")
///     .priority(1)
///     .label("backend")
///     .label("urgent")
///     .description("Add the new authentication flow")
///     .create()?;
/// ```
pub struct ItemBuilder<'a> {
    store: &'a mut Store,
    title: String,
    priority: u8,
    labels: Vec<String>,
    description: Option<String>,
}

impl<'a> ItemBuilder<'a> {
    /// Create a new builder with the given title.
    pub fn new(store: &'a mut Store, title: impl Into<String>) -> Self {
        Self {
            store,
            title: title.into(),
            priority: 2, // Default priority
            labels: Vec::new(),
            description: None,
        }
    }

    /// Set the priority (0=critical, 4=low).
    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Add a label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Add multiple labels.
    pub fn labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.labels.extend(labels.into_iter().map(|l| l.into()));
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Create the item.
    pub fn create(self) -> Result<Item> {
        let label_refs: Vec<&str> = self.labels.iter().map(|s| s.as_str()).collect();
        self.store
            .create(&self.title, self.priority, &label_refs, self.description.as_deref())
            .context("Failed to create item")
    }
}

/// Extension trait to add builder method to Store.
pub trait StoreBuilderExt {
    /// Start building a new item with the given title.
    fn build(&mut self, title: impl Into<String>) -> ItemBuilder<'_>;
}

impl StoreBuilderExt for Store {
    fn build(&mut self, title: impl Into<String>) -> ItemBuilder<'_> {
        ItemBuilder::new(self, title)
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
    fn test_builder_basic() {
        let (_temp_dir, mut store) = setup_test_store();

        let item = store.build("Test task").create().unwrap();

        assert_eq!(item.title, "Test task");
        assert_eq!(item.priority, 2); // Default
        assert!(item.labels.is_empty());
        assert!(item.description.is_none());
    }

    #[test]
    fn test_builder_with_all_fields() {
        let (_temp_dir, mut store) = setup_test_store();

        let item = store
            .build("Complex task")
            .priority(1)
            .label("backend")
            .label("urgent")
            .description("A detailed description")
            .create()
            .unwrap();

        assert_eq!(item.title, "Complex task");
        assert_eq!(item.priority, 1);
        assert_eq!(item.labels, vec!["backend", "urgent"]);
        assert_eq!(item.description, Some("A detailed description".to_string()));
    }

    #[test]
    fn test_builder_with_labels_iter() {
        let (_temp_dir, mut store) = setup_test_store();

        let item = store
            .build("Task with labels")
            .labels(["a", "b", "c"])
            .create()
            .unwrap();

        assert_eq!(item.labels, vec!["a", "b", "c"]);
    }
}
