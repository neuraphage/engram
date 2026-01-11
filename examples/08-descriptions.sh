#!/usr/bin/env bash
# 08-descriptions.sh - Working with descriptions
#
# Demonstrates using descriptions for detailed task context.
#
# Usage: ./08-descriptions.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Task Descriptions ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

# Create task with inline description
echo "--- Create Task with Description ---"
engram create "Fix login timeout bug" \
    --priority 1 \
    --labels "bug,auth" \
    --description "Users report being logged out after 5 minutes of inactivity. Expected behavior is 30 minutes. Check session cookie expiration settings."
echo

# Get the ID
BUG_ID=$(engram list | grep "timeout" | awk '{print $2}')

# View the task with description
echo "--- View Task Details ---"
engram get "$BUG_ID"
echo

# Create more tasks with descriptions
echo "--- More Tasks with Descriptions ---"
engram create "Add rate limiting" \
    --priority 2 \
    --labels "security,api" \
    --description "Implement rate limiting on API endpoints:
- /api/auth/* : 5 requests/minute
- /api/data/* : 100 requests/minute
- /api/admin/* : 20 requests/minute

Use Redis for distributed rate limiting across instances."
echo

engram create "Database migration" \
    --priority 1 \
    --labels "database,migration" \
    --description "Add new columns to users table:
1. last_login_at (timestamp)
2. login_count (integer)
3. preferences (jsonb)

Run during maintenance window. Estimated time: 15 minutes for 1M rows."
echo

# List all tasks
echo "--- All Tasks ---"
engram list
echo

# View each task (grep for status prefix to avoid matching description text)
RATE_ID=$(engram list | grep "^open.*rate limiting" | awk '{print $2}')
DB_ID=$(engram list | grep "^open.*migration" | awk '{print $2}')

echo "--- Rate Limiting Details ---"
engram get "$RATE_ID"
echo

echo "--- Migration Details ---"
engram get "$DB_ID"
echo

echo "=== Example Complete ==="
echo
echo "Tips for good descriptions:"
echo "  - Include reproduction steps for bugs"
echo "  - List acceptance criteria for features"
echo "  - Add context that helps future you"
echo "  - Use multiline for complex tasks"
