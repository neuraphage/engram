#!/usr/bin/env bash
# 15-blocked-workflow.sh - Working with blocked tasks
#
# Demonstrates how to use the blocked status for tasks
# that are waiting on external factors.
#
# Usage: ./15-blocked-workflow.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Blocked Workflow ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

# Create tasks
echo "--- Create Tasks ---"
engram create "Waiting for API access" --priority 1 --labels "external"
engram create "Waiting for design approval" --priority 2 --labels "design"
engram create "Ready to implement" --priority 2 --labels "backend"
engram create "Waiting for legal review" --priority 2 --labels "compliance"
echo

API=$(engram list | grep "API access" | awk '{print $2}')
DESIGN=$(engram list | grep "design approval" | awk '{print $2}')
IMPL=$(engram list | grep "implement" | awk '{print $2}')
LEGAL=$(engram list | grep "legal review" | awk '{print $2}')

# Set up blocking relationships
echo "--- Set Up Blocks ---"
engram block "$IMPL" "$API"    # Implementation blocked by API access
engram block "$IMPL" "$DESIGN" # Implementation also blocked by design
echo

echo "--- Initial Status ---"
echo "Ready work:"
engram ready
echo
echo "Blocked work:"
engram blocked
echo

# Simulate external resolution
echo "--- API Access Granted ---"
engram close "$API" --reason "Got API credentials"
echo

echo "--- After API Access ---"
echo "Ready work (still blocked by design):"
engram ready
echo
echo "Blocked work:"
engram blocked
echo

echo "--- Design Approved ---"
engram close "$DESIGN" --reason "Design signed off"
echo

echo "--- After Design Approval ---"
echo "Ready work (implementation now unblocked):"
engram ready
echo
echo "Blocked work:"
engram blocked
echo

# Start implementation
echo "--- Start Implementation ---"
engram start "$IMPL"
echo

echo "--- In Progress ---"
engram list --status in_progress
echo

# Complete implementation
echo "--- Complete Implementation ---"
engram close "$IMPL" --reason "Feature shipped"
echo

echo "--- Final Status ---"
echo "Open:"
engram list --status open
echo
echo "Closed:"
engram list --status closed
echo

echo "=== Example Complete ==="
echo
echo "Key points:"
echo "  - Use 'block' for dependency tracking"
echo "  - 'ready' shows actionable work"
echo "  - 'blocked' shows what's waiting"
echo "  - Close blockers to unblock dependent tasks"
