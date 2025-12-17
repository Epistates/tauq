# Tauq Accuracy Benchmark

Comprehensive LLM accuracy testing suite addressing findings from [improvingagents.com/blog/toon-benchmarks](https://www.improvingagents.com/blog/toon-benchmarks).

## Background

External research found that TOON achieved only **43-47% accuracy** vs **54-62% for Markdown/YAML**. This benchmark tests whether Tauq has similar accuracy problems.

**Hypothesis**: Tauq's simpler syntax (`id:123 name:Bob` vs TOON's `[2]{id,name}: 123,Bob`) may improve LLM comprehension and accuracy.

## Quick Start

### Development Testing (Mock Model)

```bash
# Test harness functionality
python3 accuracy_benchmark.py --dry-run --formats json tauq toon csv --models mock --runs 2
```

### Real Accuracy Test (Requires API Keys)

```bash
# Set API keys
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."

# Run full benchmark (1000+ questions)
python3 accuracy_benchmark.py \
  --formats json tauq toon csv markdown yaml \
  --models gpt-4o claude-3-5-sonnet-20241022 \
  --runs 3

# Quick test (10 questions)
python3 accuracy_benchmark.py \
  --dry-run \
  --formats json tauq toon \
  --models gpt-4o \
  --runs 3
```

## Benchmark Design

### Test Methodology

Following improvingagents.com methodology + addressing Reddit critiques:

1. **Large Sample Size**: 1000+ questions for statistical significance
2. **Multiple Runs**: 3-5 runs per question (handle temperature variance)
3. **Confidence Intervals**: Wilson score for 95% CI
4. **Deterministic Validation**: No LLM-as-judge, type-aware comparison
5. **Multiple Models**: Different families and sizes
6. **Diverse Questions**: 5 question types across multiple datasets

### Question Types

Distribution across 300 generated questions:

- **Simple Lookup** (30%): "What is Bob's email?"
- **Filtered Lookup** (20%): "What is the email of the person in Engineering?"
- **Aggregation** (20%): "How many people are active?"
- **Comparison** (15%): "Who has the highest salary?"
- **Complex Reasoning** (15%): "List all departments with more than 10 active employees"

### Datasets

Generated from `benchmark_comprehensive.py`:

1. **Employee Records**: 100 uniform records, 5 fields (tabular)
2. **Analytics**: 365 days of metrics (time-series)
3. **GitHub Repos**: 100 repositories (nested, real-world)
4. **E-commerce Orders**: 500 orders with nested items
5. **Event Logs**: 2000 semi-uniform logs (~50% with errors)
6. **Config Files**: Deeply nested application config

### Answer Validation

**Type-Aware Deterministic Validation** (no LLM judge):

```python
# Boolean validation
Expected: true
Accepts: "true", "True", "yes", "1", true

# Numeric validation
Expected: 42.5
Accepts: "42.5", "42.50", 42.5 (±1% tolerance)

# String validation
Expected: "Engineering"
Accepts: "engineering", " Engineering ", "Enginering" (Levenshtein distance < 2)

# Array validation
Expected: ["Bob", "Alice"]
Accepts: ["Bob", "Alice"], ["Alice", "Bob"], "Bob, Alice", "Bob and Alice"
```

### Statistical Analysis

**Wilson Score Method** for 95% confidence intervals:

```
For 47 correct out of 100 questions:
  Accuracy: 47.0%
  95% CI: [37.1%, 56.9%]

Interpretation: We're 95% confident true accuracy is between 37-57%
```

**Token Efficiency Metric**:

```
Accuracy per 1k tokens = (Accuracy × 1000) / Avg Tokens

Example:
  Tauq: 50% accuracy, 2000 tokens → 25.0% per 1k tokens
  JSON: 55% accuracy, 6000 tokens → 9.2% per 1k tokens
```

## Test Harness Components

### 1. Question Generator

Generates diverse questions across multiple datasets:

```python
from accuracy_benchmark import QuestionGenerator

# Generate 200 employee questions
questions = QuestionGenerator.generate_employee_questions(employee_data, 200)

# Question structure
@dataclass
class Question:
    id: str                    # Unique identifier
    question: str              # Natural language question
    expected_answer: Any       # Ground truth answer
    question_type: QuestionType  # Simple, filtered, aggregation, etc.
    dataset_name: str          # Source dataset
```

### 2. Answer Validator

Type-aware validation with fuzzy matching:

```python
from accuracy_benchmark import AnswerValidator

# Validate answers
is_correct = AnswerValidator.validate(
    predicted="42.3",
    expected=42.5,
    tolerance=0.01  # 1% tolerance for numbers
)
```

### 3. Benchmark Orchestrator

Runs tests across formats and models:

```python
from accuracy_benchmark import AccuracyBenchmark

benchmark = AccuracyBenchmark()

# Run full benchmark
results = benchmark.run_benchmark(
    formats=["json", "tauq", "toon", "csv"],
    models=["gpt-4o", "claude-3-5-sonnet-20241022"],
    num_runs=3,
    dry_run=False
)

# Generate report
report = benchmark.generate_report(results)
print(report)
```

## Output Files

After running benchmarks:

```
outputs/accuracy/
├── results.json          # Complete results with all metrics
├── report.md             # Human-readable summary
└── questions.json        # Generated questions (for reproducibility)
```

### results.json Structure

```json
{
  "json_gpt-4o": {
    "format": "json",
    "model": "gpt-4o",
    "accuracy": 0.573,
    "ci_lower": 0.541,
    "ci_upper": 0.604,
    "correct": 573,
    "total": 1000,
    "avg_tokens": 5909,
    "avg_latency_ms": 432.1,
    "accuracy_per_1k_tokens": 9.69
  },
  "tauq_gpt-4o": {
    "format": "tauq",
    "model": "gpt-4o",
    "accuracy": 0.512,
    "ci_lower": 0.479,
    "ci_upper": 0.545,
    "correct": 512,
    "total": 1000,
    "avg_tokens": 1821,
    "avg_latency_ms": 401.3,
    "accuracy_per_1k_tokens": 28.12
  }
}
```

## Supported Models

### Frontier Models (Recommended)

- **GPT-4o**: `gpt-4o` (OpenAI)
- **Claude 3.5 Sonnet**: `claude-3-5-sonnet-20241022` (Anthropic)
- **GPT-4o-mini**: `gpt-4o-mini` (OpenAI, cost-efficient)

### Additional Models (Future)

- Gemini 2.0 Flash (Google)
- Llama 3.3 70B (open source)
- Qwen 2.5 72B (open source)

## Supported Formats

| Format | Status | Notes |
|--------|--------|-------|
| JSON | ✅ Ready | Baseline format |
| Tauq | ✅ Ready | Adaptive !def usage (default) |
| Tauq (no-schemas) | ✅ Ready | No !def schemas, pure key:value (for LLM testing) |
| TOON | ✅ Ready | v3.0 spec compliant |
| CSV | ✅ Ready | Flat tabular data only |
| Markdown | ⏳ TODO | Tables for tabular, text for nested |
| YAML | ⏳ TODO | Nested data representation |

**Note**: `tauq-no-schemas` format (`--no-schemas` flag) disables !def schemas for testing LLM comprehension while still saving 50%+ tokens vs JSON. See [REFACTORING_SUMMARY.md](./REFACTORING_SUMMARY.md) for details.

## Interpreting Results

### Expected Outcomes

Based on improvingagents.com findings:

**Scenario 1: Tauq matches TOON (43-47%)**
- ❌ Both have significant accuracy problems
- ✅ Token efficiency is still valuable for cost-constrained scenarios
- 📋 Recommendation: Use JSON/Markdown for accuracy-critical tasks

**Scenario 2: Tauq between TOON and Markdown (48-53%)**
- ✅ Better than TOON (simpler syntax helps)
- ⚠️ Still worse than Markdown
- 📋 Recommendation: Balance accuracy vs token cost

**Scenario 3: Tauq matches Markdown (54-62%)**
- ✅✅ Best of both worlds: token efficiency + accuracy
- 📋 Recommendation: Tauq for most LLM applications

### Statistical Significance

**Non-overlapping 95% CIs indicate significant difference**:

```
Format A: 47.0% [37.1%, 56.9%]
Format B: 58.0% [48.2%, 67.8%]
```

CIs overlap → difference may not be significant

```
Format A: 47.0% [42.5%, 51.5%]
Format B: 58.0% [53.6%, 62.4%]
```

CIs don't overlap → difference is likely real (p < 0.05)

## Cost Analysis

Estimated costs for full benchmark (1000 questions × 3 runs):

```
GPT-4o:
  - Input: ~6M tokens × $2.50/1M = $15.00
  - Output: ~100k tokens × $10.00/1M = $1.00
  - Total per format: ~$16.00
  - 4 formats: ~$64.00

Claude 3.5 Sonnet:
  - Input: ~6M tokens × $3.00/1M = $18.00
  - Output: ~100k tokens × $15.00/1M = $1.50
  - Total per format: ~$19.50
  - 4 formats: ~$78.00

Total (both models, 4 formats): ~$142
```

**Cost Optimization**:
- Use `--dry-run` for testing (10 questions only)
- Start with `gpt-4o-mini` ($0.15/1M input)
- Test fewer formats initially

## Development

### Running Tests

```bash
# Test harness functionality (no API calls)
python3 accuracy_benchmark.py --dry-run --models mock

# Test single format
python3 accuracy_benchmark.py --dry-run --formats tauq --models gpt-4o-mini

# Full benchmark
python3 accuracy_benchmark.py --formats json tauq toon csv --models gpt-4o claude-3-5-sonnet-20241022 --runs 5
```

### Adding New Formats

```python
def your_format_encode(data: Any) -> str:
    """Encode data in your custom format"""
    # Implementation here
    return formatted_string

# In AccuracyBenchmark.format_data():
if format_name == "your-format":
    return self.your_format_encode(data)
```

### Adding New Datasets

```python
# In QuestionGenerator:
@staticmethod
def generate_your_questions(data: Dict, num_questions: int = 200):
    questions = []

    # Generate diverse questions
    for i in range(num_questions):
        questions.append(Question(
            id=f"your_{i}",
            question="Your question here?",
            expected_answer=compute_answer(data),
            question_type=QuestionType.SIMPLE_LOOKUP,
            dataset_name="your-dataset"
        ))

    return questions
```

## Commitment to Transparency

We will publish **all results**, even if they show:
- Tauq has accuracy problems
- Other formats are better for certain use cases
- Token efficiency comes at unacceptable accuracy cost

**Our goal**: Help users make informed decisions, not just promote Tauq.

## References

- [improvingagents.com: TOON accuracy analysis](https://www.improvingagents.com/blog/toon-benchmarks)
- [Reddit: Stopping the Toon hype](https://www.reddit.com/r/LocalLLaMA/comments/1oylf8m/stopping_the_toon_hype_with_a_proper_benchmark/)
- [Wilson Score Confidence Interval](https://en.wikipedia.org/wiki/Binomial_proportion_confidence_interval#Wilson_score_interval)
- [TOON Specification v3.0](https://github.com/toon-format/spec)

---

**Status**: ✅ Test harness complete and validated
**Last Updated**: 2025-11-26
**Ready for**: Real accuracy testing with API keys
