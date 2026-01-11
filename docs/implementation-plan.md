# Engram Implementation Plan

**Author:** Claude (AI Assistant)
**Date:** 2026-01-10
**Status:** Completed
**Review Passes:** 5/5

## Summary

This document describes the implementation plan used to build Engram, a minimal git-backed task graph library for AI orchestration. The implementation was completed in four phases, progressing from core data types through to polish APIs.

## Problem Statement

### Background

Neuraphage, an AI orchestration system, requires a persistent task management system that:
- Tracks work items with priorities and labels
- Manages dependencies between tasks (blocking relationships)
- Identifies ready work (tasks with no open blockers)
- Persists data in a git-friendly format for version control
- Provides fast queries via SQLite caching

### Goals

- Minimal, focused API for task graph operations
- Git-backed persistence (JSONL source of truth)
- SQLite query cache for performance
- Support for concurrent access via daemon
- Rust implementation with strong type safety

### Non-Goals

- Full-featured issue tracker (no comments, attachments, etc.)
- Web UI or REST API
- Multi-user collaboration features
- Real-time notifications

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      Public API (Store)                      │
├─────────────────────────────────────────────────────────────┤
│  create() | get() | update() | close() | ready() | blocked()│
│  add_edge() | remove_edge() | list() | query()              │
├─────────────────────────────────────────────────────────────┤
│                    Extension Traits                          │
│  StoreBuilderExt | StoreBatchExt | StoreQueryExt            │
│  StoreCompactExt                                             │
├─────────────────────────────────────────────────────────────┤
│                   Storage Layer                              │
│  JSONL append-only log  ←→  SQLite query cache              │
├─────────────────────────────────────────────────────────────┤
│                   Daemon (Optional)                          │
│  Unix socket IPC for concurrent access                       │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Writes**: Append to JSONL → Update SQLite cache
2. **Reads**: Query SQLite cache (rebuilt from JSONL on startup)
3. **Daemon**: Serializes access via request/response protocol

## Implementation Phases

### Phase 1: Core Data Types and Storage

**Objective:** Establish foundational types and persistence layer.

**Components:**
- `types.rs` - Core data types (Item, Edge, Status, EdgeKind)
- `id.rs` - ID generation (hash-based, deterministic)
- `storage.rs` - JSONL + SQLite storage layer
- `store.rs` - High-level Store API

**Key Features:**
- Item validation (title length, priority bounds, label format)
- Status state machine (Open → InProgress → Closed, with Blocked)
- Cycle detection for blocking edges
- JSONL append-only log for git-friendliness
- SQLite cache for efficient queries

**Deliverables:**
- [x] Item struct with validation
- [x] Edge struct with EdgeKind enum
- [x] Status enum with transition rules
- [x] Storage layer with JSONL + SQLite
- [x] Store CRUD operations
- [x] `ready()` query (open items with no open blockers)
- [x] Basic test coverage

### Phase 2: Daemon for Concurrent Access

**Objective:** Enable multiple processes to safely access the store.

**Components:**
- `daemon.rs` - Unix socket server
- `protocol.rs` - Request/Response serialization
- `client.rs` - Client for connecting to daemon

**Key Features:**
- Unix domain socket IPC
- JSON-serialized request/response protocol
- PID file for daemon detection
- Graceful shutdown handling

**Deliverables:**
- [x] DaemonConfig with socket path, PID file
- [x] Request enum (Create, Get, List, Close, Ready, etc.)
- [x] Response enum with results
- [x] Daemon server implementation
- [x] Client for connecting to daemon
- [x] `is_daemon_running()` utility

### Phase 3: Graph Queries and Relationships

**Objective:** Enhanced query capabilities for the task graph.

**Components:**
- Enhanced `storage.rs` - Blocking queries
- Enhanced `store.rs` - Graph operations

**Key Features:**
- `blocked()` query (items blocked by open items)
- Parent-child relationships via edges
- Edge removal (soft delete in JSONL)
- Idempotent edge operations

**Deliverables:**
- [x] `blocked()` query implementation
- [x] `remove_edge()` operation
- [x] Edge idempotency (re-adding existing edge is no-op)
- [x] Blocking edge enumeration

### Phase 4: Polish APIs and Maintenance

**Objective:** Developer-friendly APIs and operational tooling.

**Components:**
- `builder.rs` - Fluent builder API
- `batch.rs` - Bulk operations
- `query.rs` - Flexible query filters
- `compact.rs` - Storage compaction
- `vacuum.rs` - Database maintenance

**Key Features:**
- Builder pattern for item creation
- Batch create/close/status operations
- Query filters (status, labels, priority, title search, pagination)
- Compaction of old closed items (description removal)
- SQLite vacuum for space reclamation

**Deliverables:**
- [x] ItemBuilder with fluent API
- [x] batch_create(), batch_close(), batch_set_status()
- [x] Query builder with Filter
- [x] CompactConfig and compact()
- [x] vacuum() function
- [x] Full test coverage (47 tests)

## Data Model

### Item

```rust
pub struct Item {
    pub id: String,           // eg-{hash} format
    pub title: String,        // 1-500 chars, no control chars
    pub description: Option<String>,
    pub status: Status,       // Open, InProgress, Blocked, Closed
    pub priority: u8,         // 0 (critical) - 4 (low)
    pub labels: Vec<String>,  // Tags for categorization
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub close_reason: Option<String>,
}
```

### Edge

```rust
pub struct Edge {
    pub from_id: String,      // Source item
    pub to_id: String,        // Target item
    pub kind: EdgeKind,       // Blocks, Related
    pub created_at: DateTime<Utc>,
    pub deleted: bool,        // Soft delete flag
}
```

### Status State Machine

```
     ┌──────────────────┐
     │                  │
     ▼                  │
   Open ──────► InProgress ──────► Closed
     │              │                 ▲
     │              │                 │
     ▼              ▼                 │
  Blocked ◄────────┴─────────────────┘
     │
     └──────► Open (can unblock)
```

## File Structure

```
engram/
├── Cargo.toml
├── src/
│   ├── lib.rs          # Public API re-exports
│   ├── main.rs         # CLI entry point
│   ├── id.rs           # ID generation
│   ├── types.rs        # Core types + Filter
│   ├── storage.rs      # JSONL + SQLite layer
│   ├── store.rs        # High-level Store API
│   ├── daemon.rs       # Unix socket daemon
│   ├── protocol.rs     # Request/Response protocol
│   ├── client.rs       # Daemon client
│   ├── builder.rs      # Fluent builder API
│   ├── batch.rs        # Bulk operations
│   ├── query.rs        # Query builder
│   ├── compact.rs      # Compaction operations
│   └── vacuum.rs       # Database vacuum
└── docs/
    └── implementation-plan.md
```

## Storage Layout

```
.engram/
├── items.jsonl         # Append-only item log
├── edges.jsonl         # Append-only edge log
└── cache.db            # SQLite query cache
```

## API Examples

### Basic Usage

```rust
use engram::{Store, Status, EdgeKind};

// Initialize store
let mut store = Store::init(Path::new(".")).unwrap();

// Create items
let task1 = store.create("Implement login", 1, &["auth"], None).unwrap();
let task2 = store.create("Write tests", 2, &["auth", "test"], None).unwrap();

// Add dependency (task2 blocks on task1)
store.add_edge(&task2.id, &task1.id, EdgeKind::Blocks).unwrap();

// Query ready work
let ready = store.ready().unwrap();
assert_eq!(ready.len(), 1);
assert_eq!(ready[0].id, task1.id);

// Close task1
store.close(&task1.id, Some("Implemented OAuth")).unwrap();

// Now task2 is ready
let ready = store.ready().unwrap();
assert_eq!(ready[0].id, task2.id);
```

### Builder Pattern

```rust
use engram::{Store, StoreBuilderExt};

let item = store.build("Complex task")
    .priority(1)
    .label("backend")
    .label("urgent")
    .description("Detailed description here")
    .create()
    .unwrap();
```

### Batch Operations

```rust
use engram::{Store, StoreBatchExt, CreateSpec};

let specs = vec![
    CreateSpec::new("Task 1", 1),
    CreateSpec::new("Task 2", 2).labels(vec!["test"]),
];
let result = store.batch_create(specs).unwrap();
```

### Query with Filters

```rust
use engram::{Store, StoreQueryExt, Filter, Status};

let high_priority = store.query()
    .status(Status::Open)
    .max_priority(1)
    .label("backend")
    .limit(10)
    .execute()
    .unwrap();
```

## Technical Decisions

### JSONL as Source of Truth

**Decision:** Use append-only JSONL files for persistence.

**Rationale:**
- Git-friendly: Each append is a new line, easy to diff/merge
- Simple: No complex file format or binary encoding
- Recoverable: Human-readable, easy to repair
- Append-only: Natural audit trail

**Trade-offs:**
- File size grows over time (mitigated by compaction)
- Slower full rebuilds (mitigated by SQLite cache)

### SQLite as Query Cache

**Decision:** Use SQLite as an ephemeral query cache.

**Rationale:**
- Fast complex queries (ready work, filtering)
- Can be rebuilt from JSONL at any time
- Transactional consistency for multi-item operations
- Well-tested, reliable

**Trade-offs:**
- Requires cache rebuild on startup
- Additional dependency (rusqlite)

### Hash-based IDs

**Decision:** Generate IDs as `eg-{base32(hash(title+timestamp))}`.

**Rationale:**
- Deterministic: Same input produces same ID
- Short: 13 characters total
- URL-safe: Base32 encoding
- Collision-resistant: SHA256 truncated

### Extension Traits

**Decision:** Use extension traits for optional functionality.

**Rationale:**
- Keeps core Store API minimal
- Opt-in features via trait imports
- Clear separation of concerns
- Easy to extend without modifying Store

## Testing Strategy

See `/home/saidler/docs/engram-test-spec.md` for the comprehensive test specification.

**Current Coverage:**
- 47 unit tests across all modules
- All core operations tested
- Cycle detection validated
- Status transitions verified

## Future Considerations

### Potential Enhancements

1. **Watch/Subscribe** - Real-time notifications of changes
2. **Comments** - Notes on items (like Beads has)
3. **Assignments** - User ownership of items
4. **Time Tracking** - Estimates and actuals
5. **History** - Change audit log

### Performance Optimizations

1. **Blocked Cache** - Cache blocked items list (like Beads)
2. **Incremental Rebuild** - Only rebuild changed items
3. **Connection Pooling** - For high-concurrency scenarios

## References

- Beads (Go reference): `~/repos/steveyegge/beads`
- Rust CLI conventions: `~/.claude/skills/rust-cli-coder`
- Test specification: `/home/saidler/docs/engram-test-spec.md`

---

## Implementation Log

### Phase 1 (2026-01-10)
- Implemented core types, storage, and store
- 15 tests passing

### Phase 2 (2026-01-10)
- Added daemon, protocol, client modules
- 25 tests passing

### Phase 3 (2026-01-10)
- Added blocked() query, edge removal
- 35 tests passing

### Phase 4 (2026-01-10)
- Added builder, batch, query, compact, vacuum
- 47 tests passing
- All phases complete
