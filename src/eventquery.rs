//! Event query API with flexible filtering and aggregations.

use crate::storage::Storage;
use crate::store::Store;
use crate::types::{Event, EventFilter};
use chrono::{DateTime, Timelike, Utc};
use eyre::Result;
use std::collections::HashMap;

/// Query builder for fluent event queries.
pub struct EventQuery<'a> {
    storage: &'a Storage,
    filter: EventFilter,
}

impl<'a> EventQuery<'a> {
    /// Create a new event query.
    pub(crate) fn new(storage: &'a Storage) -> Self {
        Self {
            storage,
            filter: EventFilter::new(),
        }
    }

    /// Filter by event kind.
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.filter = self.filter.kind(kind);
        self
    }

    /// Filter by multiple event kinds.
    pub fn kinds(mut self, kinds: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.filter = self.filter.kinds(kinds);
        self
    }

    /// Filter by source task.
    pub fn source(mut self, task_id: impl Into<String>) -> Self {
        self.filter = self.filter.source(task_id);
        self
    }

    /// Filter by target task.
    pub fn target(mut self, task_id: impl Into<String>) -> Self {
        self.filter = self.filter.target(task_id);
        self
    }

    /// Filter by timestamp (events after this time).
    pub fn since(mut self, timestamp: DateTime<Utc>) -> Self {
        self.filter = self.filter.since(timestamp);
        self
    }

    /// Limit results.
    pub fn limit(mut self, limit: usize) -> Self {
        self.filter = self.filter.limit(limit);
        self
    }

    /// Execute the query and return matching events.
    pub fn execute(self) -> Result<Vec<Event>> {
        self.storage.query_events(&self.filter)
    }
}

/// Event counts grouped by kind.
#[derive(Debug, Clone, Default)]
pub struct EventCounts {
    /// Map of event kind to count.
    pub by_kind: HashMap<String, usize>,
    /// Total event count.
    pub total: usize,
}

/// Timeline entry representing events in a time bucket.
#[derive(Debug, Clone)]
pub struct TimelineEntry {
    /// Start of the time bucket.
    pub start: DateTime<Utc>,
    /// End of the time bucket.
    pub end: DateTime<Utc>,
    /// Events in this bucket.
    pub events: Vec<Event>,
}

/// Extension trait to add event query methods to Store.
pub trait StoreEventExt {
    /// Start building an event query.
    fn event_query(&self) -> EventQuery<'_>;

    /// Get event counts grouped by kind.
    fn event_counts(&self) -> Result<EventCounts>;

    /// Get event counts for a specific task.
    fn task_event_counts(&self, task_id: &str) -> Result<EventCounts>;

    /// Get events as a timeline grouped by hour.
    fn event_timeline(&self, since: DateTime<Utc>, limit: usize) -> Result<Vec<TimelineEntry>>;

    /// Get all event kinds that have been recorded.
    fn event_kinds(&self) -> Result<Vec<String>>;
}

impl StoreEventExt for Store {
    fn event_query(&self) -> EventQuery<'_> {
        EventQuery::new(self.storage())
    }

    fn event_counts(&self) -> Result<EventCounts> {
        let events = self.storage().recent_events(10000)?;
        let mut counts = EventCounts::default();

        for event in events {
            *counts.by_kind.entry(event.kind).or_insert(0) += 1;
            counts.total += 1;
        }

        Ok(counts)
    }

    fn task_event_counts(&self, task_id: &str) -> Result<EventCounts> {
        let events = self.storage().task_events(task_id, 10000)?;
        let mut counts = EventCounts::default();

        for event in events {
            *counts.by_kind.entry(event.kind).or_insert(0) += 1;
            counts.total += 1;
        }

        Ok(counts)
    }

    fn event_timeline(&self, since: DateTime<Utc>, limit: usize) -> Result<Vec<TimelineEntry>> {
        let filter = EventFilter::new().since(since).limit(limit);
        let events = self.storage().query_events(&filter)?;

        // Group events by hour
        let mut buckets: HashMap<DateTime<Utc>, Vec<Event>> = HashMap::new();

        for event in events {
            // Truncate to hour
            let hour = event
                .timestamp
                .date_naive()
                .and_hms_opt(event.timestamp.time().hour(), 0, 0)
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
                .unwrap_or(event.timestamp);

            buckets.entry(hour).or_default().push(event);
        }

        // Convert to timeline entries sorted by time
        let mut timeline: Vec<TimelineEntry> = buckets
            .into_iter()
            .map(|(start, events)| TimelineEntry {
                start,
                end: start + chrono::Duration::hours(1),
                events,
            })
            .collect();

        timeline.sort_by(|a, b| b.start.cmp(&a.start)); // Most recent first

        Ok(timeline)
    }

    fn event_kinds(&self) -> Result<Vec<String>> {
        let counts = self.event_counts()?;
        let mut kinds: Vec<String> = counts.by_kind.keys().cloned().collect();
        kinds.sort();
        Ok(kinds)
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
    fn test_event_query_builder() {
        let (_temp_dir, mut store) = setup_test_store();

        store
            .record_event("task_started", Some("eg-abc"), None, serde_json::json!({}))
            .unwrap();
        store
            .record_event("task_completed", Some("eg-abc"), None, serde_json::json!({}))
            .unwrap();
        store
            .record_event("task_started", Some("eg-def"), None, serde_json::json!({}))
            .unwrap();

        let started = store.event_query().kind("task_started").execute().unwrap();
        assert_eq!(started.len(), 2);

        let abc_events = store.event_query().source("eg-abc").execute().unwrap();
        assert_eq!(abc_events.len(), 2);
    }

    #[test]
    fn test_event_counts() {
        let (_temp_dir, mut store) = setup_test_store();

        store
            .record_event("task_started", None, None, serde_json::json!({}))
            .unwrap();
        store
            .record_event("task_started", None, None, serde_json::json!({}))
            .unwrap();
        store
            .record_event("task_completed", None, None, serde_json::json!({}))
            .unwrap();

        let counts = store.event_counts().unwrap();
        assert_eq!(counts.total, 3);
        assert_eq!(counts.by_kind.get("task_started"), Some(&2));
        assert_eq!(counts.by_kind.get("task_completed"), Some(&1));
    }

    #[test]
    fn test_task_event_counts() {
        let (_temp_dir, mut store) = setup_test_store();

        store
            .record_event("task_started", Some("eg-abc"), None, serde_json::json!({}))
            .unwrap();
        store
            .record_event("task_completed", Some("eg-abc"), None, serde_json::json!({}))
            .unwrap();
        store
            .record_event("task_started", Some("eg-def"), None, serde_json::json!({}))
            .unwrap();

        let counts = store.task_event_counts("eg-abc").unwrap();
        assert_eq!(counts.total, 2);
    }

    #[test]
    fn test_event_kinds() {
        let (_temp_dir, mut store) = setup_test_store();

        store
            .record_event("task_started", None, None, serde_json::json!({}))
            .unwrap();
        store
            .record_event("main_updated", None, None, serde_json::json!({}))
            .unwrap();
        store
            .record_event("task_completed", None, None, serde_json::json!({}))
            .unwrap();

        let kinds = store.event_kinds().unwrap();
        assert_eq!(kinds.len(), 3);
        assert!(kinds.contains(&"main_updated".to_string()));
        assert!(kinds.contains(&"task_completed".to_string()));
        assert!(kinds.contains(&"task_started".to_string()));
    }

    #[test]
    fn test_event_timeline() {
        let (_temp_dir, mut store) = setup_test_store();

        let now = Utc::now();
        store
            .record_event("task_started", None, None, serde_json::json!({}))
            .unwrap();
        store
            .record_event("task_completed", None, None, serde_json::json!({}))
            .unwrap();

        let timeline = store.event_timeline(now - chrono::Duration::hours(1), 100).unwrap();

        // All events should be in the same hour bucket
        assert!(!timeline.is_empty());
        let total_events: usize = timeline.iter().map(|e| e.events.len()).sum();
        assert_eq!(total_events, 2);
    }
}
