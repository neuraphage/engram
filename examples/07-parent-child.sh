#!/usr/bin/env bash
# 07-parent-child.sh - Parent-child relationships
#
# Demonstrates hierarchical task organization with parent-child links.
#
# Usage: ./07-parent-child.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Parent-Child Relationships ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

# Create parent tasks (epics)
echo "--- Create Parent Tasks (Epics) ---"
engram create "Epic: User Authentication" --priority 1 --labels "epic"
engram create "Epic: Payment System" --priority 1 --labels "epic"
echo

# Get parent IDs
AUTH_EPIC=$(engram list | grep "User Authentication" | awk '{print $2}')
PAY_EPIC=$(engram list | grep "Payment System" | awk '{print $2}')

# Create child tasks for Auth epic
echo "--- Create Auth Subtasks ---"
engram create "Design auth flow" --priority 1 --labels "design"
engram create "Implement JWT tokens" --priority 2 --labels "backend"
engram create "Create login page" --priority 2 --labels "frontend"
engram create "Add password reset" --priority 3 --labels "feature"
echo

# Get auth subtask IDs
AUTH_DESIGN=$(engram list | grep "Design auth" | awk '{print $2}')
AUTH_JWT=$(engram list | grep "JWT tokens" | awk '{print $2}')
AUTH_LOGIN=$(engram list | grep "login page" | awk '{print $2}')
AUTH_RESET=$(engram list | grep "password reset" | awk '{print $2}')

# Create child tasks for Payment epic
echo "--- Create Payment Subtasks ---"
engram create "Design payment flow" --priority 1 --labels "design"
engram create "Integrate Stripe" --priority 2 --labels "backend"
engram create "Create checkout UI" --priority 2 --labels "frontend"
echo

# Get payment subtask IDs
PAY_DESIGN=$(engram list | grep "Design payment" | awk '{print $2}')
PAY_STRIPE=$(engram list | grep "Stripe" | awk '{print $2}')
PAY_CHECKOUT=$(engram list | grep "checkout UI" | awk '{print $2}')

# Set up parent-child relationships
echo "--- Link Children to Parents ---"
echo "Auth epic children:"
engram child "$AUTH_DESIGN" "$AUTH_EPIC"
engram child "$AUTH_JWT" "$AUTH_EPIC"
engram child "$AUTH_LOGIN" "$AUTH_EPIC"
engram child "$AUTH_RESET" "$AUTH_EPIC"

echo "Payment epic children:"
engram child "$PAY_DESIGN" "$PAY_EPIC"
engram child "$PAY_STRIPE" "$PAY_EPIC"
engram child "$PAY_CHECKOUT" "$PAY_EPIC"
echo

# View the hierarchy
echo "--- View Auth Epic ---"
engram get "$AUTH_EPIC"
echo

echo "--- View Payment Epic ---"
engram get "$PAY_EPIC"
echo

# Complete some subtasks
echo "--- Complete Auth Design ---"
engram close "$AUTH_DESIGN" --reason "Design approved"
echo

echo "--- View Updated Auth Epic ---"
engram get "$AUTH_EPIC"
echo

echo "=== Example Complete ==="
echo
echo "Key points:"
echo "  - 'engram child <child> <parent>' creates parent-child link"
echo "  - Use epics to group related tasks"
echo "  - View epic details to see child progress"
