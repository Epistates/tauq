#!/bin/bash
set -e

cd "$(dirname "$0")"

echo "=== Preparing tauq source for Docker build ==="
rm -rf tauq_src
mkdir -p tauq_src

# Copy only necessary source files (not target, etc.)
cp -r ../src tauq_src/
cp -r ../Cargo.toml tauq_src/
cp -r ../Cargo.lock tauq_src/ 2>/dev/null || true

echo "=== Building Docker image ==="
docker build -t tauq-benchmark .

echo "=== Running benchmark ==="
mkdir -p outputs
docker run --rm -v "$(pwd)/outputs:/app/outputs" tauq-benchmark

echo "=== Benchmark complete! ==="
echo "Results saved to outputs/"
