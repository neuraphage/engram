#!/usr/bin/env bash
# 06-workflow-status.sh - Status workflow transitions
#
# Demonstrates the item lifecycle: Open -> InProgress -> Closed
#
# Usage: ./06-workflow-status.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Status Workflow ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

# Create a task
echo "--- Create Task ---"
engram create "Implement feature X" --priority 2 --labels "feature"
TASK_ID=$(engram list | head -1 | awk '{print $2}')
echo

# Show initial status
echo "--- Initial Status ---"
engram get "$TASK_ID"
echo

# Start working on it
echo "--- Start Task (Open -> InProgress) ---"
engram start "$TASK_ID"
echo

echo "--- Check Status ---"
engram get "$TASK_ID"
echo

# List by status
echo "--- List by Status ---"
echo "Open items:"
engram list --status open
echo

echo "In Progress items:"
engram list --status in_progress
echo

# Close the task
echo "--- Close Task (InProgress -> Closed) ---"
engram close "$TASK_ID" --reason "Feature implemented and tested"
echo

echo "--- Final Status ---"
engram get "$TASK_ID"
echo

echo "--- Closed Items ---"
engram list --status closed
echo

echo "=== Status Flow Diagram ==="
echo
echo "     +--------+     +------------+     +--------+"
echo "     |  Open  | --> | InProgress | --> | Closed |"
echo "     +--------+     +------------+     +--------+"
echo "          |              |"
echo "          v              v"
echo "     +---------+    +---------+"
echo "     | Blocked |    | Blocked |"
echo "     +---------+    +---------+"
echo
echo "Valid transitions:"
echo "  Open -> InProgress, Blocked, Closed"
echo "  InProgress -> Open, Blocked, Closed"
echo "  Blocked -> Open, InProgress, Closed"
echo "  Closed -> Open (reopen)"
