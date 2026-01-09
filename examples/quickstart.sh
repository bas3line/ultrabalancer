#!/bin/bash
# UltraBalancer Quick Start Demo

set -e

echo "╔═══════════════════════════════════════════╗"
echo "║   UltraBalancer Quick Start Demo         ║"
echo "╚═══════════════════════════════════════════╝"
echo ""

# Check if ultrabalancer is built
if [ ! -f "../target/release/ultrabalancer" ] && [ ! -f "../target/debug/ultrabalancer" ]; then
    echo "Building ultrabalancer..."
    cd ..
    cargo build --release
    cd examples
fi

BINARY="../target/release/ultrabalancer"
if [ ! -f "$BINARY" ]; then
    BINARY="../target/debug/ultrabalancer"
fi

# Start test backends
echo "Starting backend servers..."
python3 test-backend.py 8001 &
PID1=$!
python3 test-backend.py 8002 &
PID2=$!
python3 test-backend.py 8003 &
PID3=$!

sleep 2
echo "✓ Backend servers started on ports 8001, 8002, 8003"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo "Shutting down..."
    kill $PID1 $PID2 $PID3 2>/dev/null || true
    exit 0
}

trap cleanup INT TERM

echo "Starting UltraBalancer on port 8080..."
echo "Algorithm: round-robin"
echo ""
echo "Test with: curl http://localhost:8080"
echo "Metrics:   curl http://localhost:8080/metrics"
echo "Health:    curl http://localhost:8080/health"
echo ""
echo "Press Ctrl+C to stop"
echo ""

$BINARY start round-robin 127.0.0.1:8001 127.0.0.1:8002 127.0.0.1:8003 -p 8080
