#!/usr/bin/env bash
# 04-dependencies.sh - Managing task dependencies
#
# Demonstrates blocking relationships between items.
#
# Usage: ./04-dependencies.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Task Dependencies ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

# Create a project with dependencies
echo "--- Create Project Tasks ---"
engram create "Design auth system" --priority 1 --labels "design"
engram create "Implement login API" --priority 1 --labels "backend"
engram create "Implement logout API" --priority 2 --labels "backend"
engram create "Create login UI" --priority 2 --labels "frontend"
engram create "Write integration tests" --priority 2 --labels "testing"
echo

# Get the IDs
DESIGN_ID=$(engram list | grep "Design auth" | awk '{print $2}')
LOGIN_API_ID=$(engram list | grep "login API" | awk '{print $2}')
LOGOUT_API_ID=$(engram list | grep "logout API" | awk '{print $2}')
LOGIN_UI_ID=$(engram list | grep "login UI" | awk '{print $2}')
TESTS_ID=$(engram list | grep "integration tests" | awk '{print $2}')

echo "--- Set Up Dependencies ---"
echo "Design must be done before implementation..."
engram block "$LOGIN_API_ID" "$DESIGN_ID"
engram block "$LOGOUT_API_ID" "$DESIGN_ID"
engram block "$LOGIN_UI_ID" "$DESIGN_ID"

echo "Login UI needs login API..."
engram block "$LOGIN_UI_ID" "$LOGIN_API_ID"

echo "Tests need all implementations..."
engram block "$TESTS_ID" "$LOGIN_API_ID"
engram block "$TESTS_ID" "$LOGOUT_API_ID"
engram block "$TESTS_ID" "$LOGIN_UI_ID"
echo

# Show what's ready
echo "--- Ready Work ---"
engram ready
echo

# Show what's blocked
echo "--- Blocked Items ---"
engram blocked
echo

# Complete the design
echo "--- Complete Design ---"
engram close "$DESIGN_ID" --reason "Design approved"
echo

echo "--- Ready Work (after design) ---"
engram ready
echo

echo "--- Blocked Items (after design) ---"
engram blocked
echo

# Complete backend
echo "--- Complete Backend ---"
engram close "$LOGIN_API_ID" --reason "Login API done"
engram close "$LOGOUT_API_ID" --reason "Logout API done"
echo

echo "--- Ready Work (after backend) ---"
engram ready
echo

# Complete frontend
echo "--- Complete Frontend ---"
engram close "$LOGIN_UI_ID" --reason "Login UI shipped"
echo

echo "--- Ready Work (after frontend) ---"
engram ready
echo

echo "=== Example Complete ==="
echo
echo "Dependency flow:"
echo "  Design -> Login API, Logout API, Login UI"
echo "  Login API -> Login UI"
echo "  Login API, Logout API, Login UI -> Tests"
