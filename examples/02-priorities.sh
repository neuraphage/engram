#!/usr/bin/env bash
# 02-priorities.sh - Working with priorities
#
# Demonstrates priority levels (P0=critical to P4=low).
#
# Usage: ./02-priorities.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Priority Levels ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

# Create items with different priorities
echo "--- Create Items with Priorities ---"
echo "P0 (Critical):"
engram create "Fix production outage" --priority 0 --labels "urgent,production"
echo

echo "P1 (High):"
engram create "Security vulnerability patch" --priority 1 --labels "security"
echo

echo "P2 (Medium - default):"
engram create "Add new feature" --priority 2 --labels "feature"
echo

echo "P3 (Low):"
engram create "Refactor old code" --priority 3 --labels "tech-debt"
echo

echo "P4 (Backlog):"
engram create "Nice-to-have improvement" --priority 4 --labels "backlog"
echo

# List all - should be sorted by priority
echo "--- All Items (sorted by priority) ---"
engram list
echo

# Show ready items - also sorted by priority
echo "--- Ready Work (prioritized) ---"
engram ready
echo

echo "=== Example Complete ==="
