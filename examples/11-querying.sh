#!/usr/bin/env bash
# 11-querying.sh - Querying and filtering tasks
#
# Demonstrates various ways to query and filter your task list.
#
# Usage: ./11-querying.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Querying and Filtering ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

# Create a variety of tasks
echo "--- Create Sample Tasks ---"
engram create "Fix critical security bug" --priority 0 --labels "bug,security,urgent"
engram create "Add user dashboard" --priority 2 --labels "feature,frontend"
engram create "Optimize database queries" --priority 2 --labels "performance,backend"
engram create "Update documentation" --priority 3 --labels "docs"
engram create "Refactor auth module" --priority 2 --labels "tech-debt,backend,security"
engram create "Add email notifications" --priority 2 --labels "feature,backend"
engram create "Fix mobile layout" --priority 1 --labels "bug,frontend"
engram create "Add unit tests for auth" --priority 2 --labels "testing,security"
echo

# Close and progress some tasks
SEC_ID=$(engram list | grep "critical security" | awk '{print $2}')
DASH_ID=$(engram list | grep "dashboard" | awk '{print $2}')
engram close "$SEC_ID" --reason "Patched"
engram start "$DASH_ID"
echo

# Query by status
echo "=== Filter by Status ==="
echo
echo "--- Open Tasks ---"
engram list --status open
echo
echo "--- In Progress Tasks ---"
engram list --status in_progress
echo
echo "--- Closed Tasks ---"
engram list --status closed
echo

# Filter by labels using grep
echo "=== Filter by Label (using grep) ==="
echo
echo "--- Backend Tasks ---"
engram list | grep -E "\[.*backend.*\]" || echo "None"
echo
echo "--- Security Tasks ---"
engram list | grep -E "\[.*security.*\]" || echo "None"
echo
echo "--- Bug Tasks ---"
engram list | grep -E "\[.*bug.*\]" || echo "None"
echo
echo "--- Feature Tasks ---"
engram list | grep -E "\[.*feature.*\]" || echo "None"
echo

# Combine status and label filters
echo "=== Combined Filters ==="
echo
echo "--- Open Backend Tasks ---"
engram list --status open | grep -E "\[.*backend.*\]" || echo "None"
echo
echo "--- Open Bugs ---"
engram list --status open | grep -E "\[.*bug.*\]" || echo "None"
echo

# Filter by priority
echo "=== Filter by Priority (using grep) ==="
echo
echo "--- Critical (P0) ---"
engram list | grep "P0" || echo "None"
echo
echo "--- High Priority (P1) ---"
engram list | grep "P1" || echo "None"
echo
echo "--- Medium Priority (P2) ---"
engram list | grep "P2" || echo "None"
echo

# Ready vs blocked
echo "=== Ready vs Blocked ==="
echo
echo "--- Ready Work ---"
engram ready
echo
echo "--- Blocked Work ---"
engram blocked
echo

echo "=== Example Complete ==="
echo
echo "Query tips:"
echo "  - Use --status to filter by status"
echo "  - Use grep to filter by labels or priority"
echo "  - Combine filters with pipes"
echo "  - Use 'ready' and 'blocked' for dependency-aware views"
