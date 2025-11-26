# Tauq vs The World

Honest, benchmark-verified comparison of Tauq against major data serialization formats.

**All token counts verified with tiktoken (cl100k_base) - the tokenizer used by GPT-4 and Claude.**

## Quick Comparison

| Feature | JSON | YAML | TOON | Tauq |
|---------|------|------|------|------|
| **Token Efficiency** | Baseline | ~15% better | ~50% better (flat) | **~44-54% better** |
| **Count Required** | No | No | Yes `[N]` | **No** |
| **Schema Declaration** | None | None | `{fields}:` | **`!def Name fields`** |
| **Delimiter** | `,` (1 token) | Indent | `,` (1 token) | **Space (0 tokens)** |
| **Streaming Parse** | Block | Block | Block | **Iterator API** |
| **Programmable** | No | No | No | **Yes (TQQ)** |
| **Comments** | No | Yes | No | **Yes** |

---

## Benchmark Results (1000 Records)

Test data: User records with `id`, `name`, `email`, `role`, `active` fields.

| Format | Characters | Tokens | vs JSON |
|--------|------------|--------|---------|
| JSON (pretty) | 123,265 | 42,005 | -75% |
| JSON (minified) | 87,264 | **24,005** | baseline |
| TOON | 45,297 | 12,002 | **-50.0%** |
| **Tauq** | 43,297 | **11,012** | **-54.1%** |

### Why Tauq Wins on Token Count

1. **Space delimiters** - Spaces between values cost 0 extra tokens. Commas cost 1 token each.
2. **No count prefix** - TOON requires `[1000]{...}:` which adds tokens. Tauq uses `!def Name fields`.
3. **Simpler schema syntax** - `!def User id name` vs `users[N]{id,name}:`

---

## Detailed Comparisons

### Tauq vs JSON

**JSON (68 tokens):**
```json
[
  {"id": 1, "name": "Alice", "email": "alice@example.com"},
  {"id": 2, "name": "Bob", "email": "bob@example.com"}
]
```

**Tauq (24 tokens):**
```tqn
!def User id name email
1 Alice alice@example.com
2 Bob bob@example.com
```

**Savings: 65%** - Keys declared once, no braces/quotes/commas.

---

### Tauq vs TOON

**TOON:**
```toon
users[2]{id,name,email}:
  1,Alice,alice@example.com
  2,Bob,bob@example.com
```

**Tauq:**
```tqn
!def User id name email
1 Alice alice@example.com
2 Bob bob@example.com
```

**Key Differences:**

| Aspect | TOON | Tauq | Winner |
|--------|------|------|--------|
| Array declaration | `users[2]{id,name}:` | `!def User id name` | Tauq (shorter) |
| Count required | Yes (`[2]`) | No | Tauq (streaming-friendly) |
| Field delimiter | Comma | Space | Tauq (fewer tokens) |
| Nested objects | Indentation | Braces `{}` | Tie |
| Schema reuse | Per-array | Global `!use` | Tauq (more flexible) |

**Token counts (1000 records):**
- TOON: 12,002 tokens
- Tauq: 11,012 tokens
- **Tauq advantage: 8.2% fewer tokens**

**Overall (10 datasets, 55,647 tokens):**
- TOON: 34,830 tokens
- Tauq: 31,072 tokens
- **Tauq advantage: 10.8% fewer tokens**

---

### Tauq vs YAML

**YAML (38 tokens):**
```yaml
- id: 1
  name: Alice
- id: 2
  name: Bob
```

**Tauq (12 tokens):**
```tqn
!def User id name
1 Alice
2 Bob
```

**Savings: 68%** - No repeated keys, no colons, no dashes.

---

## When to Use Each Format

| Use Case | Best Format | Why |
|----------|-------------|-----|
| LLM context (tabular data) | **Tauq** | Maximum token efficiency |
| LLM context (nested config) | Tauq or YAML | Similar efficiency |
| Human editing | YAML | Familiar syntax |
| API interchange | JSON | Universal support |
| Streaming large datasets | **Tauq** | Iterator API, no count needed |
| Dynamic transformations | **Tauq (TQQ)** | Built-in query language |

---

## Streaming Comparison

### The Count Problem

TOON requires knowing array length upfront:
```toon
users[1000]{id,name}:  # Must know count before first row!
```

Tauq doesn't:
```tqn
!def User id name      # Schema defined, stream begins
1 Alice                # Rows can arrive indefinitely
2 Bob
...
```

### Iterator API

Tauq provides true streaming via `StreamingParser`:

```rust
use tauq::StreamingParser;

let mut parser = StreamingParser::new(input);
while let Some(record) = parser.next_record() {
    // Process one record at a time
    // Constant memory usage regardless of input size
}
```

---

## Benchmark Methodology

All benchmarks use:
- **Tokenizer**: tiktoken `cl100k_base` (GPT-4/Claude compatible)
- **Test data**: Synthetic user/product records with realistic field values
- **Measurements**: Actual token counts, not estimates

Run benchmarks yourself:
```bash
cd benchmarks && python3 token_benchmark.py
```

---

## Sources

- [TOON Specification](https://github.com/toon-format/spec) - Version 3.0 (Nov 2025)
- [tiktoken](https://github.com/openai/tiktoken) - OpenAI's tokenizer
- [Token Efficiency Analysis](https://saurav-samantray.medium.com/token-optimization-vs-context-loss-across-data-formats) - Nov 2025
