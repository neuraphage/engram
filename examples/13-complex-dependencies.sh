#!/usr/bin/env bash
# 13-complex-dependencies.sh - Complex dependency graphs
#
# Demonstrates managing complex task dependencies including
# diamond dependencies and multi-level blocking.
#
# Usage: ./13-complex-dependencies.sh

set -e

EXAMPLE_DIR="${EXAMPLE_DIR:-$(mktemp -d)}"
cd "$EXAMPLE_DIR"

echo "=== Complex Dependencies ==="
echo "Working directory: $EXAMPLE_DIR"
echo

engram init

# Create a diamond dependency pattern
#
#        A (Design)
#       / \
#      B   C (Implement)
#       \ /
#        D (Test)

echo "--- Create Diamond Dependency ---"
echo
echo "Pattern:"
echo "        A (Design)"
echo "       / \\"
echo "      B   C"
echo "       \\ /"
echo "        D (Test)"
echo

engram create "A: Design API" --priority 1 --labels "design"
engram create "B: Implement backend" --priority 2 --labels "backend"
engram create "C: Implement frontend" --priority 2 --labels "frontend"
engram create "D: Integration test" --priority 2 --labels "testing"

A=$(engram list | grep "^open.*A: Design" | awk '{print $2}')
B=$(engram list | grep "^open.*B: Implement" | awk '{print $2}')
C=$(engram list | grep "^open.*C: Implement" | awk '{print $2}')
D=$(engram list | grep "^open.*D: Integration" | awk '{print $2}')

# Set up diamond
engram block "$B" "$A"  # B blocked by A
engram block "$C" "$A"  # C blocked by A
engram block "$D" "$B"  # D blocked by B
engram block "$D" "$C"  # D blocked by C
echo

echo "--- Initial State ---"
echo "Ready:"
engram ready
echo
echo "Blocked:"
engram blocked
echo

# Complete A
echo "--- Complete A (Design) ---"
engram close "$A" --reason "Design approved"
echo
echo "Ready (B and C should now be ready):"
engram ready
echo

# Complete B only
echo "--- Complete B (Backend) ---"
engram close "$B" --reason "Backend done"
echo
echo "Ready (C still ready, D still blocked):"
engram ready
echo
echo "Blocked (D still needs C):"
engram blocked
echo

# Complete C
echo "--- Complete C (Frontend) ---"
engram close "$C" --reason "Frontend done"
echo
echo "Ready (D should now be ready):"
engram ready
echo

# Multi-level dependency chain
echo "=========================================="
echo "--- Multi-Level Chain ---"
echo
echo "Pattern: E -> F -> G -> H -> I"
echo

engram create "E: Research" --priority 1 --labels "research"
engram create "F: Prototype" --priority 2 --labels "prototype"
engram create "G: Implement" --priority 2 --labels "implement"
engram create "H: Review" --priority 2 --labels "review"
engram create "I: Deploy" --priority 2 --labels "deploy"

E=$(engram list | grep "^open.*E: Research" | awk '{print $2}')
F=$(engram list | grep "^open.*F: Prototype" | awk '{print $2}')
G=$(engram list | grep "^open.*G: Implement" | awk '{print $2}')
H=$(engram list | grep "^open.*H: Review" | awk '{print $2}')
I=$(engram list | grep "^open.*I: Deploy" | awk '{print $2}')

engram block "$F" "$E"
engram block "$G" "$F"
engram block "$H" "$G"
engram block "$I" "$H"
echo

echo "--- Chain State ---"
echo "Only E is ready (start of chain):"
engram ready | grep -E "(Research|Prototype|Implement|Review|Deploy)"
echo
echo "Rest are blocked:"
engram blocked | grep -E "(Research|Prototype|Implement|Review|Deploy)"
echo

# Complete through the chain
echo "--- Progress Through Chain ---"
for task in "$E" "$F" "$G" "$H"; do
    engram close "$task" --reason "Done"
    echo "After closing, ready work:"
    engram ready | grep -E "(Research|Prototype|Implement|Review|Deploy)" || echo "  (none in chain)"
done
echo

echo "=== Example Complete ==="
echo
echo "Key points:"
echo "  - Diamond: D needs both B and C"
echo "  - Chain: Each step unlocks the next"
echo "  - 'engram blocked' shows what's waiting"
echo "  - 'engram ready' shows what can be done now"
