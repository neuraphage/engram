#!/usr/bin/env bash
# 01-basic-crud.sh - Basic CRUD operations with engram CLI
#
# Demonstrates creating, listing, getting, and closing items.
#
# Usage: ./01-basic-crud.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Basic CRUD Operations ==="
echo "Working directory: $EXAMPLE_DIR"
echo

# Initialize a new engram store
echo "--- Initialize Store ---"
engram init
echo

# Create items
echo "--- Create Items ---"
engram create "Implement user login" --priority 1 --labels "backend,auth"
engram create "Add logout button" --priority 2 --labels "frontend,auth"
engram create "Write documentation" --priority 3 --description "Document the auth API"
echo

# List all items
echo "--- List All Items ---"
engram list
echo

# List only open items
echo "--- List Open Items ---"
engram list --status open
echo

# Get a specific item (we'll get the first one created)
echo "--- Get Item Details ---"
ITEM_ID=$(engram list 2>/dev/null | head -1 | awk '{print $2}')
if [ -n "$ITEM_ID" ]; then
    engram get "$ITEM_ID"
    echo

    # Close the item
    echo "--- Close Item ---"
    engram close "$ITEM_ID" --reason "Implemented with OAuth2"
    echo

    # Show item after closing
    echo "--- Item After Closing ---"
    engram get "$ITEM_ID"
fi
echo

# List closed items
echo "--- List Closed Items ---"
engram list --status closed
echo

echo "=== Example Complete ==="
echo "Store location: $EXAMPLE_DIR/.engram"
