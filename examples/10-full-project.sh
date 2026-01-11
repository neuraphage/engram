#!/usr/bin/env bash
# 10-full-project.sh - Complete project workflow
#
# A realistic example of managing a small project from start to finish.
#
# Usage: ./10-full-project.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=========================================="
echo "  Project: Build a REST API"
echo "=========================================="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

# Phase 1: Planning
echo "=== Phase 1: Planning ==="
echo

engram create "Epic: REST API MVP" --priority 0 --labels "epic"
EPIC=$(engram list | grep "Epic" | awk '{print $2}')

engram create "Write API specification" --priority 1 --labels "planning,docs"
engram create "Design database schema" --priority 1 --labels "planning,database"
engram create "Choose tech stack" --priority 1 --labels "planning"

SPEC=$(engram list | grep "specification" | awk '{print $2}')
SCHEMA=$(engram list | grep "schema" | awk '{print $2}')
STACK=$(engram list | grep "tech stack" | awk '{print $2}')

# Link to epic
engram child "$SPEC" "$EPIC"
engram child "$SCHEMA" "$EPIC"
engram child "$STACK" "$EPIC"

echo "--- Planning Tasks ---"
engram list
echo

# Phase 2: Setup
echo "=== Phase 2: Setup ==="
echo

engram create "Initialize project structure" --priority 1 --labels "setup"
engram create "Configure CI/CD pipeline" --priority 2 --labels "setup,devops"
engram create "Set up development database" --priority 1 --labels "setup,database"

INIT=$(engram list | grep "Initialize" | awk '{print $2}')
CI=$(engram list | grep "CI/CD" | awk '{print $2}')
DEVDB=$(engram list | grep "development database" | awk '{print $2}')

engram child "$INIT" "$EPIC"
engram child "$CI" "$EPIC"
engram child "$DEVDB" "$EPIC"

# Setup depends on planning
engram block "$INIT" "$STACK"
engram block "$DEVDB" "$SCHEMA"

# Phase 3: Implementation
echo "=== Phase 3: Implementation ==="
echo

engram create "Implement user endpoints" --priority 2 --labels "backend,api"
engram create "Implement auth endpoints" --priority 1 --labels "backend,api,security"
engram create "Implement data endpoints" --priority 2 --labels "backend,api"
engram create "Add request validation" --priority 2 --labels "backend"
engram create "Add error handling" --priority 2 --labels "backend"

USERS=$(engram list | grep "user endpoints" | awk '{print $2}')
AUTH=$(engram list | grep "auth endpoints" | awk '{print $2}')
DATA=$(engram list | grep "data endpoints" | awk '{print $2}')
VALID=$(engram list | grep "validation" | awk '{print $2}')
ERRORS=$(engram list | grep "error handling" | awk '{print $2}')

engram child "$USERS" "$EPIC"
engram child "$AUTH" "$EPIC"
engram child "$DATA" "$EPIC"
engram child "$VALID" "$EPIC"
engram child "$ERRORS" "$EPIC"

# Implementation depends on setup
engram block "$USERS" "$INIT"
engram block "$AUTH" "$INIT"
engram block "$DATA" "$INIT"
engram block "$DATA" "$DEVDB"

# Phase 4: Testing & Docs
echo "=== Phase 4: Testing & Documentation ==="
echo

engram create "Write unit tests" --priority 2 --labels "testing"
engram create "Write integration tests" --priority 2 --labels "testing"
engram create "Write API documentation" --priority 3 --labels "docs"

UNIT=$(engram list | grep "unit tests" | awk '{print $2}')
INTEG=$(engram list | grep "integration" | awk '{print $2}')
DOCS=$(engram list | grep "API documentation" | awk '{print $2}')

engram child "$UNIT" "$EPIC"
engram child "$INTEG" "$EPIC"
engram child "$DOCS" "$EPIC"

# Testing depends on implementation
engram block "$UNIT" "$USERS"
engram block "$UNIT" "$AUTH"
engram block "$INTEG" "$UNIT"
engram block "$DOCS" "$SPEC"

echo "--- All Tasks Created ---"
engram list
echo

echo "--- What's Ready to Work On? ---"
engram ready
echo

echo "--- What's Blocked? ---"
engram blocked
echo

# Simulate working through the project
echo "=========================================="
echo "  Simulating Project Progress"
echo "=========================================="
echo

echo "--- Complete Planning ---"
engram close "$SPEC" --reason "OpenAPI spec written"
engram close "$SCHEMA" --reason "ERD complete"
engram close "$STACK" --reason "Chose Rust + Axum + PostgreSQL"
echo

echo "--- Ready After Planning ---"
engram ready
echo

echo "--- Complete Setup ---"
engram close "$INIT" --reason "Cargo project initialized"
engram close "$DEVDB" --reason "Docker Compose configured"
engram start "$CI"  # Mark as in progress
echo

echo "--- Ready After Setup ---"
engram ready
echo

echo "--- Start Implementation ---"
engram start "$AUTH"
engram start "$USERS"
echo

echo "--- In Progress Items ---"
engram list --status in_progress
echo

echo "--- Complete Auth ---"
engram close "$AUTH" --reason "JWT auth working"
echo

echo "--- Complete Users ---"
engram close "$USERS" --reason "CRUD endpoints done"
echo

echo "--- Ready Now ---"
engram ready
echo

echo "--- Final Status ---"
echo "Open:"
engram list --status open
echo
echo "In Progress:"
engram list --status in_progress
echo
echo "Closed:"
engram list --status closed
echo

echo "=========================================="
echo "  Project Progress Summary"
echo "=========================================="
TOTAL=$(engram list 2>/dev/null | wc -l)
CLOSED=$(engram list --status closed 2>/dev/null | wc -l)
echo "Tasks complete: $CLOSED / $TOTAL"
echo "=========================================="
