#!/usr/bin/env bash
# 09-daemon-mode.sh - Daemon mode for concurrent access
#
# Demonstrates running engram in daemon mode for better performance
# and concurrent access from multiple processes.
#
# Usage: ./09-daemon-mode.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Daemon Mode ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

echo "--- Without Daemon (each command opens/closes store) ---"
echo "Creating tasks sequentially..."
time (
    engram create "Task 1" --priority 2
    engram create "Task 2" --priority 2
    engram create "Task 3" --priority 2
    engram list > /dev/null
)
echo

# Clean up for daemon test
rm -rf .engram
engram init

echo "--- Check Daemon Status (before starting) ---"
engram daemon-status || true
echo

echo "--- Start Daemon ---"
engram daemon &
DAEMON_PID=$!
sleep 1  # Give daemon time to start

echo "--- Check Daemon Status (after starting) ---"
engram daemon-status || true
echo

echo "--- With Daemon (persistent connection) ---"
echo "Creating tasks with daemon running..."
time (
    engram create "Task A" --priority 2
    engram create "Task B" --priority 2
    engram create "Task C" --priority 2
    engram list > /dev/null
)
echo

echo "--- Concurrent Access Demo ---"
echo "Multiple processes accessing simultaneously..."

# Run multiple operations in parallel
(
    engram create "Parallel 1" --priority 1 &
    engram create "Parallel 2" --priority 2 &
    engram create "Parallel 3" --priority 3 &
    wait
)
echo

echo "--- Final Task List ---"
engram list
echo

# Clean up daemon using proper command
echo "--- Stopping Daemon ---"
engram daemon-stop || kill $DAEMON_PID 2>/dev/null || true
wait $DAEMON_PID 2>/dev/null || true
echo

echo "--- Check Daemon Status (after stopping) ---"
engram daemon-status || true
echo

echo "=== Example Complete ==="
echo
echo "Key points:"
echo "  - 'engram daemon' starts background service"
echo "  - 'engram daemon-status' checks if daemon is running"
echo "  - 'engram daemon-stop' gracefully stops daemon"
echo "  - Daemon handles concurrent access safely"
echo "  - Better performance for many operations"
