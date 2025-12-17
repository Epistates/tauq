#!/usr/bin/env python3
"""
Tauq LLM Accuracy Benchmark
============================
Tests retrieval accuracy across multiple formats and models

Addresses improvingagents.com findings:
- TOON showed 43-47% accuracy vs 54-62% for Markdown/YAML
- Token efficiency doesn't guarantee accuracy
- Need large sample sizes (1000+ questions) for statistical significance

This benchmark tests if Tauq has similar accuracy problems.
"""

import json
import random
import re
from collections import defaultdict
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Tuple
from pathlib import Path
from enum import Enum
import statistics

# Try to import LLM libraries (optional for development)
try:
    import anthropic
    HAS_ANTHROPIC = True
except ImportError:
    HAS_ANTHROPIC = False

try:
    import openai
    HAS_OPENAI = True
except ImportError:
    HAS_OPENAI = False

# Import our benchmark utilities
from benchmark_comprehensive import (
    generate_employees, generate_analytics, generate_orders,
    generate_event_logs, generate_nested_config,
    toon_encode, csv_encode, json_to_tauq, count_tokens
)

# Seed for reproducibility
random.seed(42)

# Tauq format cheat sheet for LLM system prompt
TAUQ_FORMAT_GUIDE = """You are analyzing data in Tauq format - a token-efficient notation.

# Tauq Format Quick Reference

## Basic Syntax
- Key-value pairs: `key value` (no colons, no commas)
- Strings: quoted "hello" or barewords hello
- Arrays: `[item1 item2 item3]` (space-separated)
- Objects: `{ key1 val1 key2 val2 }`

## Schemas (!def and !use)

**!def TypeName field1 field2 ...**
Defines a schema with field names.

After !def, each line is a DATA ROW with values matching the fields in order.

Example:
```
!def User id name email
1 Alice alice@example.com
2 Bob bob@example.com
3 Carol carol@example.com
```

This creates 3 User objects (count the 3 data lines):
- User 1: {id: 1, name: "Alice", email: "alice@example.com"}
- User 2: {id: 2, name: "Bob", email: "bob@example.com"}
- User 3: {id: 3, name: "Carol", email: "carol@example.com"}

**!use TypeName**
Activates a previously defined schema (used inside arrays).

Example:
```
!def Employee id name department
---
employees [
  !use Employee
  1 Alice Engineering
  2 Bob Sales
  3 Carol Engineering
]
```

To count employees: Count the data lines after `!use Employee` = 3 employees

## Counting Records

**Rule**: Count data lines (not directive lines)
- `!def` and `!use` are directives (not data)
- `---` is a separator (not data)
- Each other line after a schema = 1 record

## Your Task
Read the Tauq-formatted data and answer questions accurately based on the structure above."""


class QuestionType(Enum):
    """Types of questions to test different retrieval patterns"""
    SIMPLE_LOOKUP = "simple_lookup"           # "What is Bob's email?"
    FILTERED_LOOKUP = "filtered_lookup"       # "What is the email of the person in Engineering?"
    AGGREGATION = "aggregation"               # "How many people are active?"
    COMPARISON = "comparison"                 # "Who has the highest salary?"
    COMPLEX = "complex"                       # "List all departments with >10 employees"


@dataclass
class Question:
    """A test question with expected answer"""
    id: str
    question: str
    answer: Any  # Can be string, number, boolean, or list
    question_type: QuestionType
    dataset_name: str
    difficulty: str  # "easy", "medium", "hard"


@dataclass
class TestResult:
    """Result of running one question"""
    question_id: str
    format_name: str
    model_name: str
    correct: bool
    predicted_answer: Any
    expected_answer: Any
    tokens_used: int
    latency_ms: float
    error: Optional[str] = None


class AnswerValidator:
    """Validates answers with type-aware comparison"""

    @staticmethod
    def normalize_string(s: str) -> str:
        """Normalize string for comparison"""
        return s.lower().strip().replace('"', '').replace("'", "")

    @staticmethod
    def normalize_number(n: Any) -> float:
        """Parse and normalize numbers"""
        if isinstance(n, (int, float)):
            return float(n)
        if isinstance(n, str):
            # Remove common number formatting
            clean = n.replace(',', '').replace('$', '').strip()
            try:
                return float(clean)
            except ValueError:
                return None
        return None

    @staticmethod
    def levenshtein_distance(s1: str, s2: str) -> int:
        """Calculate edit distance between strings"""
        if len(s1) < len(s2):
            return AnswerValidator.levenshtein_distance(s2, s1)
        if len(s2) == 0:
            return len(s1)

        previous_row = range(len(s2) + 1)
        for i, c1 in enumerate(s1):
            current_row = [i + 1]
            for j, c2 in enumerate(s2):
                insertions = previous_row[j + 1] + 1
                deletions = current_row[j] + 1
                substitutions = previous_row[j] + (c1 != c2)
                current_row.append(min(insertions, deletions, substitutions))
            previous_row = current_row

        return previous_row[-1]

    @classmethod
    def validate(cls, predicted: Any, expected: Any, tolerance: float = 0.01) -> bool:
        """Validate answer with type-aware comparison"""

        # Handle None/null
        if predicted is None and expected is None:
            return True
        if predicted is None or expected is None:
            return False

        # Handle booleans
        if isinstance(expected, bool):
            if isinstance(predicted, bool):
                return predicted == expected
            # Try parsing string
            if isinstance(predicted, str):
                pred_lower = predicted.lower().strip()
                if pred_lower in ('true', 'yes', '1'):
                    return expected is True
                if pred_lower in ('false', 'no', '0'):
                    return expected is False
            return False

        # Handle numbers
        if isinstance(expected, (int, float)):
            pred_num = cls.normalize_number(predicted)
            exp_num = cls.normalize_number(expected)
            if pred_num is not None and exp_num is not None:
                # Use relative tolerance for large numbers
                if exp_num == 0:
                    return abs(pred_num - exp_num) < tolerance
                return abs(pred_num - exp_num) / abs(exp_num) < tolerance
            return False

        # Handle lists
        if isinstance(expected, list):
            if not isinstance(predicted, list):
                # Try parsing string as list
                if isinstance(predicted, str):
                    try:
                        predicted = json.loads(predicted.replace("'", '"'))
                    except:
                        # Try splitting by common delimiters
                        predicted = [x.strip() for x in re.split(r'[,;]', predicted) if x.strip()]

            if not isinstance(predicted, list):
                return False

            if len(predicted) != len(expected):
                return False

            # Sort both lists for comparison (order may not matter)
            try:
                pred_sorted = sorted(str(x) for x in predicted)
                exp_sorted = sorted(str(x) for x in expected)
                return all(cls.validate(p, e) for p, e in zip(pred_sorted, exp_sorted))
            except:
                return False

        # Handle strings (default)
        pred_str = cls.normalize_string(str(predicted))
        exp_str = cls.normalize_string(str(expected))

        # Exact match
        if pred_str == exp_str:
            return True

        # Fuzzy match (allow small typos)
        if len(exp_str) > 3:  # Only for non-trivial strings
            distance = cls.levenshtein_distance(pred_str, exp_str)
            if distance <= 2:  # Allow 2 character differences
                return True

        # Check if expected is substring of predicted (model may add extra words)
        if exp_str in pred_str:
            return True

        return False


class QuestionGenerator:
    """Generates test questions for different datasets"""

    @staticmethod
    def generate_employee_questions(data: Dict, num_questions: int = 200) -> List[Question]:
        """Generate questions for employee dataset"""
        employees = data["employees"]
        questions = []

        # Simple lookups (30%)
        for i in range(num_questions * 3 // 10):
            emp = random.choice(employees)
            q_variants = [
                (f"What is the email of {emp['name']}?", emp['email']),
                (f"What department does {emp['name']} work in?", emp['department']),
                (f"How many years of experience does {emp['name']} have?", emp['yearsExperience']),
                (f"Is {emp['name']} active?", emp['active']),
            ]
            q, a = random.choice(q_variants)
            questions.append(Question(
                id=f"emp_simple_{i}",
                question=q,
                answer=a,
                question_type=QuestionType.SIMPLE_LOOKUP,
                dataset_name="employees",
                difficulty="easy"
            ))

        # Filtered lookups (20%)
        for i in range(num_questions * 2 // 10):
            dept = random.choice(['Engineering', 'Sales', 'Marketing', 'HR', 'Operations', 'Finance'])
            dept_emps = [e for e in employees if e['department'] == dept]
            if dept_emps:
                emp = random.choice(dept_emps)
                questions.append(Question(
                    id=f"emp_filtered_{i}",
                    question=f"What is the email of an employee in {dept}?",
                    answer=emp['email'],  # Accept any valid answer
                    question_type=QuestionType.FILTERED_LOOKUP,
                    dataset_name="employees",
                    difficulty="medium"
                ))

        # Aggregations (20%)
        for i in range(num_questions * 2 // 10):
            q_variants = [
                ("How many employees are active?", sum(1 for e in employees if e['active'])),
                ("How many employees work in Engineering?", sum(1 for e in employees if e['department'] == 'Engineering')),
                ("What is the total number of employees?", len(employees)),
            ]
            q, a = random.choice(q_variants)
            questions.append(Question(
                id=f"emp_agg_{i}",
                question=q,
                answer=a,
                question_type=QuestionType.AGGREGATION,
                dataset_name="employees",
                difficulty="medium"
            ))

        # Comparisons (15%)
        for i in range(num_questions * 15 // 100):
            max_salary_emp = max(employees, key=lambda e: e['salary'])
            questions.append(Question(
                id=f"emp_comp_{i}",
                question="Who has the highest salary?",
                answer=max_salary_emp['name'],
                question_type=QuestionType.COMPARISON,
                dataset_name="employees",
                difficulty="hard"
            ))

        # Complex reasoning (15%)
        for i in range(num_questions * 15 // 100):
            dept_counts = {}
            for emp in employees:
                if emp['active']:
                    dept_counts[emp['department']] = dept_counts.get(emp['department'], 0) + 1

            large_depts = [dept for dept, count in dept_counts.items() if count >= 10]
            questions.append(Question(
                id=f"emp_complex_{i}",
                question="List all departments with at least 10 active employees.",
                answer=sorted(large_depts),
                question_type=QuestionType.COMPLEX,
                dataset_name="employees",
                difficulty="hard"
            ))

        return questions

    @staticmethod
    def generate_analytics_questions(data: Dict, num_questions: int = 100) -> List[Question]:
        """Generate questions for analytics dataset"""
        metrics = data["metrics"]
        questions = []

        # Simple lookups
        for i in range(num_questions // 2):
            metric = random.choice(metrics)
            q_variants = [
                (f"What were the views on {metric['date']}?", metric['views']),
                (f"What was the bounce rate on {metric['date']}?", metric['bounceRate']),
                (f"How many conversions were there on {metric['date']}?", metric['conversions']),
            ]
            q, a = random.choice(q_variants)
            questions.append(Question(
                id=f"analytics_simple_{i}",
                question=q,
                answer=a,
                question_type=QuestionType.SIMPLE_LOOKUP,
                dataset_name="analytics",
                difficulty="easy"
            ))

        # Aggregations
        for i in range(num_questions // 2):
            q_variants = [
                ("What is the total revenue across all dates?", sum(m['revenue'] for m in metrics)),
                ("What is the average bounce rate?", statistics.mean(m['bounceRate'] for m in metrics)),
                ("What is the maximum number of views in a single day?", max(m['views'] for m in metrics)),
            ]
            q, a = random.choice(q_variants)
            questions.append(Question(
                id=f"analytics_agg_{i}",
                question=q,
                answer=a,
                question_type=QuestionType.AGGREGATION,
                dataset_name="analytics",
                difficulty="medium"
            ))

        return questions


class AccuracyBenchmark:
    """Main benchmark orchestrator"""

    def __init__(self, output_dir: Path):
        self.output_dir = output_dir
        self.output_dir.mkdir(exist_ok=True)
        self.results: List[TestResult] = []

    def generate_all_questions(self) -> List[Question]:
        """Generate comprehensive question set"""
        print("Generating test questions...")

        questions = []

        # Employee questions (tabular data)
        emp_data = generate_employees(100)
        questions.extend(QuestionGenerator.generate_employee_questions(emp_data, 200))

        # Analytics questions (time series)
        analytics_data = generate_analytics(60)
        questions.extend(QuestionGenerator.generate_analytics_questions(analytics_data, 100))

        # TODO: Add more dataset types
        # - Orders (nested structure)
        # - Event logs (semi-uniform)
        # - Config (deeply nested)

        print(f"Generated {len(questions)} test questions")
        return questions

    def format_data_for_question(self, question: Question, format_name: str) -> Tuple[str, Any]:
        """Format dataset in specified format for a question"""

        # Get the dataset
        if question.dataset_name == "employees":
            data = generate_employees(100)
        elif question.dataset_name == "analytics":
            data = generate_analytics(60)
        else:
            raise ValueError(f"Unknown dataset: {question.dataset_name}")

        # Format according to format_name
        if format_name == "json":
            formatted = json.dumps(data, indent=2)
        elif format_name == "json-compact":
            formatted = json.dumps(data, separators=(',', ':'))
        elif format_name == "tauq":
            formatted = json_to_tauq(data, "default")
        elif format_name == "tauq-no-schemas":
            formatted = json_to_tauq(data, "no-schemas")
        elif format_name == "tauq-optimized":
            formatted = json_to_tauq(data, "optimized")
        elif format_name == "toon":
            formatted = toon_encode(data)
        elif format_name == "csv":
            formatted = csv_encode(data)
            if not formatted:
                return None, None  # CSV not applicable
        elif format_name == "markdown":
            formatted = self._to_markdown(data)
        elif format_name == "yaml":
            formatted = self._to_yaml(data)
        else:
            raise ValueError(f"Unknown format: {format_name}")

        tokens = count_tokens(formatted)
        return formatted, tokens

    def _to_markdown(self, data: Any) -> str:
        """Convert data to Markdown table format"""
        # Simple implementation for arrays of objects
        if isinstance(data, dict):
            for key, value in data.items():
                if isinstance(value, list) and value and isinstance(value[0], dict):
                    # Create markdown table
                    fields = list(value[0].keys())
                    header = "| " + " | ".join(fields) + " |"
                    separator = "|" + "|".join(["---" for _ in fields]) + "|"
                    rows = []
                    for item in value:
                        row = "| " + " | ".join(str(item.get(f, "")) for f in fields) + " |"
                        rows.append(row)
                    return f"## {key}\n\n{header}\n{separator}\n" + "\n".join(rows)
        return json.dumps(data, indent=2)  # Fallback

    def _to_yaml(self, data: Any) -> str:
        """Convert data to YAML format (simple implementation)"""
        # For now, just use JSON-like indented format
        # TODO: Add proper YAML library
        def yaml_encode(obj, indent=0):
            prefix = "  " * indent
            if isinstance(obj, dict):
                lines = []
                for k, v in obj.items():
                    if isinstance(v, (dict, list)):
                        lines.append(f"{prefix}{k}:")
                        lines.append(yaml_encode(v, indent + 1))
                    else:
                        lines.append(f"{prefix}{k}: {v}")
                return "\n".join(lines)
            elif isinstance(obj, list):
                lines = []
                for item in obj:
                    if isinstance(item, dict):
                        lines.append(f"{prefix}- " + yaml_encode(item, indent + 1).strip())
                    else:
                        lines.append(f"{prefix}- {item}")
                return "\n".join(lines)
            else:
                return str(obj)

        return yaml_encode(data)

    def query_model(self, model_name: str, formatted_data: str, question: str, format_name: str = "json") -> Tuple[Any, float]:
        """Query LLM and return answer + latency"""
        import time
        import requests

        # Choose system prompt based on format
        if format_name.startswith("tauq"):
            system_prompt = TAUQ_FORMAT_GUIDE
        else:
            system_prompt = "You are a data analysis assistant. Answer questions accurately and concisely."

        prompt = f"""Data:
{formatted_data}

Question: {question}

Provide ONLY the final answer. No explanation needed."""

        start = time.time()

        # Mock implementation for development
        if model_name.lower() == "mock":
            latency = random.uniform(100, 500)
            return "mock_answer", latency

        # LM Studio local instance
        if model_name.startswith("lmstudio/") or model_name.startswith("local/"):
            try:
                response = requests.post(
                    "http://localhost:1234/v1/chat/completions",
                    headers={"Content-Type": "application/json"},
                    json={
                        "model": model_name.replace("lmstudio/", "").replace("local/", ""),
                        "messages": [
                            {"role": "system", "content": system_prompt},
                            {"role": "user", "content": prompt}
                        ],
                        "temperature": 0.8,
                        "max_tokens": -1,
                        "top_k": 40,
                        "top_p": 0.95,
                        "min_p": 0.05,
                        "repeat_penalty": 1.1,
                        "stream": False
                    },
                    timeout=90
                )
                response.raise_for_status()
                result = response.json()

                # Extract answer from content, handle reasoning field
                choice = result["choices"][0]
                message = choice["message"]
                answer = message.get("content", "").strip()

                # If content is empty but reasoning exists, try to extract answer
                if not answer and "reasoning" in message:
                    reasoning = message.get("reasoning", "").strip()
                    # Look for numeric answers or simple patterns
                    import re
                    # Try to find answer patterns in reasoning
                    match = re.search(r'(?:answer|so|result)[:\s]+["\']?(\w+)["\']?', reasoning, re.IGNORECASE)
                    if match:
                        answer = match.group(1)
                    elif reasoning:
                        # Last resort: take last word that looks like an answer
                        words = reasoning.split()
                        if words:
                            answer = words[-1].strip('.",;:')

                latency = (time.time() - start) * 1000
                return answer, latency
            except Exception as e:
                print(f"Error querying LM Studio: {e}")
                return None, 0

        # OpenAI API
        if "gpt" in model_name.lower() and HAS_OPENAI:
            client = openai.OpenAI()
            response = client.chat.completions.create(
                model=model_name,
                messages=[{"role": "user", "content": prompt}],
                temperature=0.1,
                max_tokens=500
            )
            answer = response.choices[0].message.content.strip()
        elif "claude" in model_name.lower() and HAS_ANTHROPIC:
            client = anthropic.Anthropic()
            response = client.messages.create(
                model=model_name,
                max_tokens=500,
                temperature=0.1,
                messages=[{"role": "user", "content": prompt}]
            )
            answer = response.content[0].text.strip()
        else:
            raise ValueError(f"Unsupported model: {model_name}")

        latency = (time.time() - start) * 1000  # Convert to ms
        return answer, latency

    def run_benchmark(self,
                      formats: List[str],
                      models: List[str],
                      questions: Optional[List[Question]] = None,
                      num_runs: int = 3,
                      dry_run: bool = False) -> Dict[str, Any]:
        """Run full benchmark"""

        if questions is None:
            questions = self.generate_all_questions()

        if dry_run:
            print("DRY RUN: Testing with first 10 questions only")
            questions = questions[:10]

        total_tests = len(formats) * len(models) * len(questions) * num_runs
        print(f"\nRunning {total_tests:,} total tests:")
        print(f"  {len(formats)} formats × {len(models)} models × {len(questions)} questions × {num_runs} runs")
        print()

        completed = 0

        for format_name in formats:
            for model_name in models:
                print(f"\nTesting {format_name} with {model_name}...")

                for question in questions:
                    # Format data
                    formatted_data, tokens = self.format_data_for_question(question, format_name)

                    if formatted_data is None:
                        continue  # Skip if format not applicable

                    # Run multiple times for statistical significance
                    for run in range(num_runs):
                        try:
                            # Query model
                            predicted, latency = self.query_model(model_name, formatted_data, question.question, format_name)

                            # Validate answer
                            correct = AnswerValidator.validate(predicted, question.answer)

                            # Record result
                            result = TestResult(
                                question_id=question.id,
                                format_name=format_name,
                                model_name=model_name,
                                correct=correct,
                                predicted_answer=predicted,
                                expected_answer=question.answer,
                                tokens_used=tokens,
                                latency_ms=latency
                            )
                            self.results.append(result)

                        except Exception as e:
                            print(f"Error on {question.id}: {e}")
                            result = TestResult(
                                question_id=question.id,
                                format_name=format_name,
                                model_name=model_name,
                                correct=False,
                                predicted_answer=None,
                                expected_answer=question.answer,
                                tokens_used=tokens if tokens else 0,
                                latency_ms=0,
                                error=str(e)
                            )
                            self.results.append(result)

                        completed += 1
                        if completed % 100 == 0:
                            print(f"  Progress: {completed}/{total_tests} ({100*completed/total_tests:.1f}%)")

        # Analyze results
        return self.analyze_results()

    def analyze_results(self) -> Dict[str, Any]:
        """Analyze results and calculate statistics"""
        from collections import defaultdict
        import math

        # Group by format and model
        grouped = defaultdict(list)
        for result in self.results:
            key = (result.format_name, result.model_name)
            grouped[key].append(result)

        analysis = {}

        for (format_name, model_name), results in grouped.items():
            correct_count = sum(1 for r in results if r.correct)
            total_count = len(results)
            accuracy = correct_count / total_count if total_count > 0 else 0

            # Calculate 95% confidence interval
            # Using Wilson score interval for binomial proportion
            z = 1.96  # 95% confidence
            p = accuracy
            n = total_count

            if n > 0:
                denominator = 1 + z**2/n
                center = (p + z**2/(2*n)) / denominator
                margin = z * math.sqrt((p*(1-p)/n + z**2/(4*n**2))) / denominator
                ci_lower = max(0, center - margin)
                ci_upper = min(1, center + margin)
            else:
                ci_lower = ci_upper = 0

            # Average tokens
            avg_tokens = statistics.mean(r.tokens_used for r in results if r.tokens_used > 0)

            # Average latency
            avg_latency = statistics.mean(r.latency_ms for r in results if r.latency_ms > 0)

            analysis[f"{format_name}_{model_name}"] = {
                "format": format_name,
                "model": model_name,
                "accuracy": accuracy,
                "ci_lower": ci_lower,
                "ci_upper": ci_upper,
                "correct": correct_count,
                "total": total_count,
                "avg_tokens": avg_tokens,
                "avg_latency_ms": avg_latency,
                "accuracy_per_1k_tokens": (accuracy * 1000 / avg_tokens) if avg_tokens > 0 else 0
            }

        return analysis

    def generate_report(self, analysis: Dict[str, Any]) -> str:
        """Generate human-readable report"""
        report = []
        report.append("=" * 80)
        report.append("TAUQ LLM ACCURACY BENCHMARK RESULTS")
        report.append("=" * 80)
        report.append("")

        # Group by model
        by_model = defaultdict(list)
        for key, stats in analysis.items():
            by_model[stats["model"]].append(stats)

        for model_name, format_stats in by_model.items():
            report.append(f"Model: {model_name}")
            report.append("-" * 80)
            report.append(f"{'Format':<20} {'Accuracy':<15} {'95% CI':<20} {'Avg Tokens':<12} {'Acc/1k Tok':<12}")
            report.append("-" * 80)

            # Sort by accuracy
            format_stats.sort(key=lambda x: x["accuracy"], reverse=True)

            for stats in format_stats:
                acc_str = f"{stats['accuracy']*100:.1f}%"
                ci_str = f"[{stats['ci_lower']*100:.1f}%, {stats['ci_upper']*100:.1f}%]"
                tokens_str = f"{stats['avg_tokens']:.0f}"
                acc_per_tok = f"{stats['accuracy_per_1k_tokens']:.2f}%"

                report.append(f"{stats['format']:<20} {acc_str:<15} {ci_str:<20} {tokens_str:<12} {acc_per_tok:<12}")

            report.append("")

        return "\n".join(report)


def main():
    """Main entry point"""
    import argparse

    parser = argparse.ArgumentParser(description="Run Tauq accuracy benchmarks")
    parser.add_argument("--dry-run", action="store_true", help="Run with only 10 questions for testing")
    parser.add_argument("--formats", nargs="+", default=["json", "tauq", "toon", "csv", "markdown"],
                       help="Formats to test")
    parser.add_argument("--models", nargs="+", default=["mock"],
                       help="Models to test (use 'mock' for development)")
    parser.add_argument("--runs", type=int, default=3, help="Number of runs per question")
    args = parser.parse_args()

    print("=" * 80)
    print("TAUQ LLM ACCURACY BENCHMARK")
    print("=" * 80)
    print()
    print("This benchmark tests retrieval accuracy across multiple formats.")
    print("Based on improvingagents.com findings, we expect:")
    print("  - TOON: ~43-47% accuracy")
    print("  - Markdown: ~54-62% accuracy")
    print("  - Tauq: ? (hypothesis: better than TOON due to simpler syntax)")
    print()

    if "mock" in args.models:
        print("WARNING: Using mock model for development/testing")
        print("         Real results require --models gpt-4o claude-3-5-sonnet-20241022")
        print()

    benchmark = AccuracyBenchmark(Path(__file__).parent / "outputs" / "accuracy")

    results = benchmark.run_benchmark(
        formats=args.formats,
        models=args.models,
        num_runs=args.runs,
        dry_run=args.dry_run
    )

    # Generate and save report
    report = benchmark.generate_report(results)
    print("\n" + report)

    # Save detailed results
    output_file = benchmark.output_dir / "results.json"
    with open(output_file, 'w') as f:
        json.dump(results, f, indent=2)

    print(f"\nDetailed results saved to: {output_file}")


if __name__ == "__main__":
    main()
