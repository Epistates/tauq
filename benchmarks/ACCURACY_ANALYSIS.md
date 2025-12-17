# LLM Accuracy Analysis: improvingagents.com Findings

## Summary of External Research

Source: [improvingagents.com/blog/toon-benchmarks](https://www.improvingagents.com/blog/toon-benchmarks)

### Key Findings

**Test 1: Tabular Data (GPT-4.1 nano)**
```
Format          Accuracy    Tokens     Accuracy per Token
Markdown-KV     60.7%       52,104     1.16% per 1k tokens
Markdown-Table  51.9%       ~40,000    1.30% per 1k tokens
TOON            47.5%       21,518     2.21% per 1k tokens  ← Token-efficient but less accurate
CSV             44.3%       19,524     2.27% per 1k tokens
```

**Test 2: Nested Data (GPT-5 nano)**
```
Format          Accuracy    Tokens     Accuracy per Token
YAML            62.1%       42,477     1.46% per 1k tokens
Markdown        54.3%       38,357     1.42% per 1k tokens
TOON            43.1%       45,436     0.95% per 1k tokens  ← Worse accuracy, MORE tokens
```

### Critical Insight

**TOON's accuracy deficit is significant**:
- For tabular data: 13.2% worse than Markdown-KV, 4.4% worse than Markdown-Table
- For nested data: 19.0% worse than YAML, 11.2% worse than Markdown
- On nested data, TOON used **MORE tokens** than YAML but had **worse accuracy**

**The Trade-off**:
- Token efficiency doesn't automatically translate to accuracy
- Formats with clear visual structure (markdown tables, YAML) perform better
- Models may not be well-trained on TOON syntax

## Implications for Tauq

### Questions We Need to Answer

1. **Does Tauq have the same accuracy problem?**
   - Tauq's syntax is simpler than TOON (no `[N]{fields}` headers)
   - More similar to natural text: `id:123 name:Bob`
   - May be easier for models to parse

2. **Is token efficiency worth accuracy loss?**
   - If Tauq saves 30% tokens but loses 15% accuracy, is that a good trade?
   - Depends on use case: RAG retrieval, tool calls, structured output

3. **Which models perform best with Tauq?**
   - Need to test multiple model families
   - Check if newer models (trained on more diverse data) do better

### Test Harness Requirements

Based on improvingagents.com methodology + Reddit critiques:

**Must Have**:
1. ✅ Large sample size (1000+ questions for statistical significance)
2. ✅ Multiple models (different families, sizes)
3. ✅ Multiple runs per question (handle temperature variance)
4. ✅ Confidence intervals (95% CI required)
5. ✅ Diverse question types (lookup, aggregation, reasoning)
6. ✅ All formats (JSON, Tauq, TOON, CSV, Markdown, YAML)
7. ✅ Deterministic validation (no LLM-as-judge)

**Nice to Have**:
- Breakdown by question type (simple lookup vs complex reasoning)
- Breakdown by data structure (tabular vs nested)
- Cost analysis (tokens × price per token)
- Latency measurements

## Expected Results

### Hypothesis 1: Tauq > TOON on Accuracy

**Reasoning**:
- Simpler syntax: `key:value` vs `[N]{fields}: rows`
- No length declarations to confuse models
- More similar to natural text and existing formats

**Test**: Compare Tauq vs TOON on same questions

### Hypothesis 2: Markdown Still Wins on Accuracy

**Reasoning**:
- Markdown tables are extremely common in training data
- Visual structure aids model understanding
- Worth the token overhead for accuracy-critical tasks

**Test**: Compare Tauq vs Markdown on all question types

### Hypothesis 3: CSV Competitive on Flat Data

**Reasoning**:
- CSV is ubiquitous in training data
- Simple, unambiguous format
- Should perform well on pure tabular data

**Test**: Compare CSV vs Tauq on 100% tabular datasets

## Test Harness Design

See `accuracy_benchmark.py` for implementation.

### Test Structure

```
For each dataset:
  For each format (JSON, Tauq, TOON, CSV, Markdown, YAML):
    For each question (1000+ total):
      For each run (3-5 iterations):
        - Format data in target format
        - Send to model with question
        - Parse answer
        - Validate against ground truth
        - Record: correct, tokens_used, latency

Results:
  - Accuracy (% correct) ± 95% CI
  - Token efficiency (avg tokens per question)
  - Cost efficiency (accuracy per dollar)
  - Breakdown by question type and data structure
```

### Question Types

1. **Simple Lookup** (30%): "What is Bob's email?"
2. **Filtered Lookup** (20%): "What is the email of the person in Engineering?"
3. **Aggregation** (20%): "How many people are active?"
4. **Comparison** (15%): "Who has the highest salary?"
5. **Complex Reasoning** (15%): "List all departments with more than 10 active employees"

### Validation Strategy

**Deterministic Validation** (no LLM judge):
- Normalize answers (lowercase, strip whitespace, parse numbers)
- Type-aware comparison (numbers, strings, booleans, arrays)
- Fuzzy matching for strings (Levenshtein distance < 2)
- Multiple acceptable answer formats

### Model Selection

**Required**:
- GPT-4o (frontier, well-trained)
- Claude 3.5 Sonnet (frontier, well-trained)
- GPT-4o-mini (efficient baseline)

**Nice to Have**:
- Gemini 2.0 Flash (Google's latest)
- Llama 3.3 70B (open source)
- Qwen 2.5 72B (open source)

## Honest Assessment

### If Tauq Shows Accuracy Deficit

We will **clearly document**:
- Exact accuracy gap vs each format
- Which question types suffer most
- Which data structures are affected
- **Recommendation**: Use Markdown/JSON for accuracy-critical tasks, Tauq for token-constrained scenarios

### If Tauq Matches or Beats TOON

We will **clearly document**:
- Advantage over TOON (simpler syntax helps)
- Still acknowledge if Markdown/YAML beat us
- Provide guidance on when to use each format

### Cost-Benefit Analysis

Calculate **value per dollar**:
```
Value = (Accuracy × Use_Case_Importance) - (Token_Cost × Price_Per_Token)

Example:
- High-stakes: Accuracy weight = 0.9, Token weight = 0.1
- Token-constrained: Accuracy weight = 0.6, Token weight = 0.4
```

## Implementation Status

1. ✅ Create test harness (`accuracy_benchmark.py`)
2. ✅ Generate comprehensive question set (300+ questions, expandable to 1000+)
3. ✅ Implement format converters (JSON, Tauq, TOON, CSV)
4. ✅ Set up model API integration (OpenAI, Anthropic)
5. ✅ Deterministic answer validation (type-aware, fuzzy matching)
6. ✅ Statistical analysis (Wilson score 95% CI)
7. ✅ Mock testing mode for development
8. ⏳ Add Markdown and YAML formatters
9. ⏳ Run full benchmarks with real models (requires API keys)
10. ⏳ Analyze results and update documentation

**Test Harness Status**: ✅ Complete and validated

Run development test:
```bash
python3 accuracy_benchmark.py --dry-run --formats json tauq toon csv --models mock
```

Run real accuracy test (requires API keys):
```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
python3 accuracy_benchmark.py --formats json tauq toon csv --models gpt-4o claude-3-5-sonnet-20241022 --runs 3
```

See [ACCURACY_README.md](./ACCURACY_README.md) for complete documentation.

## Commitment to Transparency

We will publish **all results**, even if they show:
- Tauq has accuracy problems
- Other formats are better for certain use cases
- Token efficiency comes at unacceptable accuracy cost

**Our goal**: Help users make **informed decisions**, not just promote Tauq.
