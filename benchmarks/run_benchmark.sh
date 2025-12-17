#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  Tauq Comprehensive Benchmark Suite                           ║"
echo "║  Token Efficiency: Tauq vs TOON vs JSON                       ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo

echo "=== Preparing tauq source for Docker build ==="
rm -rf tauq_src
mkdir -p tauq_src

# Copy only necessary source files (not target, etc.)
cp -r ../src tauq_src/
cp -r ../benches tauq_src/ 2>/dev/null || true
cp -r ../Cargo.toml tauq_src/
cp -r ../Cargo.lock tauq_src/ 2>/dev/null || true

echo "=== Building Docker image ==="
docker build -t tauq-benchmark .

echo
echo "=== Running Token Efficiency Benchmark ==="
mkdir -p outputs
docker run --rm -v "$(pwd)/outputs:/app/outputs" tauq-benchmark

echo
echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  Benchmark Complete!                                           ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo
echo "Results saved to:"
echo "  • outputs/benchmark_results.json - Complete results"
echo "  • outputs/*.tqn                  - Tauq formatted outputs"
echo "  • outputs/*.toon                 - TOON formatted outputs"
echo "  • outputs/*.json                 - JSON baselines"
echo
echo "To run Rust performance benchmarks (Criterion.rs):"
echo "  cargo bench --manifest-path=../Cargo.toml"
echo "  open ../target/criterion/report/index.html"
echo
