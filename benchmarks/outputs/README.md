# Benchmark Outputs

This directory contains the generated output files from running the benchmark suite.

## Contents

After running benchmarks, this directory will contain:

- `*.json` - Minified JSON versions of test datasets
- `*.tqn` - Standard Tauq format (space-delimited)
- `*.opt.tqn` - Optimized Tauq format (comma-delimited)
- `*.ultra.tqn` - Ultra-compact Tauq format
- `*.toon` - TOON format (via toon-python library)
- `benchmark_results.json` - Complete benchmark results with token counts and comparisons

## Datasets

1. **flat_100** - 100 user records (5 fields)
2. **flat_1000** - 1,000 user records (5 fields)
3. **mixed_structure** - Nested objects with arrays
4. **deeply_nested** - 10 organizations with deep nesting
5. **wide_records** - 100 records with 15 fields each
6. **heterogeneous** - 100 records with varying schemas
7. **timeseries** - 200 timestamp/value pairs
8. **ecommerce** - Product catalog with nested data
9. **api_response** - Paginated API response
10. **config_style** - Realistic application config

## Usage

Run benchmarks to regenerate these files:

```bash
cd ..
docker build -t tauq-benchmark .
docker run --rm tauq-benchmark
```

## Note

These files are excluded from git via `.gitignore` to keep the repository clean. They are regenerated each time benchmarks run.
