# Tauq - Token-Efficient Data Notation

**44% fewer tokens than JSON overall. 11% more efficient than TOON. Verified with tiktoken.**

[![Crates.io](https://img.shields.io/crates/v/tauq?label=crates.io)](https://crates.io/crates/tauq)
[![npm](https://img.shields.io/npm/v/tauq?label=npm)](https://www.npmjs.com/package/tauq)
[![PyPI](https://img.shields.io/pypi/v/tauq?label=pypi)](https://pypi.org/project/tauq/)
[![Downloads](https://img.shields.io/crates/d/tauq?label=downloads)](https://crates.io/crates/tauq)
[![Tests](https://img.shields.io/badge/tests-88_passing-brightgreen)]()
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## What is Tauq?

**Tauq** (τq) is two things:

1.  **Tauq Notation (`.tqn`)**: A schema-driven data format that achieves 44-54% fewer tokens than JSON (verified with tiktoken cl100k_base).
2.  **Tauq Query (`.tqq`)**: A pre-processor with shell integration for data transformations.

Built for the AI era where every token counts.

---

## Benchmark (1000 Records)

| Format | Tokens | vs JSON |
|--------|--------|---------|
| JSON (minified) | 24,005 | baseline |
| TOON | 12,002 | -50.0% |
| **Tauq** | **11,012** | **-54.1%** |

*All counts verified with tiktoken cl100k_base (GPT-4/Claude tokenizer).*

**Overall (10 datasets, 55,647 tokens):** Tauq saves 44.2% vs JSON, 10.8% vs TOON. See `benchmarks/` for full results.

## Quick Example

**JSON:**
```json
[{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]
```

**Tauq:**
```tqn
!def User id name
1 Alice
2 Bob
```

---

## Features

### Token-Optimal
- 44-54% fewer tokens than JSON (verified benchmarks)
- 11% more efficient than TOON overall
- Space delimiters tokenize better than commas

### True Streaming
- `StreamingParser` iterator API
- Process records one at a time
- No count required (unlike TOON's `[N]`)

### Schema-Driven
- Define data shapes with `!def`
- Switch schemas with `!use`
- Nested types and typed arrays

### 🔧 **Programmable**
- Tauq Query for data transformations
- Unix pipe model
- Polyglot support (Python, Rhai, JavaScript)

### 🛠️ **Production-Ready CLI**
- `tauq build` - Parse to JSON
- `tauq format` - JSON → Tauq
- `tauq minify` - Compress to one line
- `tauq exec` - Run Tauq Query pipelines
- `tauq validate` - Check syntax

---

## Quick Start

### Installation

**CLI Tool:**
```bash
cargo install tauq
```

### Language Bindings

**Rust:**
```toml
[dependencies]
tauq = "0.1"
```

**Python:**
```bash
pip install tauq
```

**JavaScript/TypeScript:**
```bash
npm install tauq
```

**Go:**
```bash
go get github.com/epistates/tauq
```

Other languages: Java, C#, Swift - see [Language Bindings](bindings/README.md)

### Hello World

Create `config.tqn`:
```tqn
app_name "MyService"
version "1.0.0"
port 8080
debug true
features [api websockets metrics]
```

Parse to JSON:
```bash
$ tauq build config.tqn --pretty
{
  "app_name": "MyService",
  "version": "1.0.0",
  "port": 8080,
  "debug": true,
  "features": ["api", "websockets", "metrics"]
}
```

---

## Syntax Guide

### Simple Values
```tqn
name "Alice"
age 30
active true
score 99.5
missing null
role admin  # Barewords don't need quotes
```

### Arrays
```tqn
tags [web api backend]
ids [1 2 3 4 5]
mixed [1 "two" true null]
```

### Tabular Data (The Killer Feature)

```tqn
!def User id name email role

1 Alice "alice@example.com" admin
2 Bob "bob@example.com" user
3 Carol "carol@example.com" user
```

### Schema Block

Define schemas upfront with `---` to separate from data:

```tqn
!def User id name role
---
users [
  !use User
  1 Alice admin
  2 Bob user
]
```

The `---` separator clears the implicit schema scope, allowing structured key-value data that uses `!use` inside arrays.

### Nested Types

```tqn
!def Address street city
!def User id name addr:Address

1 Alice { "123 Main" "NYC" }
2 Bob { "456 Oak" "LA" }
```

### Lists of Objects

```tqn
!def Employee name role
!def Department name budget employees:[Employee]

Engineering 1000000 [
    Alice "Principal Engineer"
    Bob "Senior Engineer"
]
```

### Minified Syntax

```tqn
!def U id name; 1 Alice; 2 Bob
```

All on one line for maximum compression!

---

## Examples

We have provided a comprehensive set of examples in the `examples/` directory:

*   **[Basics](examples/1_basics/)**: Simple configuration and primitive types.
*   **[Schemas](examples/2_schemas/)**: Typed schemas and nested types.
*   **[Modularity](examples/3_modularity/)**: Multi-file imports and modular configurations.
*   **[Real World](examples/4_real_world/)**: Production configurations like Kubernetes deployments.
*   **[Queries](examples/5_queries/)**: ETL pipelines and data generation with TauqQ.
*   **[Minified](examples/6_minified/)**: Compact single-line syntax examples.

---

## CLI Usage

### Build: Tauq → JSON

```bash
# To stdout
tauq build data.tqn

# To file with pretty formatting
tauq build data.tqn -o data.json --pretty

# From stdin
cat data.tqn | tauq build -
```

### Format: JSON → Tauq

The formatter intelligently detects arrays of uniform objects and creates schemas automatically:

```bash
# Convert JSON to Tauq (auto-generates schemas for nested arrays)
tauq format data.json -o data.tqn

# From stdin
echo '{"users": [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]}' | tauq format -
# Output:
# !def User id name
# ---
# users [
#   !use User
#   1 Alice
#   2 Bob
# ]
```

### Execute Tauq Query

```bash
# Run data transformations
tauq exec pipeline.tqq -o output.json

# Run in SAFE MODE (disable shell execution)
tauq exec pipeline.tqq --safe
```

### Minify

```bash
# Compress to single line
tauq minify data.tqn -o data.min.tqn
```

---

## Contributing

Tauq is in active development. Contributions welcome!

**Areas of interest:**
- Parser optimizations
- Error message improvements
- Language bindings (Python, JS, Go)
- Documentation
- Real-world use cases

---

## License

MIT

---

**Tauq (τq) - Stop wasting tokens on JSON. Start using the future.** 🚀