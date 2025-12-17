# LM Studio Accuracy Test Plan

## Test Configuration

**Model**: `gpt-oss-120b` (via LM Studio on `localhost:1234`)
**Formats**: json, tauq, tauq-no-schemas, toon
**Questions**: 300 (generated across multiple datasets)
**Runs per question**: 2 (to handle temperature variance)
**Total LLM calls**: 2,400 (300 × 4 formats × 2 runs)

## Test Setup

```bash
# LM Studio running locally
curl http://localhost:1234/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-oss-120b",
    "messages": [...],
    "temperature": 0.1,
    "max_tokens": 200,
    "stream": false
  }'

# Run benchmark
python3 accuracy_benchmark.py \
  --formats json tauq tauq-no-schemas toon \
  --models lmstudio/gpt-oss-120b \
  --runs 2
```

## What We're Testing

### Hypothesis 1: All Formats Have Similar Accuracy

**Expectation**: Token-efficient formats (Tauq, TOON) don't sacrifice accuracy.

**Why**: The data is structured identically, just represented differently.

**Metric**: If accuracies are within 95% CI overlap, no significant difference.

### Hypothesis 2: Tauq-No-Schemas ≥ Tauq (with schemas)

**Expectation**: Removing `!def` may improve or maintain accuracy.

**Reasoning**:
- No schema indirection (simpler for LLM to parse)
- More similar to training data (logs, configs use `key:value`)
- Still highly token-efficient (-54% vs JSON)

**Metric**: Compare accuracy of `tauq` vs `tauq-no-schemas`

### Hypothesis 3: Token Efficiency Matters

**Expectation**: Tauq wins on "accuracy per 1k tokens" metric.

**Reasoning**:
- Same accuracy with fewer tokens = better value
- Lower latency (less tokens to process)
- Lower cost (fewer tokens = cheaper API calls)

**Metric**: `(Accuracy × 1000) / Avg Tokens`

## Question Types

Generated questions cover 5 difficulty levels:

1. **Simple Lookup** (110 questions): "What is Alice's email?"
   - Direct field access
   - Single entity

2. **Filtered Lookup** (40 questions): "What is the email of the person in Engineering?"
   - Requires filtering
   - Single field from filtered result

3. **Aggregation** (90 questions): "How many people are active?"
   - Counting, summing
   - Requires iterating over dataset

4. **Comparison** (30 questions): "Who has the highest salary?"
   - Finding max/min
   - Requires comparing values

5. **Complex Reasoning** (30 questions): "List all departments with more than 10 active employees"
   - Multi-step logic
   - Grouping + filtering

## Token Efficiency Baseline

From dry run (10 questions):

| Format | Avg Tokens | vs JSON | Accuracy per 1k Tokens |
|--------|------------|---------|------------------------|
| JSON | 5,909 | baseline | 0.14% |
| **Tauq** | 1,821 | **-69.2%** | **0.44%** (3.2x better) |
| **Tauq-no-schemas** | 2,703 | **-54.3%** | **0.30%** (2.1x better) |
| TOON | ~2,100 | ~-64% | ~0.38% |

## Expected Results

### Scenario A: All Formats Equal Accuracy

```
Format             Accuracy    Tokens   Acc/1k Tok
JSON               75%         5,909    0.13%
Tauq               75%         1,821    0.41%     ← 3.2x better value
Tauq-no-schemas    75%         2,703    0.28%     ← 2.1x better value
TOON               75%         2,100    0.36%
```

**Conclusion**: Use Tauq for maximum efficiency with no accuracy loss.

### Scenario B: Tauq-No-Schemas Wins

```
Format             Accuracy    Tokens   Acc/1k Tok
JSON               75%         5,909    0.13%
Tauq               73%         1,821    0.40%
Tauq-no-schemas    78%         2,703    0.29%     ← Best for LLMs
TOON               70%         2,100    0.33%
```

**Conclusion**: For LLM applications, use `--no-schemas` for better comprehension while still saving 54% tokens.

### Scenario C: Tauq Has Issues

```
Format             Accuracy    Tokens   Acc/1k Tok
JSON               75%         5,909    0.13%
Tauq               65%         1,821    0.36%     ← Accuracy problem
Tauq-no-schemas    72%         2,703    0.27%     ← Better but still behind
TOON               68%         2,100    0.32%
```

**Conclusion**: Token-efficient formats have accuracy trade-offs. Recommend JSON/Markdown for accuracy-critical tasks.

## Statistical Significance

Using Wilson score for 95% confidence intervals:

**Small sample (N=10)**:
- 80% accuracy: CI = [49%, 94%] (wide)
- Hard to determine significant differences

**Large sample (N=300)**:
- 75% accuracy: CI = [70%, 80%] (narrow)
- Can detect differences of ~5% or more

**Significance test**:
- If CIs don't overlap: Significant difference (p < 0.05)
- If CIs overlap: No significant difference

## Validation Method

**Deterministic, type-aware validation**:

```python
# Numbers (1% tolerance)
Expected: 42.5  →  Accepts: 42.5, 42.51, "42.5"

# Strings (case-insensitive + fuzzy)
Expected: "Engineering"  →  Accepts: "engineering", "Enginering"

# Booleans
Expected: true  →  Accepts: true, "true", "yes", "1"

# Arrays (order-independent)
Expected: ["Alice", "Bob"]  →  Accepts: ["Bob", "Alice"], "Alice, Bob"
```

**No LLM-as-judge** - fully deterministic.

## Real-Time Monitoring

```bash
# Watch progress
tail -f outputs/accuracy/full_test.log

# Check results
cat outputs/accuracy/results.json | python3 -m json.tool

# View report
cat outputs/accuracy/report.md
```

## Cost Analysis (Local = Free!)

**Advantage of LM Studio**:
- Zero API costs
- Unlimited testing
- Fully reproducible
- Privacy (data stays local)

**If using OpenAI** (for comparison):
- 300 questions × 4 formats × 2 runs = 2,400 calls
- ~6M input tokens × $2.50/1M = $15
- ~100k output tokens × $10/1M = $1
- **Total: ~$16** (vs FREE with LM Studio)

## Success Criteria

✅ **Reproducibility**: Can others run this with LM Studio
✅ **Statistical rigor**: Large sample (N=300), multiple runs, 95% CI
✅ **Fair comparison**: Same questions, same model, deterministic validation
✅ **Transparent**: All results published, even if Tauq has issues

## Next Steps

1. **Wait for test to complete** (~30-60 minutes)
2. **Analyze results** with statistical significance
3. **Document findings** honestly
4. **Update recommendations** based on data

---

**Test Started**: 2025-11-26
**Status**: 🏃 Running
**Expected Duration**: 30-60 minutes
**Progress**: Check `outputs/accuracy/full_test.log`
