#!/usr/bin/env bash
# 03-labels.sh - Using labels for organization
#
# Demonstrates tagging items with labels.
#
# Usage: ./03-labels.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Labels for Organization ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

# Create items with various labels
echo "--- Create Items with Labels ---"
engram create "Setup CI/CD" --priority 1 --labels "devops,infrastructure"
engram create "Add unit tests" --priority 2 --labels "testing,backend"
engram create "Add integration tests" --priority 2 --labels "testing,backend,integration"
engram create "Setup monitoring" --priority 2 --labels "devops,observability"
engram create "Implement login API" --priority 1 --labels "backend,auth"
engram create "Create login form" --priority 2 --labels "frontend,auth"
engram create "Add logout button" --priority 3 --labels "frontend,auth"
engram create "Document API" --priority 3 --labels "docs,backend"
echo

# List all items
echo "--- All Items ---"
engram list
echo

# Filter by label using grep (engram list shows labels in brackets)
echo "--- Backend Items (grep filter) ---"
engram list | grep -E "\[.*backend.*\]" || echo "No backend items"
echo

echo "--- Testing Items (grep filter) ---"
engram list | grep -E "\[.*testing.*\]" || echo "No testing items"
echo

echo "--- Auth Items (grep filter) ---"
engram list | grep -E "\[.*auth.*\]" || echo "No auth items"
echo

echo "--- DevOps Items (grep filter) ---"
engram list | grep -E "\[.*devops.*\]" || echo "No devops items"
echo

echo "=== Example Complete ==="
echo
echo "Tip: Labels help organize work by:"
echo "  - Component: backend, frontend, devops"
echo "  - Type: feature, bug, testing, docs"
echo "  - Domain: auth, payments, users"
