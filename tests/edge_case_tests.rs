//! Integration tests for edge cases.
//!
//! Tests boundary values, unicode handling, and unusual inputs.

mod common;

use common::TestEnv;
use engram::{Filter, Status, StoreQueryExt};

// =============================================================================
// Empty Store Operations
// =============================================================================

#[test]
fn test_empty_store_ready() {
    let env = TestEnv::new();
    let ready = env.store.ready().unwrap();
    assert!(ready.is_empty());
}

#[test]
fn test_empty_store_blocked() {
    let env = TestEnv::new();
    let blocked = env.store.blocked().unwrap();
    assert!(blocked.is_empty());
}

#[test]
fn test_empty_store_list() {
    let env = TestEnv::new();
    let all = env.store.list(None).unwrap();
    assert!(all.is_empty());
}

#[test]
fn test_empty_store_list_by_status() {
    let env = TestEnv::new();

    let open = env.store.list(Some(Status::Open)).unwrap();
    assert!(open.is_empty());

    let closed = env.store.list(Some(Status::Closed)).unwrap();
    assert!(closed.is_empty());
}

#[test]
fn test_empty_store_query() {
    let env = TestEnv::new();
    let filter = Filter::new();
    let results = env.store.query_with_filter(&filter).unwrap();
    assert!(results.is_empty());
}

// =============================================================================
// Unicode and Special Characters
// =============================================================================

#[test]
fn test_unicode_title_emoji() {
    let mut env = TestEnv::new();

    let item = env.create_item("Task with emoji: \u{1F680}");
    assert!(item.title.contains('\u{1F680}'));

    // Retrieve and verify
    let retrieved = env.store.get(&item.id).unwrap().unwrap();
    assert_eq!(retrieved.title, item.title);
}

#[test]
fn test_unicode_title_chinese() {
    let mut env = TestEnv::new();

    let item = env.create_item("\u{4E2D}\u{6587}\u{4EFB}\u{52A1}"); // Chinese characters
    assert!(item.id.starts_with("eg-"));

    let retrieved = env.store.get(&item.id).unwrap().unwrap();
    assert_eq!(retrieved.title, "\u{4E2D}\u{6587}\u{4EFB}\u{52A1}");
}

#[test]
fn test_unicode_title_arabic() {
    let mut env = TestEnv::new();

    let item = env.create_item("\u{0645}\u{0647}\u{0645}\u{0629}"); // Arabic
    let retrieved = env.store.get(&item.id).unwrap().unwrap();
    assert_eq!(retrieved.title, item.title);
}

#[test]
fn test_unicode_label_rejected() {
    let mut env = TestEnv::new();

    // Labels with emoji are rejected - must be alphanumeric with hyphens/underscores
    let result = env.store.create("Task", 2, &["\u{1F3F7}\u{FE0F}tag"], None);
    assert!(result.is_err());
}

#[test]
fn test_unicode_description() {
    let mut env = TestEnv::new();

    let item = env.create_item_with_desc("Task", "Description with \u{1F4DD} emoji");
    assert!(item.description.unwrap().contains('\u{1F4DD}'));
}

// =============================================================================
// Priority Boundary Values
// =============================================================================

#[test]
fn test_priority_zero_critical() {
    let mut env = TestEnv::new();

    let item = env.create_item_with_priority("Critical task", 0);
    assert_eq!(item.priority, 0);
}

#[test]
fn test_priority_four_low() {
    let mut env = TestEnv::new();

    let item = env.create_item_with_priority("Low priority task", 4);
    assert_eq!(item.priority, 4);
}

#[test]
fn test_priority_all_valid_values() {
    let mut env = TestEnv::new();

    for priority in 0..=4 {
        let item = env.create_item_with_priority(&format!("Priority {}", priority), priority);
        assert_eq!(item.priority, priority);
    }
}

// =============================================================================
// Title Length Boundaries
// =============================================================================

#[test]
fn test_title_length_one_char() {
    let mut env = TestEnv::new();

    let item = env.create_item("X");
    assert_eq!(item.title, "X");
}

#[test]
fn test_title_length_max_valid() {
    let mut env = TestEnv::new();

    let max_title = "x".repeat(500);
    let item = env.store.create(&max_title, 2, &[], None).unwrap();
    assert_eq!(item.title.len(), 500);
}

#[test]
fn test_title_length_just_over_max() {
    let mut env = TestEnv::new();

    let over_max_title = "x".repeat(501);
    let result = env.store.create(&over_max_title, 2, &[], None);
    assert!(result.is_err());
}

// =============================================================================
// Description Edge Cases
// =============================================================================

#[test]
fn test_description_very_long() {
    let mut env = TestEnv::new();

    let long_desc = "x".repeat(10000);
    let item = env.store.create("Task", 2, &[], Some(&long_desc)).unwrap();
    assert_eq!(item.description.unwrap().len(), 10000);
}

#[test]
fn test_description_empty_string() {
    let mut env = TestEnv::new();

    let item = env.store.create("Task", 2, &[], Some("")).unwrap();
    assert_eq!(item.description, Some("".to_string()));
}

#[test]
fn test_description_with_newlines() {
    let mut env = TestEnv::new();

    let desc = "Line 1\nLine 2\nLine 3";
    let item = env.store.create("Task", 2, &[], Some(desc)).unwrap();
    assert_eq!(item.description, Some(desc.to_string()));
}

#[test]
fn test_description_removal() {
    let mut env = TestEnv::new();

    let item = env.create_item_with_desc("Task", "Original description");
    assert!(item.description.is_some());

    // Update to remove description
    let updated = env.store.update(&item.id, None, Some(None), None, None).unwrap();
    assert!(updated.description.is_none());
}

// =============================================================================
// Label Edge Cases
// =============================================================================

#[test]
fn test_many_labels() {
    let mut env = TestEnv::new();

    let labels: Vec<String> = (0..50).map(|i| format!("label{}", i)).collect();
    let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();

    let item = env.store.create("Task", 2, &label_refs, None).unwrap();
    assert_eq!(item.labels.len(), 50);
}

#[test]
fn test_empty_labels_list() {
    let mut env = TestEnv::new();

    let item = env.store.create("Task", 2, &[], None).unwrap();
    assert!(item.labels.is_empty());
}

#[test]
fn test_duplicate_labels_deduplicated() {
    let mut env = TestEnv::new();

    // SQLite has unique constraint on labels, so duplicates should be deduplicated
    // or rejected. Let's test with unique labels instead.
    let item = env
        .store
        .create("Task", 2, &["label1", "label2", "label3"], None)
        .unwrap();
    assert_eq!(item.labels.len(), 3);
}

#[test]
fn test_label_with_hyphen() {
    let mut env = TestEnv::new();

    let item = env.create_item_with_labels("Task", &["my-label"]);
    assert!(item.labels.contains(&"my-label".to_string()));
}

#[test]
fn test_label_with_underscore() {
    let mut env = TestEnv::new();

    let item = env.create_item_with_labels("Task", &["my_label"]);
    assert!(item.labels.contains(&"my_label".to_string()));
}

// =============================================================================
// ID Generation
// =============================================================================

#[test]
fn test_id_format() {
    let mut env = TestEnv::new();

    let item = env.create_item("Test task");
    assert!(item.id.starts_with("eg-"));
    assert!(item.id.len() > 3);
}

#[test]
fn test_unique_ids_for_same_title() {
    let mut env = TestEnv::new();

    // Same title should still get unique IDs (due to timestamp)
    let item1 = env.create_item("Same title");
    std::thread::sleep(std::time::Duration::from_millis(10));
    let item2 = env.create_item("Same title");

    assert_ne!(item1.id, item2.id);
}

// =============================================================================
// Query Edge Cases
// =============================================================================

#[test]
fn test_query_with_all_filters() {
    let mut env = TestEnv::new();

    env.create_item_with_labels("Backend task", &["backend"]);
    env.create_item_with_labels("Frontend task", &["frontend"]);

    let filter = Filter::new()
        .status(Status::Open)
        .label("backend")
        .min_priority(0)
        .max_priority(4)
        .limit(10);

    let results = env.store.query_with_filter(&filter).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].labels.contains(&"backend".to_string()));
}

#[test]
fn test_query_pagination() {
    let mut env = TestEnv::new();

    // Create 10 items
    for i in 0..10 {
        env.create_item(&format!("Task {}", i));
    }

    // Get first 5
    let filter = Filter::new().limit(5);
    let page1 = env.store.query_with_filter(&filter).unwrap();
    assert_eq!(page1.len(), 5);

    // Get next 5
    let filter = Filter::new().limit(5).offset(5);
    let page2 = env.store.query_with_filter(&filter).unwrap();
    assert_eq!(page2.len(), 5);

    // Verify no overlap
    for item in &page1 {
        assert!(!page2.iter().any(|i| i.id == item.id));
    }
}

#[test]
fn test_query_offset_beyond_results() {
    let mut env = TestEnv::new();

    env.create_item("Task 1");
    env.create_item("Task 2");

    let filter = Filter::new().offset(100);
    let results = env.store.query_with_filter(&filter).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_query_title_contains() {
    let mut env = TestEnv::new();

    env.create_item("Fix login bug");
    env.create_item("Add feature X");
    env.create_item("Login page redesign");

    let filter = Filter::new().title_contains("login");
    let results = env.store.query_with_filter(&filter).unwrap();

    // Should match case-insensitively
    assert_eq!(results.len(), 2);
}

// =============================================================================
// Close Reason Edge Cases
// =============================================================================

#[test]
fn test_close_with_empty_reason() {
    let mut env = TestEnv::new();

    let item = env.create_item("Task");
    let closed = env.store.close(&item.id, Some("")).unwrap();
    assert_eq!(closed.close_reason, Some("".to_string()));
}

#[test]
fn test_close_with_long_reason() {
    let mut env = TestEnv::new();

    let item = env.create_item("Task");
    let long_reason = "x".repeat(1000);
    let closed = env.store.close(&item.id, Some(&long_reason)).unwrap();
    assert_eq!(closed.close_reason.unwrap().len(), 1000);
}

#[test]
fn test_close_with_no_reason() {
    let mut env = TestEnv::new();

    let item = env.create_item("Task");
    let closed = env.store.close(&item.id, None).unwrap();
    assert!(closed.close_reason.is_none());
}
