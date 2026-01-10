//! Query API with flexible filtering.

use crate::storage::Storage;
use crate::store::Store;
use crate::types::{Filter, Item, Status};
use eyre::Result;

/// Query builder for fluent queries.
pub struct Query<'a> {
    storage: &'a Storage,
    filter: Filter,
}

impl<'a> Query<'a> {
    /// Create a new query.
    pub(crate) fn new(storage: &'a Storage) -> Self {
        Self {
            storage,
            filter: Filter::new(),
        }
    }

    /// Filter by status.
    pub fn status(mut self, status: Status) -> Self {
        self.filter = self.filter.status(status);
        self
    }

    /// Filter by label.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.filter = self.filter.label(label);
        self
    }

    /// Filter by multiple labels.
    pub fn labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter = self.filter.labels(labels);
        self
    }

    /// Filter by minimum priority.
    pub fn min_priority(mut self, priority: u8) -> Self {
        self.filter = self.filter.min_priority(priority);
        self
    }

    /// Filter by maximum priority.
    pub fn max_priority(mut self, priority: u8) -> Self {
        self.filter = self.filter.max_priority(priority);
        self
    }

    /// Filter by title substring.
    pub fn title_contains(mut self, substring: impl Into<String>) -> Self {
        self.filter = self.filter.title_contains(substring);
        self
    }

    /// Limit results.
    pub fn limit(mut self, limit: usize) -> Self {
        self.filter = self.filter.limit(limit);
        self
    }

    /// Skip first N results.
    pub fn offset(mut self, offset: usize) -> Self {
        self.filter = self.filter.offset(offset);
        self
    }

    /// Execute the query and return matching items.
    pub fn execute(self) -> Result<Vec<Item>> {
        self.storage.query_items(&self.filter)
    }

    /// Count matching items without fetching them.
    pub fn count(self) -> Result<usize> {
        self.storage.count_items(&self.filter)
    }
}

/// Extension trait to add query method to Store.
pub trait StoreQueryExt {
    /// Start building a query.
    fn query(&self) -> Query<'_>;

    /// Query with a pre-built filter.
    fn query_with_filter(&self, filter: &Filter) -> Result<Vec<Item>>;
}

impl StoreQueryExt for Store {
    fn query(&self) -> Query<'_> {
        Query::new(self.storage())
    }

    fn query_with_filter(&self, filter: &Filter) -> Result<Vec<Item>> {
        self.storage().query_items(filter)
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
    fn test_filter_builder() {
        let filter = Filter::new()
            .status(Status::Open)
            .label("backend")
            .min_priority(0)
            .max_priority(2)
            .limit(10);

        assert_eq!(filter.status, Some(Status::Open));
        assert_eq!(filter.labels, Some(vec!["backend".to_string()]));
        assert_eq!(filter.min_priority, Some(0));
        assert_eq!(filter.max_priority, Some(2));
        assert_eq!(filter.limit, Some(10));
    }

    #[test]
    fn test_query_by_status() {
        let (_temp_dir, mut store) = setup_test_store();

        store.create("Open task", 2, &[], None).unwrap();
        let item = store.create("Closed task", 2, &[], None).unwrap();
        store.close(&item.id, None).unwrap();

        let open_items = store.query().status(Status::Open).execute().unwrap();
        assert_eq!(open_items.len(), 1);
        assert_eq!(open_items[0].title, "Open task");

        let closed_items = store.query().status(Status::Closed).execute().unwrap();
        assert_eq!(closed_items.len(), 1);
        assert_eq!(closed_items[0].title, "Closed task");
    }

    #[test]
    fn test_query_by_label() {
        let (_temp_dir, mut store) = setup_test_store();

        store.create("Backend task", 2, &["backend"], None).unwrap();
        store.create("Frontend task", 2, &["frontend"], None).unwrap();
        store.create("Full stack", 2, &["backend", "frontend"], None).unwrap();

        let backend = store.query().label("backend").execute().unwrap();
        assert_eq!(backend.len(), 2);
    }

    #[test]
    fn test_query_by_priority() {
        let (_temp_dir, mut store) = setup_test_store();

        store.create("Critical", 0, &[], None).unwrap();
        store.create("High", 1, &[], None).unwrap();
        store.create("Normal", 2, &[], None).unwrap();
        store.create("Low", 4, &[], None).unwrap();

        let high_priority = store.query().max_priority(1).execute().unwrap();
        assert_eq!(high_priority.len(), 2);

        let low_priority = store.query().min_priority(2).execute().unwrap();
        assert_eq!(low_priority.len(), 2);
    }

    #[test]
    fn test_query_limit_offset() {
        let (_temp_dir, mut store) = setup_test_store();

        for i in 0..5 {
            store.create(&format!("Task {}", i), 2, &[], None).unwrap();
        }

        let limited = store.query().limit(2).execute().unwrap();
        assert_eq!(limited.len(), 2);

        let offset = store.query().offset(3).execute().unwrap();
        assert_eq!(offset.len(), 2);
    }
}
