#!/usr/bin/env bash
# 12-git-integration.sh - Understanding file-based storage
#
# Demonstrates how engram stores data and how it can be
# version controlled with git.
#
# Usage: ./12-git-integration.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== File-Based Storage ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

echo "--- Directory Structure After Init ---"
ls -la .engram/
echo

# Create some tasks
echo "--- Create Initial Tasks ---"
engram create "First task" --priority 1 --labels "initial"
engram create "Second task" --priority 2 --labels "initial"
echo

echo "--- View JSONL Source of Truth ---"
echo "The items.jsonl file contains all task data:"
cat .engram/items.jsonl
echo

# Make changes
echo "--- Make Changes ---"
FIRST=$(engram list | grep "^open.*First" | awk '{print $2}')
engram close "$FIRST" --reason "Completed"
engram create "Third task" --priority 3 --labels "added-later"
echo

echo "--- JSONL Shows Append-Only History ---"
echo "Each operation appends a new record:"
cat .engram/items.jsonl
echo

echo "--- SQLite Cache for Fast Queries ---"
echo "Cache file: .engram/engram.db"
ls -la .engram/engram.db
echo
echo "The cache is rebuilt from JSONL on startup."
echo "JSONL is the source of truth, SQLite is for performance."
echo

echo "=== Git Integration Workflow ==="
echo
echo "To version control your tasks:"
echo "  1. Add .engram/ to your project's git repo"
echo "  2. Commit changes: git add .engram && git commit -m 'Update tasks'"
echo "  3. Share with team via git push/pull"
echo "  4. JSONL format enables clean diffs and merges"
echo

# Demonstrate git integration (if in a git repo)
echo "--- Demonstrating Git Integration ---"
git init -q
git add .engram/
git commit -q -m "Initial task setup"
echo "Committed .engram/ to git"

# Make more changes
engram create "Fourth task" --priority 2
echo

git add .engram/
git commit -q -m "Add fourth task"

echo "Git log:"
git log --oneline
echo

echo "=== Example Complete ==="
echo
echo "Key architecture points:"
echo "  - .engram/ directory stores all data"
echo "  - items.jsonl = append-only source of truth"
echo "  - edges.jsonl = relationships between items"
echo "  - engram.db = SQLite cache for fast queries"
echo "  - Add .engram/ to git for version control"
