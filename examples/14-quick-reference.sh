#!/usr/bin/env bash
# 14-quick-reference.sh - Command quick reference
#
# A quick overview of all engram commands with examples.
#
# Usage: ./14-quick-reference.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=========================================="
echo "  Engram Command Quick Reference"
echo "=========================================="
echo

engram init

# CRUD Operations
echo "=== CRUD Operations ==="
echo

echo "# Create a task"
echo '$ engram create "Task title" --priority 2 --labels "bug,urgent"'
engram create "Example task" --priority 2 --labels "demo"
echo

echo "# Create with description"
echo '$ engram create "Task" --description "Detailed description here"'
engram create "Detailed task" --description "This is a longer description"
echo

echo "# List all tasks"
echo '$ engram list'
engram list
echo

echo "# List by status"
echo '$ engram list --status open'
engram list --status open
echo

echo "# Get task details"
TASK_ID=$(engram list | head -1 | awk '{print $2}')
echo "$ engram get $TASK_ID"
engram get "$TASK_ID"
echo

echo "# Close a task"
echo "$ engram close $TASK_ID --reason \"Done\""
engram close "$TASK_ID" --reason "Done"
echo

# Status Transitions
echo "=== Status Transitions ==="
echo

engram create "Status demo" --priority 2
STATUS_ID=$(engram list --status open | head -1 | awk '{print $2}')

echo "# Start working (Open -> InProgress)"
echo "$ engram start $STATUS_ID"
engram start "$STATUS_ID"
echo

echo "# Close task (InProgress -> Closed)"
echo "$ engram close $STATUS_ID --reason \"Completed\""
engram close "$STATUS_ID" --reason "Completed"
echo

# Dependencies
echo "=== Dependencies ==="
echo

engram create "Blocker task" --priority 1
engram create "Blocked task" --priority 2
BLOCKER=$(engram list --status open | grep "Blocker" | awk '{print $2}')
BLOCKED=$(engram list --status open | grep "Blocked" | awk '{print $2}')

echo "# Block one task on another"
echo "$ engram block $BLOCKED $BLOCKER"
engram block "$BLOCKED" "$BLOCKER"
echo

echo "# View ready work (unblocked)"
echo '$ engram ready'
engram ready
echo

echo "# View blocked work"
echo '$ engram blocked'
engram blocked
echo

# Hierarchy
echo "=== Hierarchy ==="
echo

engram create "Parent task" --priority 1
engram create "Child task" --priority 2
PARENT=$(engram list --status open | grep "Parent" | awk '{print $2}')
CHILD=$(engram list --status open | grep "Child" | awk '{print $2}')

echo "# Set parent-child relationship"
echo "$ engram child $CHILD $PARENT"
engram child "$CHILD" "$PARENT"
echo

# Summary
echo "=========================================="
echo "  Command Summary"
echo "=========================================="
echo
echo "Initialization:"
echo "  engram init              Initialize new engram store"
echo
echo "CRUD:"
echo "  engram create TITLE      Create new task"
echo "  engram list              List all tasks"
echo "  engram get ID            Get task details"
echo "  engram close ID          Close a task"
echo
echo "Status:"
echo "  engram start ID          Mark task in progress"
echo "  engram list --status X   Filter by status (open|in_progress|closed)"
echo
echo "Dependencies:"
echo "  engram block A B         A is blocked by B"
echo "  engram ready             Show unblocked tasks"
echo "  engram blocked           Show blocked tasks"
echo
echo "Hierarchy:"
echo "  engram child C P         C is child of P"
echo
echo "Options:"
echo "  --priority N             Priority 0-4 (0=critical)"
echo "  --labels L1,L2           Comma-separated labels"
echo "  --description TEXT       Task description"
echo "  --reason TEXT            Reason for closing"
echo
echo "=========================================="
