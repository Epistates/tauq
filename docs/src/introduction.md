# Introduction

**Tauq** (τq) is a token-efficient data format suite built for the AI era where every token counts.

## The Problem

JSON is verbose. In the age of Large Language Models (LLMs), verbosity equals cost and latency.

```json
[
  {"id": 1, "name": "Alice", "email": "alice@example.com", "role": "admin", "active": true},
  {"id": 2, "name": "Bob", "email": "bob@example.com", "role": "user", "active": true}
]
```
**~242 tokens** (minified) per 100 records. **92 KB** for 1,000 records.

## The Solution

Tauq offers two complementary formats:

### TQN (Tauq Notation) - For LLM Token Efficiency

```tqn
!def User id name email role active
1 Alice alice@example.com admin true
2 Bob bob@example.com user true
```
**~110 tokens** per 100 records. **54% fewer tokens** than JSON.

### TBF (Tauq Binary Format) - For Size & Speed

```rust
#[derive(TableEncode)]
struct User {
    #[tauq(encoding = "u16")]
    id: u32,
    name: String,
}
```
**16 KB** for 1,000 records (**83% smaller** than JSON with schema-aware encoding).
With generic serde: 44-56% reduction via CLI converter.

## Benchmark Results

### Token Efficiency (for LLMs)

| Format | 1000 Records | Tokens | vs JSON |
|--------|--------------|--------|---------|
| JSON (minified) | 92 KB | 24,005 | baseline |
| TOON | 45 KB | 12,002 | -50.0% |
| **Tauq (TQN)** | **43 KB** | **11,012** | **-54.1%** |

### Binary Size (for storage/network)

| Format | 1000 Records | vs JSON |
|--------|--------------|---------|
| JSON (minified) | 92 KB | baseline |
| Tauq (TQN) | 43 KB | -53% |
| **Tauq (TBF)** | **16 KB** | **-83%** |

*All token counts verified with tiktoken cl100k_base (GPT-4/Claude tokenizer).*

## Key Features

- **Token-Optimal (TQN):** 44-54% fewer tokens than JSON for LLM inputs.
- **Binary Format (TBF):** 83% smaller than JSON with columnar encoding.
- **Schema-Driven:** Define shapes with `!def` and switch with `!use`.
- **True Streaming:** `StreamingParser` yields records one at a time.
- **Iceberg Integration:** TBF integrates with Apache Iceberg for data lakes.
- **Programmable:** Use **Tauq Query (TQQ)** for data transformations.
- **Polyglot:** Bindings for Python, JavaScript, Go, Java, C#, Swift, and Rust.

## When to Use Which Format

| Scenario | Use TQN | Use TBF |
|----------|---------|---------|
| LLM prompts/responses | Yes | No |
| Config files | Yes | No |
| Database storage | No | Yes |
| Network transfer | Either | Preferred |
| Apache Iceberg tables | No | Yes |
| Human editing | Yes | No |

## Why Tauq (TQN) Beats TOON

| Feature | TOON | Tauq (TQN) |
|---------|------|------------|
| Count required | Yes `[N]` | **No** |
| Delimiter | Comma (1 token) | **Space (0 tokens)** |
| Streaming | Block parse | **Iterator API** |
| Query language | No | **Yes (TQQ)** |
| Comments | No | **Yes** |
