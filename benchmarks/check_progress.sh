#!/bin/bash
# Quick progress checker for accuracy tests

LOG_FILE="outputs/accuracy/full_test.log"

if [ ! -f "$LOG_FILE" ]; then
    echo "Test not started yet..."
    exit 1
fi

echo "===== ACCURACY TEST PROGRESS ====="
echo ""

# Count completed tests
TESTING_JSON=$(grep -c "Testing json with" "$LOG_FILE" 2>/dev/null || echo 0)
TESTING_TAUQ=$(grep -c "Testing tauq with" "$LOG_FILE" 2>/dev/null || echo 0)
TESTING_NO_SCHEMAS=$(grep -c "Testing tauq-no-schemas with" "$LOG_FILE" 2>/dev/null || echo 0)
TESTING_TOON=$(grep -c "Testing toon with" "$LOG_FILE" 2>/dev/null || echo 0)

echo "Progress:"
echo "  JSON:            $TESTING_JSON/300"
echo "  Tauq:            $TESTING_TAUQ/300"
echo "  Tauq-no-schemas: $TESTING_NO_SCHEMAS/300"
echo "  TOON:            $TESTING_TOON/300"
echo ""

# Check for errors
ERRORS=$(grep -c "Error" "$LOG_FILE" 2>/dev/null || echo 0)
echo "Errors: $ERRORS"
echo ""

# Show last few lines
echo "Last 10 lines:"
tail -10 "$LOG_FILE"
