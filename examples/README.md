# Engram CLI Examples

This directory contains example scripts demonstrating how to use the `engram` CLI.

## Prerequisites

Make sure `engram` is installed and available in your PATH:

```bash
cargo install --path .
```

## Running Examples

Each example creates a temporary directory for its engram store, so they won't
affect your actual data. Run any example:

```bash
./01-basic-crud.sh
```

Or run in a specific directory:

```bash
EXAMPLE_DIR=/tmp/my-test ./01-basic-crud.sh
```

## Example Index

| # | File | Description |
|---|------|-------------|
| 01 | basic-crud.sh | Create, list, get, and close tasks |
| 02 | priorities.sh | Priority levels P0 (critical) to P4 (backlog) |
| 03 | labels.sh | Tagging and filtering with labels |
| 04 | dependencies.sh | Blocking relationships between tasks |
| 05 | ready-work.sh | Finding actionable items with `ready` |
| 06 | workflow-status.sh | Status transitions (Open → InProgress → Closed) |
| 07 | parent-child.sh | Hierarchical task organization |
| 08 | descriptions.sh | Adding detailed descriptions to tasks |
| 09 | daemon-mode.sh | Running engram daemon for concurrent access |
| 10 | full-project.sh | Complete project workflow simulation |
| 11 | querying.sh | Filtering and searching tasks |
| 12 | git-integration.sh | Understanding git-backed storage |
| 13 | complex-dependencies.sh | Diamond and chain dependency patterns |
| 14 | quick-reference.sh | Command reference with examples |
| 15 | blocked-workflow.sh | Managing blocked tasks |

## Quick Start

Start with `01-basic-crud.sh` to learn the fundamentals, then explore based on
your needs:

- **Task management**: 01, 02, 03, 08
- **Dependencies**: 04, 05, 13, 15
- **Status workflow**: 06
- **Organization**: 03, 07, 11
- **Architecture**: 09, 12
- **Full workflows**: 10

## Notes

- Examples use `mktemp -d` to create isolated test directories
- The `.gitignore` in this directory excludes `.engram/` directories
- Each example is self-contained and can be run independently
