#!/bin/bash
# Check progress of accuracy benchmark

echo "=========================================="
echo "ACCURACY BENCHMARK PROGRESS"
echo "=========================================="
echo ""

# Check if process is running
PID=$(ps aux | grep "accuracy_benchmark" | grep -v grep | awk '{print $2}')
if [ -n "$PID" ]; then
    echo "✓ Benchmark process is running (PID: $PID)"

    # Show CPU and memory usage
    ps -p $PID -o %cpu,%mem,etime,command | tail -1
    echo ""
else
    echo "✗ No benchmark process found"
    echo ""
fi

# Check results file
if [ -f "outputs/accuracy/results.json" ]; then
    echo "Results file exists:"
    echo "  Size: $(ls -lh outputs/accuracy/results.json | awk '{print $5}')"
    echo "  Last modified: $(ls -l outputs/accuracy/results.json | awk '{print $6, $7, $8}')"
    echo ""

    echo "Current results:"
    cat outputs/accuracy/results.json | python3 -m json.tool 2>/dev/null | head -40
else
    echo "No results file yet"
fi

echo ""
echo "=========================================="
