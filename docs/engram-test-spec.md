# Design Document: Engram Test Specification

**Author:** Claude (AI Assistant)
**Date:** 2026-01-10
**Status:** Draft
**Review Passes:** 5/5

## Summary

This document defines a comprehensive test specification for the Rust-based Engram library, using the Go-based Beads test suite as a reference spec. The goal is to achieve feature parity and correctness validation by implementing equivalent test scenarios that cover task management, graph operations, and storage layer functionality.

## Problem Statement

### Background

Engram is a minimal git-backed task graph library for AI orchestration. It was implemented in Rust with JSONL as the source of truth and SQLite as a query cache. The Beads project is a mature Go implementation with 322+ test files covering similar functionality.

### Problem

The Engram implementation needs comprehensive test coverage to validate:
1. Core functionality matches expected behavior
2. Edge cases and error conditions are handled correctly
3. Performance characteristics are acceptable
4. The implementation can serve as a reliable foundation for Neuraphage

### Goals

- Achieve 90%+ test coverage on Engram core modules
- Validate all critical operations match Beads reference behavior
- Establish performance baselines for key operations
- Document test patterns for future development

### Non-Goals

- 1:1 test porting from Go to Rust (adapt to Rust idioms)
- Testing Beads-specific features not in Engram (e.g., federation)
- Benchmarking against Beads (different languages, different goals)
- Testing the daemon/IPC layer extensively (focus on core library)

## Proposed Solution

### Overview

Implement a test suite organized into tiers:
1. **Unit tests** - Individual function/module validation
2. **Integration tests** - Cross-module behavior validation
3. **Property tests** - Invariant validation with generated data
4. **Benchmark tests** - Performance baseline establishment

### Test Categories (from Beads Reference)

#### 1. Basic Operations Tests

| Test Area | Beads Reference | Engram Equivalent |
|-----------|-----------------|-------------------|
| Item CRUD | sqlite_test.go | store::tests |
| Labels | labels_test.go | types::tests, query::tests |
| Validation | types validation | types::tests |

**Key Scenarios:**
```rust
// Create and retrieve
let item = store.create("Test task", 2, &["label"], Some("desc")).unwrap();
let retrieved = store.get(&item.id).unwrap().unwrap();
assert_eq!(retrieved.title, "Test task");

// Update
let updated = store.update(&item.id, Some("New title"), None, None, None).unwrap();
assert_eq!(updated.title, "New title");

// List by status
let open = store.list(Some(Status::Open)).unwrap();
assert!(open.iter().any(|i| i.id == item.id));
```

#### 2. Graph Operations Tests

| Test Area | Beads Reference | Engram Equivalent |
|-----------|-----------------|-------------------|
| Dependencies | dependencies_test.go | store::tests |
| Cycle Detection | cycle_detection_test.go | store::tests |
| Parent-Child | child_id_test.go, graph_links_test.go | store::tests |
| Blocking | blocked_cache_test.go | storage::tests |

**Key Scenarios:**
```rust
// Ready work calculation
// Issue1: open, no deps -> READY
// Issue2: open, depends on Issue1 (open) -> BLOCKED
// Issue3: open, depends on Issue1 (closed) -> READY
let blocker = store.create("Blocker", 1, &[], None).unwrap();
let blocked = store.create("Blocked", 2, &[], None).unwrap();
store.add_edge(&blocked.id, &blocker.id, EdgeKind::Blocks).unwrap();

let ready = store.ready().unwrap();
assert!(ready.iter().any(|i| i.id == blocker.id));
assert!(!ready.iter().any(|i| i.id == blocked.id));

store.close(&blocker.id, None).unwrap();
let ready = store.ready().unwrap();
assert!(ready.iter().any(|i| i.id == blocked.id));

// Cycle detection
let a = store.create("A", 2, &[], None).unwrap();
let b = store.create("B", 2, &[], None).unwrap();
let c = store.create("C", 2, &[], None).unwrap();
store.add_edge(&a.id, &b.id, EdgeKind::Blocks).unwrap();
store.add_edge(&b.id, &c.id, EdgeKind::Blocks).unwrap();
let result = store.add_edge(&c.id, &a.id, EdgeKind::Blocks);
assert!(result.is_err()); // Cycle detected
```

#### 3. Query and Filter Tests

| Test Area | Beads Reference | Engram Equivalent |
|-----------|-----------------|-------------------|
| Search | search in sqlite_test.go | query::tests |
| Filtering | list with filters | query::tests |
| Pagination | limit/offset handling | query::tests |

**Key Scenarios:**
```rust
// Filter by status
let filter = Filter::new().status(Status::Open);
let open = store.query_with_filter(&filter).unwrap();

// Filter by label
let filter = Filter::new().label("backend");
let backend = store.query_with_filter(&filter).unwrap();

// Filter by priority range
let filter = Filter::new().min_priority(1).max_priority(2);
let mid_priority = store.query_with_filter(&filter).unwrap();

// Pagination
let filter = Filter::new().limit(10).offset(20);
let page = store.query_with_filter(&filter).unwrap();
```

#### 4. Status Transition Tests

| Test Area | Beads Reference | Engram Equivalent |
|-----------|-----------------|-------------------|
| Valid transitions | types validation | types::tests |
| Invalid transitions | negative tests | store::tests |
| Close with reason | sqlite_test.go | store::tests |

**Key Scenarios:**
```rust
// Valid: Open -> InProgress -> Closed
let task1 = store.create("Task 1", 2, &[], None).unwrap();
store.set_status(&task1.id, Status::InProgress).unwrap();
store.close(&task1.id, Some("Done")).unwrap();

// Invalid: Closed -> InProgress
let task2 = store.create("Task 2", 2, &[], None).unwrap();
store.close(&task2.id, None).unwrap();
let result = store.set_status(&task2.id, Status::InProgress);
assert!(result.is_err());

// Valid: Open -> Blocked -> Open
let task3 = store.create("Task 3", 2, &[], None).unwrap();
store.set_status(&task3.id, Status::Blocked).unwrap();
store.set_status(&task3.id, Status::Open).unwrap();
```

#### 5. Batch Operations Tests

| Test Area | Beads Reference | Engram Equivalent |
|-----------|-----------------|-------------------|
| Bulk create | batch operations | batch::tests |
| Bulk close | batch close | batch::tests |
| Bulk status | batch update | batch::tests |

**Key Scenarios:**
```rust
// Batch create
let specs = vec![
    CreateSpec::new("Task 1", 1),
    CreateSpec::new("Task 2", 2).labels(vec!["test"]),
    CreateSpec::new("Task 3", 3).description("Desc"),
];
let result = store.batch_create(specs).unwrap();
assert_eq!(result.created.len(), 3);

// Batch close
let ids: Vec<&str> = result.created.iter().map(|i| i.id.as_str()).collect();
let closed = store.batch_close(&ids, Some("Done")).unwrap();
assert_eq!(closed.closed.len(), 3);
```

#### 6. Validation Tests

| Test Area | Beads Reference | Engram Equivalent |
|-----------|-----------------|-------------------|
| Title validation | types validation | types::tests |
| Priority bounds | 0-4 validation | types::tests |
| Label format | label validation | types::tests |

**Key Scenarios:**
```rust
// Empty title rejected
let result = store.create("", 2, &[], None);
assert!(result.is_err());

// Title too long rejected (>500 chars)
let long_title = "x".repeat(501);
let result = store.create(&long_title, 2, &[], None);
assert!(result.is_err());

// Invalid priority rejected
let result = store.create("Task", 5, &[], None);
assert!(result.is_err());

// Invalid label rejected (control chars)
let result = store.create("Task", 2, &["bad\nlabel"], None);
assert!(result.is_err());
```

#### 7. Storage Layer Tests

| Test Area | Beads Reference | Engram Equivalent |
|-----------|-----------------|-------------------|
| Init/Open | sqlite_test.go | storage::tests |
| JSONL append | storage tests | storage::tests |
| SQLite cache | cache tests | storage::tests |
| Vacuum | compact_bench_test.go | vacuum::tests |

**Key Scenarios:**
```rust
// Init creates directories and files
let temp = TempDir::new().unwrap();
Store::init(temp.path()).unwrap();
assert!(temp.path().join(".engram").exists());
assert!(temp.path().join(".engram/items.jsonl").exists());
assert!(temp.path().join(".engram/edges.jsonl").exists());
assert!(temp.path().join(".engram/cache.db").exists());

// Vacuum reclaims space
let result = vacuum(temp.path()).unwrap();
assert!(result.size_after <= result.size_before);
```

#### 8. Edge Removal Tests

| Test Area | Beads Reference | Engram Equivalent |
|-----------|-----------------|-------------------|
| Remove edge | dependencies_test.go | store::tests |
| Idempotent removal | negative tests | store::tests |
| Edge not found | error handling | store::tests |

**Key Scenarios:**
```rust
// Remove blocking dependency
let blocker = store.create("Blocker", 2, &[], None).unwrap();
let blocked = store.create("Blocked", 2, &[], None).unwrap();
store.add_edge(&blocked.id, &blocker.id, EdgeKind::Blocks).unwrap();

// blocked is blocked by blocker
let ready = store.ready().unwrap();
assert!(ready.iter().any(|i| i.id == blocker.id));   // blocker is ready
assert!(!ready.iter().any(|i| i.id == blocked.id));  // blocked is NOT ready

// Remove the edge
store.remove_edge(&blocked.id, &blocker.id, EdgeKind::Blocks).unwrap();

// blocked is now ready (no longer blocked)
let ready = store.ready().unwrap();
assert!(ready.iter().any(|i| i.id == blocked.id));

// Idempotent removal (no error on second removal)
store.remove_edge(&blocked.id, &blocker.id, EdgeKind::Blocks).unwrap();
```

#### 9. Error Handling Tests

| Test Area | Beads Reference | Engram Equivalent |
|-----------|-----------------|-------------------|
| Item not found | negative tests | store::tests |
| Invalid operations | validation tests | store::tests |
| Storage errors | error propagation | storage::tests |

**Key Scenarios:**
```rust
// Get non-existent item
let result = store.get("eg-nonexistent").unwrap();
assert!(result.is_none());

// Update non-existent item
let result = store.update("eg-nonexistent", Some("title"), None, None, None);
assert!(result.is_err());

// Close non-existent item
let result = store.close("eg-nonexistent", None);
assert!(result.is_err());

// Add edge with non-existent item
let item = store.create("Task", 2, &[], None).unwrap();
let result = store.add_edge(&item.id, "eg-nonexistent", EdgeKind::Blocks);
assert!(result.is_err());
```

#### 10. Edge Case Tests

| Test Area | Beads Reference | Engram Equivalent |
|-----------|-----------------|-------------------|
| Empty store | sqlite_test.go | store::tests |
| Unicode handling | validation tests | types::tests |
| Boundary values | types validation | types::tests |

**Key Scenarios:**
```rust
// Empty store operations
let ready = store.ready().unwrap();
assert!(ready.is_empty());

let blocked = store.blocked().unwrap();
assert!(blocked.is_empty());

let all = store.list(None).unwrap();
assert!(all.is_empty());

// Unicode titles and labels
let item = store.create("Task with emoji: rocket", 2, &["label"], None).unwrap();
assert_eq!(item.title, "Task with emoji: rocket");

// Chinese characters
let item = store.create("Chinese title", 2, &[], None).unwrap();
assert!(item.id.starts_with("eg-"));

// Priority boundary values
let p0 = store.create("Critical", 0, &[], None).unwrap();
assert_eq!(p0.priority, 0);

let p4 = store.create("Low priority", 4, &[], None).unwrap();
assert_eq!(p4.priority, 4);

// Very long description (within limits)
let long_desc = "x".repeat(10000);
let item = store.create("Task", 2, &[], Some(&long_desc)).unwrap();
assert_eq!(item.description.unwrap().len(), 10000);

// Many labels
let labels: Vec<&str> = (0..50).map(|i| "label").collect();
let item = store.create("Task", 2, &labels, None).unwrap();
assert_eq!(item.labels.len(), 50);
```

#### 11. Compaction Tests

| Test Area | Beads Reference | Engram Equivalent |
|-----------|-----------------|-------------------|
| Description truncation | compact operations | compact::tests |
| Age filtering | time-based compaction | compact::tests |

**Key Scenarios:**
```rust
// Compact old items
let config = CompactConfig::new()
    .older_than_days(7)
    .max_description_len(None); // Remove descriptions entirely

let result = store.compact(&config).unwrap();
// Items closed >7 days ago have descriptions removed
```

### Implementation Plan

#### Phase 1: Core Test Infrastructure
1. Create test helper module with `TestEnv` struct
2. Implement factory functions for common test scenarios
3. Set up property testing with `proptest` crate

#### Phase 2: Unit Tests (Existing + New)
1. Expand `store::tests` for all CRUD operations
2. Expand `types::tests` for all validation rules
3. Add `storage::tests` for JSONL/SQLite operations

#### Phase 3: Integration Tests
1. Add `tests/integration/` directory
2. Implement graph operation integration tests
3. Implement query/filter integration tests

#### Phase 4: Property Tests
1. Add property tests for ID generation uniqueness
2. Add property tests for graph invariants (no cycles)
3. Add property tests for status transition validity

#### Phase 5: Benchmark Tests
1. Add `benches/` directory with criterion
2. Benchmark ready() with 1K, 10K items
3. Benchmark query operations

### Test Organization

```
engram/
├── src/
│   ├── lib.rs
│   ├── store.rs          # Unit tests in #[cfg(test)] mod tests
│   ├── storage.rs        # Unit tests in #[cfg(test)] mod tests
│   ├── types.rs          # Validation tests in #[cfg(test)] mod tests
│   ├── batch.rs          # Batch tests in #[cfg(test)] mod tests
│   ├── builder.rs        # Builder tests in #[cfg(test)] mod tests
│   ├── query.rs          # Query tests in #[cfg(test)] mod tests
│   ├── compact.rs        # Compaction tests in #[cfg(test)] mod tests
│   └── vacuum.rs         # Vacuum tests in #[cfg(test)] mod tests
├── tests/
│   ├── common/
│   │   └── mod.rs        # Shared test helpers (TestEnv)
│   ├── integration/
│   │   ├── graph_test.rs # Graph operation integration tests
│   │   └── query_test.rs # Query integration tests
│   └── proptest/
│       └── invariants.rs # Property-based tests
└── benches/
    └── ready.rs          # Criterion benchmarks
```

### CI/CD Integration

Tests run via `otto ci` which executes:
1. `cargo check` - Compilation verification
2. `cargo clippy` - Lint checks
3. `cargo fmt --check` - Format verification
4. `cargo test` - All unit and integration tests

For benchmarks (manual):
```bash
cargo bench --bench ready
```

### Test Infrastructure

```rust
// tests/helpers/mod.rs
pub struct TestEnv {
    pub temp_dir: TempDir,
    pub store: Store,
}

impl TestEnv {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let store = Store::init(temp_dir.path()).unwrap();
        Self { temp_dir, store }
    }

    pub fn create_item(&mut self, title: &str) -> Item {
        self.store.create(title, 2, &[], None).unwrap()
    }

    pub fn add_blocking_dep(&mut self, from: &Item, to: &Item) -> Edge {
        self.store.add_edge(&from.id, &to.id, EdgeKind::Blocks).unwrap()
    }

    pub fn assert_ready(&self, item: &Item) {
        let ready = self.store.ready().unwrap();
        assert!(ready.iter().any(|i| i.id == item.id));
    }

    pub fn assert_blocked(&self, item: &Item) {
        let ready = self.store.ready().unwrap();
        assert!(!ready.iter().any(|i| i.id == item.id));
    }
}
```

## Alternatives Considered

### Alternative 1: Direct Test Port

- **Description:** Translate Go tests directly to Rust line-by-line
- **Pros:** Exact parity, no interpretation errors
- **Cons:** Go idioms don't translate well, maintenance burden
- **Why not chosen:** Rust has different testing patterns and capabilities

### Alternative 2: Minimal Test Suite

- **Description:** Only test public API happy paths
- **Pros:** Fast to implement, low maintenance
- **Cons:** Misses edge cases, low confidence
- **Why not chosen:** Insufficient for production reliability

### Alternative 3: Property-Only Testing

- **Description:** Use only property-based testing with generators
- **Pros:** High coverage with less code, finds edge cases
- **Cons:** Hard to debug failures, slow execution
- **Why not chosen:** Complementary approach is better

## Technical Considerations

### Dependencies

```toml
[dev-dependencies]
tempfile = "3"
proptest = "1"
criterion = "0.5"
```

### Performance

- Unit tests: <1ms per test
- Integration tests: <100ms per test
- Property tests: <1s per property (100 cases)
- Benchmarks: 1s warm-up + 3s measurement

### Security

- Tests use temporary directories only
- No external network access
- No privileged operations

### Testing Strategy

| Level | Coverage Target | Focus |
|-------|-----------------|-------|
| Unit | 90%+ | Function correctness |
| Integration | 80%+ | Cross-module behavior |
| Property | Key invariants | Correctness under random input |
| Benchmark | Key operations | Performance regression detection |

### Expected Test Counts

| Category | Current | Target | Notes |
|----------|---------|--------|-------|
| Unit tests | 47 | 80+ | Expand existing modules |
| Integration tests | 0 | 15+ | New tests/ directory |
| Property tests | 0 | 10+ | Invariant validation |
| Benchmarks | 0 | 5+ | Performance baselines |
| **Total** | **47** | **110+** | |

### Rollout Plan

1. Merge Phase 1 (infrastructure) first
2. Phases 2-4 can be incremental PRs
3. Phase 5 (benchmarks) after baseline functionality

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Beads features missing in Engram | Medium | Low | Document gaps, add tests as features are implemented |
| Property tests find bugs | High | Medium | Fix bugs before release, valuable finding |
| Performance regressions | Low | Medium | Benchmark tests catch early |
| Test maintenance burden | Medium | Low | Good test helpers reduce duplication |
| Time-based test flakiness | Medium | Medium | Use mocked time or generous thresholds |
| SQLite locking in parallel tests | Low | High | Use separate temp dirs per test |

## Open Questions

- [ ] Should we add fuzzing tests for deserialization?
- [ ] What is the acceptable performance baseline for ready()?
- [ ] Should compaction tests use mocked time or real age?
- [ ] Do we need concurrent access tests for the daemon?
- [ ] Should we test JSONL file corruption recovery?
- [ ] What edge kinds beyond Blocks and Related should be tested?

## References

- Beads repository: `~/repos/steveyegge/beads`
- Beads test helpers: `internal/storage/sqlite/test_helpers.go`
- Engram source: `/home/saidler/repos/neuraphage/engram`
- Rust testing guide: https://doc.rust-lang.org/book/ch11-00-testing.html
- proptest crate: https://docs.rs/proptest
- criterion crate: https://docs.rs/criterion

---

## Review Log

### Pass 1: Completeness (2026-01-10)
- Added Edge Removal Tests section (section 8)
- Added Error Handling Tests section (section 9)
- Added risks for time-based test flakiness and SQLite locking
- Added open questions about concurrent access and corruption recovery

### Pass 2: Correctness (2026-01-10)
- Fixed edge removal test assertions (was checking wrong item)
- Fixed status transition tests (duplicate variable names)
- Verified Filter API matches actual implementation

### Pass 3: Edge Cases (2026-01-10)
- Added Edge Case Tests section (section 10)
- Added tests for empty store operations
- Added tests for unicode/special characters
- Added tests for priority boundary values (0 and 4)
- Added tests for large descriptions and many labels

### Pass 4: Architecture (2026-01-10)
- Added Test Organization section with directory structure
- Added CI/CD Integration section with otto ci workflow
- Clarified test file placement conventions

### Pass 5: Clarity (2026-01-10)
- Added Expected Test Counts table with current/target metrics
- Document converged - no significant changes needed
- Ready for implementation

**Final Status:** Document complete after 5 passes. Ready for implementation.
