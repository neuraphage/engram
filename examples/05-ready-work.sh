#!/usr/bin/env bash
# 05-ready-work.sh - Finding ready work
#
# Demonstrates the 'ready' command for finding actionable items.
#
# Usage: ./05-ready-work.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Finding Ready Work ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

# Create a mix of tasks
echo "--- Create Tasks ---"
engram create "Task A - Independent P0" --priority 0
engram create "Task B - Independent P2" --priority 2
engram create "Task C - Will be blocked" --priority 1
engram create "Task D - Will have multiple blockers" --priority 1
echo

# Get IDs
ID_A=$(engram list | grep "Task A" | awk '{print $2}')
ID_B=$(engram list | grep "Task B" | awk '{print $2}')
ID_C=$(engram list | grep "Task C" | awk '{print $2}')
ID_D=$(engram list | grep "Task D" | awk '{print $2}')

# Set up dependencies
echo "--- Set Up Dependencies ---"
engram block "$ID_C" "$ID_A"  # C blocked by A
engram block "$ID_D" "$ID_B"  # D blocked by B
engram block "$ID_D" "$ID_C"  # D also blocked by C
echo

# Check ready work
echo "--- Ready Work (initial) ---"
echo "Only independent, unblocked items are ready:"
engram ready
echo

echo "--- Blocked Items ---"
engram blocked
echo

# Complete Task A
echo "--- Complete Task A ---"
engram close "$ID_A" --reason "Done"
echo

echo "--- Ready Work (after A) ---"
echo "Task C is now unblocked (was only blocked by A):"
engram ready
echo

echo "--- Blocked Items (after A) ---"
echo "Task D is still blocked (needs both B and C):"
engram blocked
echo

# Complete Task B
echo "--- Complete Task B ---"
engram close "$ID_B" --reason "Done"
echo

echo "--- Ready Work (after B) ---"
echo "Task D is still blocked (needs C):"
engram ready
echo

# Complete Task C
echo "--- Complete Task C ---"
engram close "$ID_C" --reason "Done"
echo

echo "--- Ready Work (after C) ---"
echo "Task D is now ready (all blockers done):"
engram ready
echo

echo "=== Example Complete ==="
echo
echo "Key points:"
echo "  - 'engram ready' shows items with no open blockers"
echo "  - Items are sorted by priority (P0 first)"
echo "  - An item needs ALL blockers closed to become ready"
