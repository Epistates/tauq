# Tauq Benchmarks

Rigorous token efficiency benchmarks comparing Tauq vs TOON vs JSON using tiktoken cl100k_base (GPT-4/Claude tokenizer).

## Results Summary

- **Tauq vs JSON**: -44.2% (11% more efficient)
- **Tauq vs TOON**: -10.8% (3,758 tokens saved across all datasets)
- **Tauq wins**: 7 of 10 dataset types

## Running Benchmarks

### Using Docker (Recommended)

```bash
docker build -t tauq-benchmark .
docker run --rm tauq-benchmark
```

### Manual Run

Requires Python 3.12+, Rust toolchain, and tauq binary built:

```bash
pip install tiktoken toon-python tabulate
cargo build --release --manifest-path=../Cargo.toml
python3 benchmark.py
```

## Benchmark Datasets

1. **flat_100**: 100 user records (5 fields)
2. **flat_1000**: 1,000 user records (5 fields)
3. **mixed_structure**: Nested objects with arrays
4. **deeply_nested**: 10 organizations with deep nesting
5. **wide_records**: 100 records with 15 fields each
6. **heterogeneous**: 100 records with varying schemas
7. **timeseries**: 200 timestamp/value pairs
8. **ecommerce**: Product catalog with nested data
9. **api_response**: Paginated API response
10. **config_style**: Realistic application config

## Output

Results are saved to `outputs/` directory:
- `*.json` - Minified JSON
- `*.tqn` - Standard Tauq (space-delimited)
- `*.opt.tqn` - Optimized Tauq (comma-delimited)
- `*.ultra.tqn` - Ultra-compact Tauq
- `*.toon` - TOON format
- `benchmark_results.json` - Full results with token counts

## Methodology

- **Tokenizer**: tiktoken cl100k_base (same as GPT-4 and Claude)
- **TOON**: Official toon-python library (spec compliant)
- **Tauq**: v0.1.0 (space-delimited standard mode)
- **Fair comparison**: No artificial handicaps, proper implementations for all formats

## Files

- `benchmark.py` - Main benchmark script
- `Dockerfile` - Docker environment for reproducible benchmarks
- `tauq_src/` - Copy of tauq source for building in Docker
- `outputs/` - Generated benchmark outputs
