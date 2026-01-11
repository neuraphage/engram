#!/usr/bin/env bash
# 12-git-integration.sh - Understanding git-backed storage
#
# Demonstrates how engram stores data in git and enables
# history, collaboration, and version control of tasks.
#
# Usage: ./12-git-integration.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Git-Backed Storage ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

echo "--- Directory Structure After Init ---"
ls -la .engram/
echo

echo "--- Git Repository ---"
cd .engram
git status
cd ..
echo

# Create some tasks
echo "--- Create Initial Tasks ---"
engram create "First task" --priority 1 --labels "initial"
engram create "Second task" --priority 2 --labels "initial"
echo

echo "--- Check Git Commits ---"
cd .engram
git log --oneline
cd ..
echo

echo "--- View JSONL Source of Truth ---"
echo "The items.jsonl file contains all task data:"
head -5 .engram/items.jsonl 2>/dev/null || cat .engram/items.jsonl
echo

# Make changes
echo "--- Make Changes ---"
FIRST=$(engram list | grep "First" | awk '{print $2}')
engram close "$FIRST" --reason "Completed"
engram create "Third task" --priority 3 --labels "added-later"
echo

echo "--- Git History After Changes ---"
cd .engram
git log --oneline
cd ..
echo

echo "--- JSONL Shows Append-Only History ---"
cat .engram/items.jsonl
echo

echo "--- SQLite Cache for Fast Queries ---"
echo "Cache file: .engram/engram.db"
ls -la .engram/engram.db
echo
echo "The cache is rebuilt from JSONL on startup."
echo "JSONL is the source of truth, SQLite is for performance."
echo

echo "=== Collaboration Workflow ==="
echo
echo "Since tasks are stored in git, you can:"
echo "  1. Push .engram/ to a shared repo"
echo "  2. Multiple team members can sync tasks"
echo "  3. Conflicts are resolved like any git conflict"
echo "  4. Full history of all task changes is preserved"
echo

echo "=== Example Complete ==="
echo
echo "Key architecture points:"
echo "  - .engram/ is a git repository"
echo "  - items.jsonl = append-only source of truth"
echo "  - edges.jsonl = relationships between items"
echo "  - engram.db = SQLite cache for fast queries"
echo "  - Git enables history, branching, collaboration"
