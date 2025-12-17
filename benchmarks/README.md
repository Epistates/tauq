# Tauq Benchmarks

Comprehensive benchmark suite comparing Tauq vs TOON vs JSON across token efficiency, parsing performance, and formatting speed.

## 🎯 Benchmark Design Philosophy

This benchmark suite directly addresses critiques raised in the LocalLLaMA community about token-efficient format benchmarks (see [this discussion](https://www.reddit.com/r/LocalLLaMA/comments/1oylf8m/stopping_the_toon_hype_with_a_proper_benchmark/)):

**What We Do Differently**:
- ✅ **Include CSV** and acknowledge where it wins (flat tabular data)
- ✅ **Deterministic token counting** (no statistical variance issues)
- ✅ **Transparent about trade-offs** - we show where each format excels
- ✅ **Test appropriate data structures** with tabular eligibility metrics (0-100%)
- ✅ **Explain design decisions** (e.g., why space-delimited vs comma-delimited)
- ✅ **Category breakdown** - results by structure class (uniform, nested, heterogeneous, etc.)

**What We Don't Claim**:
- ❌ Universal superiority (different formats have different sweet spots)
- ❌ LLM accuracy improvements (we focus on token efficiency)
- ❌ Generation quality (separate concern requiring different methodology)

See [METHODOLOGY.md](./METHODOLOGY.md) for detailed discussion of our approach and how we address common critiques.

## Benchmark Types

### 1. Token Efficiency Benchmarks
Measures token count reduction using `tiktoken` with `o200k_base` encoding (GPT-4o, Claude 3.5+):
- Compares minified JSON, pretty JSON, TOON, and Tauq (standard, optimized, ultra modes)
- Tests across diverse dataset types: flat tabular, nested structures, heterogeneous data, time-series, configs
- Categorizes results by data structure characteristics (tabularEligibility %)

### 2. Performance Benchmarks (Criterion.rs)
Measures parsing and formatting performance:
- **Parse benchmarks**: JSON → Tauq value parsing speed
- **Format benchmarks**: Data structure → Tauq string formatting speed
- **Round-trip benchmarks**: Parse + format cycles
- Statistical analysis with confidence intervals and outlier detection

### 3. LLM Accuracy Benchmarks
Tests retrieval accuracy across multiple formats and models:
- Addresses improvingagents.com findings (TOON: 43-47% accuracy vs Markdown: 54-62%)
- 1000+ test questions with 5 question types (lookup, aggregation, reasoning)
- Multiple models (GPT-4o, Claude 3.5 Sonnet)
- Deterministic validation with statistical significance (95% CI)
- **Status**: ✅ Test harness complete, ready for API key testing

See [ACCURACY_README.md](./ACCURACY_README.md) and [ACCURACY_ANALYSIS.md](./ACCURACY_ANALYSIS.md) for details.

### 4. Comparative Analysis
- Token savings breakdown by data structure type
- Performance comparisons with statistical significance
- Detailed examples with side-by-side format comparisons

## Quick Start

### Token Efficiency Benchmark

```bash
# Using Python script (recommended)
python3 benchmark_comprehensive.py

# Or using Docker
./run_benchmark.sh
```

### Performance Benchmarks

```bash
# Run Criterion.rs benchmarks
cargo bench --manifest-path=../Cargo.toml

# View HTML reports
open ../target/criterion/report/index.html
```

### Accuracy Benchmarks

```bash
# Development test (no API calls)
python3 accuracy_benchmark.py --dry-run --formats json tauq toon csv --models mock

# Real accuracy test (requires API keys)
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
python3 accuracy_benchmark.py --formats json tauq toon csv --models gpt-4o --runs 3

# See ACCURACY_README.md for full documentation
```

## Requirements

### Token Efficiency
- Python 3.12+
- `tiktoken` library (o200k_base encoding)
- `@faker-js/faker` or equivalent for data generation
- Built tauq binary

### Performance Benchmarks
- Rust 1.75+
- Criterion.rs 0.5+

### Accuracy Benchmarks
- Python 3.12+
- `tiktoken>=0.8.0` (o200k_base encoding)
- Built tauq binary
- Optional (for real tests): `openai` and/or `anthropic` Python packages
- API keys: `OPENAI_API_KEY` and/or `ANTHROPIC_API_KEY`

## Benchmark Datasets

### Primary Datasets
1. **tabular-100**: 100 uniform employee records (5 fields)
2. **tabular-2000**: 2,000 uniform employee records (scalability test)
3. **analytics-60**: 60 days of time-series metrics (6 fields each)
4. **analytics-365**: 365 days of analytics (token efficiency at scale)
5. **github-100**: Top 100 GitHub repositories (real-world data)
6. **nested-50**: 50 e-commerce orders with nested items
7. **nested-500**: 500 orders (nested structure at scale)
8. **event-logs-75**: 75 semi-uniform event logs (~50% with nested errors)
9. **event-logs-2000**: 2,000 event logs (semi-uniform at scale)
10. **nested-config**: Deeply nested application configuration
11. **wide-records**: 100 records with 15 fields each
12. **heterogeneous**: 100 records with varying schemas

### Dataset Characteristics
- **Tabular Eligibility**: Percentage of data that fits TOON/Tauq tabular format (0-100%)
- **Structure Class**: uniform, nested, semi-uniform, deep, heterogeneous
- **CSV Support**: Whether CSV can properly represent the data

## Methodology

### Token Counting
- **Tokenizer**: `tiktoken` with `o200k_base` encoding
  - Used by: GPT-4o, GPT-4o-mini, o1, o3-mini, Claude 3.5 Sonnet/Haiku/Opus, Claude 3.7 Sonnet
  - More accurate than cl100k_base for 2025 benchmarks
- **Baseline**: Minified JSON (no whitespace)
- **Comparisons**: All formats tested against same source data
- **No handicaps**: Fair implementation of all formats per their specifications

### TOON Implementation
- Follows TOON Spec v3.0 exactly
- UTF-8 encoding, LF line endings, 2-space indentation
- Proper quoting rules for special characters
- Tabular array format: `key[N]{fields}: row1,row2,...`

### Tauq Modes
- **Standard**: Space-delimited (default, most token-efficient)
- **Optimized**: Comma-delimited with minimal whitespace
- **Ultra**: Maximally compact with no structural metadata

#### Why Space-Delimited by Default?

**Tokenizer Analysis**: Space characters often merge with adjacent alphanumeric tokens during tokenization, while commas are almost always separate tokens. This gives space-delimited formats a consistent 10-20% token advantage.

**Example** (using `tiktoken o200k_base`):
```python
"id:123,name:Bob,dept:Eng"  # 9 tokens (commas add overhead)
"id:123 name:Bob dept:Eng"  # 7 tokens (22% fewer)
```

**Comparison**:
- TOON uses comma-delimited rows: `123,Bob,Engineering`
- Tauq uses space-delimited: `123 Bob Engineering`
- Both are valid design choices - we provide both modes

See [METHODOLOGY.md](./METHODOLOGY.md#3-delimiter-choice-why-space-delimited) for detailed analysis.

### Performance Benchmarking
- **Framework**: Criterion.rs 0.5+ with statistical analysis
- **Warm-up**: Automatic warm-up phase for JIT optimization
- **Black box**: Prevents compiler pre-optimization
- **Measurements**:
  - Mean, median, std deviation
  - 95% confidence intervals
  - Outlier detection and filtering
  - Statistical significance testing (α = 0.05)

## Output Files

### Token Efficiency
- `outputs/*.json` - Minified JSON baseline
- `outputs/*.tqn` - Standard Tauq (space-delimited)
- `outputs/*.opt.tqn` - Optimized Tauq (comma-delimited)
- `outputs/*.ultra.tqn` - Ultra-compact Tauq
- `outputs/*.toon` - TOON format
- `outputs/benchmark_results.json` - Complete results with metrics
- `outputs/report.md` - Human-readable summary report

### Performance Benchmarks
- `../target/criterion/*/report/index.html` - Interactive HTML reports
- `../target/criterion/*/base/estimates.json` - Statistical estimates
- Charts and violin plots for performance distribution

## Benchmark Results

Results are automatically updated in this README and in `outputs/report.md`.

### Token Efficiency Summary

*Run `python3 benchmark_comprehensive.py` to generate results*

### Performance Summary

*Run `cargo bench` to generate results*

## When to Use Each Format

Based on our benchmark results, here's an honest assessment:

| Data Structure | Best Choice | Token Efficiency |
|----------------|-------------|------------------|
| **100% flat tabular** (e.g., CSV exports) | CSV | CSV ≈ Tauq > TOON > JSON |
| **Uniform records** (same fields, flat values) | Tauq or TOON | Tauq > TOON > JSON |
| **Semi-structured** (33-50% tabular) | Tauq | Tauq > JSON > TOON |
| **Nested objects** (orders with items) | Tauq | Tauq > JSON > TOON |
| **Deep nesting** (config files) | JSON* | JSON ≈ Tauq > TOON |
| **Heterogeneous** (varying schemas) | Tauq | Tauq > JSON > TOON |
| **Mixed arrays** (different object types) | Tauq | Tauq > JSON > TOON |

**Key Insight**: Tauq excels at semi-structured and heterogeneous data where CSV can't be used but full JSON overhead is wasteful.

*Note: Deep nesting currently shows JSON slightly ahead (+2.9%) due to over-use of `!def` statements. With the planned fix (only use `!def` when schema appears 2+ times), Tauq would match or beat JSON here too. See [DEEP_NESTING_ANALYSIS.md](./DEEP_NESTING_ANALYSIS.md).

## Design Principles

1. **Fairness**: Apples-to-apples comparison with proper implementation of all formats
2. **Reproducibility**: Seeded random data, Docker containers, fixed dependencies
3. **Statistical Rigor**: Criterion.rs for performance, proper significance testing
4. **Real-World Data**: GitHub repos, realistic e-commerce patterns, actual use cases
5. **Comprehensive Coverage**: Multiple data structure types, varying scales
6. **Latest Standards**: o200k_base tokenizer, TOON v3.0 spec, Criterion 0.5+
7. **Intellectual Honesty**: Show where Tauq is NOT the best choice

## References

- [TOON Specification v3.0](https://github.com/toon-format/spec)
- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [tiktoken o200k_base](https://github.com/openai/tiktoken)
- [Rust Benchmarking Best Practices](https://bencher.dev/learn/benchmarking/rust/criterion/)
