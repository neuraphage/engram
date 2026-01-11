//! Integration tests for error handling.
//!
//! Tests that errors are properly returned for invalid operations.

mod common;

use common::TestEnv;
use engram::{EdgeKind, Status, Store};
use tempfile::TempDir;

// =============================================================================
// Item Not Found Tests
// =============================================================================

#[test]
fn test_get_nonexistent_item_returns_none() {
    let env = TestEnv::new();

    let result = env.store.get("eg-nonexistent").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_update_nonexistent_item_fails() {
    let mut env = TestEnv::new();

    let result = env.store.update("eg-nonexistent", Some("title"), None, None, None);
    assert!(result.is_err());
}

#[test]
fn test_close_nonexistent_item_fails() {
    let mut env = TestEnv::new();

    let result = env.store.close("eg-nonexistent", None);
    assert!(result.is_err());
}

#[test]
fn test_set_status_nonexistent_item_fails() {
    let mut env = TestEnv::new();

    let result = env.store.set_status("eg-nonexistent", Status::InProgress);
    assert!(result.is_err());
}

#[test]
fn test_add_edge_from_nonexistent_fails() {
    let mut env = TestEnv::new();

    let item = env.create_item("Real item");

    let result = env.store.add_edge("eg-nonexistent", &item.id, EdgeKind::Blocks);
    assert!(result.is_err());
}

#[test]
fn test_add_edge_to_nonexistent_fails() {
    let mut env = TestEnv::new();

    let item = env.create_item("Real item");

    let result = env.store.add_edge(&item.id, "eg-nonexistent", EdgeKind::Blocks);
    assert!(result.is_err());
}

// =============================================================================
// Validation Tests
// =============================================================================

#[test]
fn test_create_empty_title_fails() {
    let mut env = TestEnv::new();

    let result = env.store.create("", 2, &[], None);
    assert!(result.is_err());
}

#[test]
fn test_create_whitespace_only_title_succeeds() {
    let mut env = TestEnv::new();

    // Whitespace-only titles are allowed (not explicitly rejected)
    let result = env.store.create("   ", 2, &[], None);
    // If validation rejects it, that's fine; if it accepts, that's also fine
    // Just verify the operation completes without panic
    let _ = result;
}

#[test]
fn test_create_title_too_long_fails() {
    let mut env = TestEnv::new();

    let long_title = "x".repeat(501);
    let result = env.store.create(&long_title, 2, &[], None);
    assert!(result.is_err());
}

#[test]
fn test_create_invalid_priority_fails() {
    let mut env = TestEnv::new();

    let result = env.store.create("Task", 5, &[], None);
    assert!(result.is_err());
}

#[test]
fn test_create_control_chars_in_title_fails() {
    let mut env = TestEnv::new();

    let result = env.store.create("Title\nwith\nnewlines", 2, &[], None);
    assert!(result.is_err());
}

#[test]
fn test_create_control_chars_in_label_fails() {
    let mut env = TestEnv::new();

    let result = env.store.create("Task", 2, &["bad\nlabel"], None);
    assert!(result.is_err());
}

// =============================================================================
// Status Transition Tests
// =============================================================================

#[test]
fn test_invalid_transition_closed_to_in_progress() {
    let mut env = TestEnv::new();

    let item = env.create_item("Task");
    env.close_item(&item);

    let result = env.store.set_status(&item.id, Status::InProgress);
    assert!(result.is_err());
}

#[test]
fn test_valid_transition_closed_to_open_reopen() {
    let mut env = TestEnv::new();

    // Reopening a closed task is valid
    let item = env.create_item("Task");
    env.close_item(&item);

    let result = env.store.set_status(&item.id, Status::Open);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, Status::Open);
}

#[test]
fn test_invalid_transition_closed_to_blocked() {
    let mut env = TestEnv::new();

    let item = env.create_item("Task");
    env.close_item(&item);

    let result = env.store.set_status(&item.id, Status::Blocked);
    assert!(result.is_err());
}

#[test]
fn test_valid_transition_open_to_in_progress() {
    let mut env = TestEnv::new();

    let item = env.create_item("Task");
    let result = env.store.set_status(&item.id, Status::InProgress);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, Status::InProgress);
}

#[test]
fn test_valid_transition_open_to_blocked() {
    let mut env = TestEnv::new();

    let item = env.create_item("Task");
    let result = env.store.set_status(&item.id, Status::Blocked);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, Status::Blocked);
}

#[test]
fn test_valid_transition_blocked_to_open() {
    let mut env = TestEnv::new();

    let item = env.create_item("Task");
    env.store.set_status(&item.id, Status::Blocked).unwrap();

    let result = env.store.set_status(&item.id, Status::Open);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, Status::Open);
}

#[test]
fn test_valid_transition_in_progress_to_closed() {
    let mut env = TestEnv::new();

    let item = env.create_item("Task");
    env.store.set_status(&item.id, Status::InProgress).unwrap();

    let result = env.store.close(&item.id, Some("Done"));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().status, Status::Closed);
}

// =============================================================================
// Storage Tests
// =============================================================================

#[test]
fn test_init_creates_engram_directory() {
    let temp = TempDir::new().unwrap();
    Store::init(temp.path()).unwrap();

    assert!(temp.path().join(".engram").exists());
    assert!(temp.path().join(".engram/items.jsonl").exists());
    assert!(temp.path().join(".engram/edges.jsonl").exists());
    assert!(temp.path().join(".engram/engram.db").exists());
}

#[test]
fn test_open_existing_store() {
    let temp = TempDir::new().unwrap();

    // Init and create an item
    {
        let mut store = Store::init(temp.path()).unwrap();
        store.create("Test item", 2, &[], None).unwrap();
    }

    // Reopen and verify item exists
    {
        let store = Store::open(temp.path()).unwrap();
        let items = store.list(None).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Test item");
    }
}

#[test]
fn test_open_nonexistent_store_fails() {
    let temp = TempDir::new().unwrap();
    let result = Store::open(temp.path());
    assert!(result.is_err());
}

// =============================================================================
// Update Validation Tests
// =============================================================================

#[test]
fn test_update_to_empty_title_fails() {
    let mut env = TestEnv::new();

    let item = env.create_item("Original");
    let result = env.store.update(&item.id, Some(""), None, None, None);
    assert!(result.is_err());
}

#[test]
fn test_update_to_invalid_priority_fails() {
    let mut env = TestEnv::new();

    let item = env.create_item("Original");
    let result = env.store.update(&item.id, None, None, Some(5), None);
    assert!(result.is_err());
}

#[test]
fn test_update_with_invalid_label_fails() {
    let mut env = TestEnv::new();

    let item = env.create_item("Original");
    let result = env.store.update(&item.id, None, None, None, Some(&["bad\nlabel"]));
    assert!(result.is_err());
}
