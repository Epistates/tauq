# Why Tauq Loses to JSON on Deeply Nested Structures

## Current Results

```
Dataset: nested-config (0% tabular, deep nesting)
  JSON: 311 tokens  ← WINNER
  Tauq: 320 tokens  (+2.9% vs JSON)
  TOON: 342 tokens  (+10.0% vs JSON)
```

## Root Causes

### Issue #1: Over-Use of `!def` Statements

**Problem**: The current formatter creates `!def` statements for schemas that are only used **once or twice**,adding overhead rather than saving tokens.

**Example from `nested-config.tqn`**:
```tauq
!def Replica host port priority    ← 6 tokens + 1 separator
!def Variant name weight config    ← 6 tokens
---

replicas [
  !use Replica                      ← Used only ONCE (3 tokens)
  replica-0.example.com 5432 1
  ...
]
```

**Token Analysis**:
- **Without `!def`**: Inline header `[3]{host,port,priority}:` = 8 tokens (used once)
- **With `!def`**: Definition (6) + separator (1) + use (3) = **10 tokens**
- **Overhead**: +2 tokens per single-use schema

**Break-Even Point**: `!def` should only be used when a schema is referenced **2+ times**:
```
1 use:  without=8 tok, with=10 tok → DON'T USE !def (-2 tokens)
2 uses: without=16 tok, with=13 tok → USE !def (+3 tokens)
3 uses: without=24 tok, with=16 tok → USE !def (+8 tokens)
```

**Current Threshold**: The formatter uses `arr.len() < 2` (needs at least 2 *items* in array)
**Correct Threshold**: Should check if schema is used `< 2` *times across the document*

### Issue #2: Indentation Overhead on Deep Nesting

**Problem**: Deep nesting without tabular data accumulates significant whitespace overhead.

**Example**:
```json
// JSON (compact): 46 tokens
{"level1":{"level2":{"level3":{"level4":{"level5":{"data":[1,2,3,4,5]}}}}}}

// Tauq (formatted): 51 tokens
level1:
  level2:
    level3:
      level4:
        level5:
          data: [1 2 3 4 5]
```

**Why JSON Wins**:
1. **No newlines**: Compact JSON uses 0 newlines (each newline = 1 token in Tauq)
2. **No indentation**: JSON has no spaces, Tauq accumulates (2, 4, 6, 8, 10... spaces)
3. **Compact braces**: `{}` are 1-2 tokens total vs Tauq's newlines + indentation

**Token Breakdown**:
```
Tauq overhead per nesting level:
  - Newline: 1 token
  - Indentation (2 spaces per level): 1 token per 2-3 levels
  - Colon: still needed (0 extra)

5 levels deep = ~5 newlines + ~2 indentation tokens = 7 extra tokens
JSON compact = 0 extra tokens
```

### Issue #3: No Tabular Data to Optimize

**Context**: Tauq's strength is tabular/semi-structured data where schema efficiency matters.

**Deeply Nested Config Files**:
- No repeated structures (0% tabular)
- No arrays of uniform objects
- Just nested key-value pairs

**Result**: Tauq's optimizations don't apply, but overhead remains.

## Solutions

### Solution #1: Fix `!def` Threshold (Immediate)

Change the schema detection logic to only use `!def` when:
1. Schema is used **2+ times** across the entire document
2. Token savings calculation: `(uses × inline_cost) > (def_cost + separator + uses × use_cost)`

**Code Change Needed** (in `formatter.rs`):
```rust
// Current: checks array length
fn detect_uniform_objects(&self, arr: &[Value]) -> Option<Vec<String>> {
    if arr.len() < 2 {  // ← Wrong threshold!
        return None;
    }
    // ...
}

// Should be: count schema uses across document
fn should_use_def(&self, schema: &Schema, use_count: usize) -> bool {
    if use_count < 2 {  // ← Correct threshold!
        return false;
    }

    // Calculate token savings
    let inline_cost = self.calc_inline_header_tokens(schema);
    let def_cost = self.calc_def_statement_tokens(schema);
    let use_cost = self.calc_use_statement_tokens(schema);
    let separator_cost = 1;

    let without_def = inline_cost * use_count;
    let with_def = def_cost + separator_cost + (use_cost * use_count);

    with_def < without_def
}
```

### Solution #2: Inline Compact Mode for Deep Nesting (Future)

For deeply nested objects (>4 levels) with no arrays, consider inline syntax:

```tauq
// Current (multi-line):
level1:
  level2:
    level3:
      data: value

// Proposed (inline):
level1: { level2: { level3: { data: value } } }
```

**When to Use**:
- Nesting depth > 4 levels
- No arrays or tabular data
- Object has < 3 keys per level
- Calculate: would inline be fewer tokens?

### Solution #3: Adaptive Format Selection (Future)

The formatter could detect when JSON would be more efficient:

```rust
fn should_use_json_instead(&self, value: &Value) -> bool {
    let json_tokens = count_tokens(&serde_json::to_string(value));
    let tauq_tokens = count_tokens(&self.format(value));
    json_tokens < tauq_tokens
}
```

Then emit: `!json {"level1":{"level2":...}}`

## Impact Analysis

### With Solution #1 (Fix `!def` Threshold):

**Expected Improvement for `nested-config`**:
- Remove unnecessary `!def` overhead: ~10-15 tokens
- New result: ~305-310 tokens vs JSON's 311 tokens
- **Tauq would tie or beat JSON** on this dataset

**Impact on Other Datasets**:
- No negative impact (only removes overhead from single-use schemas)
- Potential improvement on other low-repetition datasets

### With Solutions #2 + #3 (Inline + Adaptive):

**Expected Improvement**:
- Deep nesting (>4 levels): -20% tokens vs current Tauq
- Would match or beat JSON on purely nested structures

**Trade-offs**:
- More complex formatter logic
- Less human-readable output (inline is denser)
- Need to balance readability vs token efficiency

## Recommendation

**Priority 1** (Immediate): Fix `!def` threshold
- Simple change with clear benefit
- No downsides
- Would fix the `nested-config` benchmark

**Priority 2** (Future): Inline compact mode
- Opt-in flag: `--ultra-compact`
- For production LLM use cases where tokens > readability

**Priority 3** (Research): Adaptive format selection
- Interesting but complex
- May not be worth the added complexity

## Updated Benchmark Claims

**After fixing `!def` threshold**, our honest assessment would be:

| Data Structure | Best Choice | Notes |
|---|---|---|
| **100% flat tabular** | CSV | CSV ≈ Tauq > TOON > JSON |
| **Semi-structured** | Tauq | Tauq >> JSON > TOON |
| **Deep nesting** | **Tauq ≈ JSON** | After fixing !def overhead |
| **Heterogeneous** | Tauq | Tauq >> JSON >> TOON |

**Key Message**: With proper `!def` threshold, Tauq matches or beats JSON on **all** data structures except pure flat tabular (where CSV wins).
