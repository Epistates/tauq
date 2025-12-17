#!/usr/bin/env python3
"""
Tauq LLM Accuracy Benchmark
============================
A comprehensive benchmark comparing data format accuracy for LLM understanding.

Methodology based on: https://www.improvingagents.com/blog/best-input-data-format-for-llms

Key Features:
- 1000 synthetic records with 8 attributes (matching improvingagents.com)
- 1000 randomized lookup questions (simple value retrieval)
- Deterministic validation with fuzzy matching
- Statistical analysis with 95% confidence intervals
- Support for: JSON, Tauq, CSV, Markdown-KV, YAML, Markdown-Table
- Format-specific system prompts with cheatsheets
- LM Studio integration (OpenAI-compatible API)

Usage:
    # Quick test (10 questions, single format)
    python3 llm_accuracy_benchmark.py --dry-run 10 --formats tauq

    # Full benchmark
    python3 llm_accuracy_benchmark.py --formats json tauq csv markdown-kv yaml

    # Custom settings
    python3 llm_accuracy_benchmark.py --records 500 --questions 500 --runs 3
"""

import json
import random
import time
import statistics
import math
import csv
import io
import subprocess
import tempfile
import os
import argparse
from typing import Any, Dict, List, Tuple, Optional
from dataclasses import dataclass, field, asdict
from pathlib import Path
from datetime import datetime

try:
    import requests
except ImportError:
    print("ERROR: requests not installed. Run: pip install requests")
    exit(1)

try:
    import tiktoken
    ENCODER = tiktoken.get_encoding("o200k_base")
except ImportError:
    print("WARNING: tiktoken not installed. Token counting will estimate.")
    ENCODER = None


# ============================================================================
# Configuration
# ============================================================================

@dataclass
class BenchmarkConfig:
    """Configuration for the benchmark."""
    num_records: int = 1000
    num_questions: int = 1000
    num_runs: int = 1
    seed: int = 42
    output_dir: Path = field(default_factory=lambda: Path("outputs/accuracy"))

    # LM Studio settings
    lm_studio_url: str = "http://localhost:1234/v1/chat/completions"
    model: str = "local-model"
    temperature: float = 0.0  # Deterministic for reproducibility
    max_tokens: int = 100  # Short answers only
    timeout: int = 120

    # Validation settings
    numeric_tolerance: float = 0.01  # 1% tolerance for numeric comparisons
    string_case_sensitive: bool = False


# ============================================================================
# Data Generation (matching improvingagents.com schema)
# ============================================================================

@dataclass
class Employee:
    """Employee record matching improvingagents.com structure."""
    id: int
    name: str
    age: int
    city: str
    department: str
    salary: int
    experience: int  # years
    project_count: int


def generate_employees(count: int, seed: int = 42) -> List[Employee]:
    """Generate synthetic employee dataset."""
    random.seed(seed)

    first_names = [
        "Alice", "Bob", "Carol", "Dave", "Eve", "Frank", "Grace", "Hank",
        "Ivy", "Jack", "Kate", "Liam", "Maya", "Noah", "Olivia", "Pete",
        "Quinn", "Rita", "Sam", "Tina", "Uma", "Victor", "Wendy", "Xavier",
        "Yara", "Zack"
    ]

    cities = [
        "NYC", "LA", "Chicago", "Houston", "Phoenix", "Philadelphia",
        "San Antonio", "San Diego", "Dallas", "Austin", "Jacksonville",
        "San Jose", "Fort Worth", "Columbus", "Charlotte", "Seattle",
        "Denver", "Boston", "Detroit", "Portland"
    ]

    departments = [
        "Engineering", "Sales", "Marketing", "HR", "Finance",
        "Operations", "Support", "Legal", "Product", "Design"
    ]

    employees = []
    for i in range(count):
        # Generate unique identifier (like "Alice X413" from improvingagents.com)
        first_name = random.choice(first_names)
        suffix = f"{chr(65 + (i // 1000) % 26)}{i % 1000:03d}"
        name = f"{first_name} {suffix}"

        employees.append(Employee(
            id=i + 1,
            name=name,
            age=random.randint(22, 65),
            city=random.choice(cities),
            department=random.choice(departments),
            salary=random.randint(40000, 180000),
            experience=random.randint(0, 35),
            project_count=random.randint(1, 50)
        ))

    return employees


# ============================================================================
# Question Generation
# ============================================================================

@dataclass
class Question:
    """A benchmark question with expected answer."""
    text: str
    expected_answer: Any
    field_name: str
    employee_name: str
    answer_type: str  # "number", "string"


def generate_questions(employees: List[Employee], count: int, seed: int = 42) -> List[Question]:
    """Generate lookup questions following improvingagents.com methodology."""
    random.seed(seed + 1)  # Different seed from employee generation

    questions = []
    field_templates = {
        "experience": (
            "How many years of experience does {name} have?",
            lambda e: e.experience,
            "number"
        ),
        "salary": (
            "What is {name}'s salary?",
            lambda e: e.salary,
            "number"
        ),
        "age": (
            "How old is {name}?",
            lambda e: e.age,
            "number"
        ),
        "city": (
            "What city does {name} work in?",
            lambda e: e.city,
            "string"
        ),
        "department": (
            "What department does {name} work in?",
            lambda e: e.department,
            "string"
        ),
        "project_count": (
            "How many projects is {name} working on?",
            lambda e: e.project_count,
            "number"
        ),
    }

    field_names = list(field_templates.keys())

    for _ in range(count):
        emp = random.choice(employees)
        field = random.choice(field_names)
        template, getter, answer_type = field_templates[field]

        questions.append(Question(
            text=template.format(name=emp.name),
            expected_answer=getter(emp),
            field_name=field,
            employee_name=emp.name,
            answer_type=answer_type
        ))

    return questions


# ============================================================================
# Format Converters
# ============================================================================

def count_tokens(text: str) -> int:
    """Count tokens using tiktoken o200k_base (GPT-4o, Claude 3.5+)."""
    if ENCODER:
        return len(ENCODER.encode(text))
    # Fallback: estimate ~4 chars per token
    return len(text) // 4


def to_json(employees: List[Employee]) -> str:
    """Convert to minified JSON."""
    data = [asdict(e) for e in employees]
    return json.dumps(data, separators=(',', ':'))


def to_tauq(employees: List[Employee]) -> str:
    """Convert to Tauq format using the CLI."""
    data = [asdict(e) for e in employees]

    tauq_bin = Path(__file__).parent.parent / "target" / "release" / "tauq"
    if not tauq_bin.exists():
        # Try debug build
        tauq_bin = Path(__file__).parent.parent / "target" / "debug" / "tauq"

    if not tauq_bin.exists():
        raise RuntimeError("Tauq binary not found. Run: cargo build --release")

    with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
        json.dump(data, f)
        json_path = f.name

    try:
        result = subprocess.run(
            [str(tauq_bin), "format", json_path],
            capture_output=True,
            text=True,
            check=True
        )
        return result.stdout
    finally:
        os.unlink(json_path)


def to_csv(employees: List[Employee]) -> str:
    """Convert to CSV format."""
    output = io.StringIO()
    fieldnames = ['id', 'name', 'age', 'city', 'department', 'salary', 'experience', 'project_count']
    writer = csv.DictWriter(output, fieldnames=fieldnames)
    writer.writeheader()
    for emp in employees:
        writer.writerow(asdict(emp))
    return output.getvalue()


def to_markdown_kv(employees: List[Employee]) -> str:
    """Convert to Markdown key-value format (best performer in improvingagents.com)."""
    lines = []
    for emp in employees:
        lines.append(f"## Employee {emp.id}")
        lines.append(f"- **id**: {emp.id}")
        lines.append(f"- **name**: {emp.name}")
        lines.append(f"- **age**: {emp.age}")
        lines.append(f"- **city**: {emp.city}")
        lines.append(f"- **department**: {emp.department}")
        lines.append(f"- **salary**: {emp.salary}")
        lines.append(f"- **experience**: {emp.experience}")
        lines.append(f"- **project_count**: {emp.project_count}")
        lines.append("")
    return "\n".join(lines)


def to_markdown_table(employees: List[Employee]) -> str:
    """Convert to Markdown table format."""
    lines = [
        "| id | name | age | city | department | salary | experience | project_count |",
        "|---|---|---|---|---|---|---|---|"
    ]
    for emp in employees:
        lines.append(
            f"| {emp.id} | {emp.name} | {emp.age} | {emp.city} | {emp.department} | "
            f"{emp.salary} | {emp.experience} | {emp.project_count} |"
        )
    return "\n".join(lines)


def to_yaml(employees: List[Employee]) -> str:
    """Convert to YAML format."""
    lines = ["employees:"]
    for emp in employees:
        lines.append(f"  - id: {emp.id}")
        lines.append(f"    name: \"{emp.name}\"")
        lines.append(f"    age: {emp.age}")
        lines.append(f"    city: \"{emp.city}\"")
        lines.append(f"    department: \"{emp.department}\"")
        lines.append(f"    salary: {emp.salary}")
        lines.append(f"    experience: {emp.experience}")
        lines.append(f"    project_count: {emp.project_count}")
    return "\n".join(lines)


FORMAT_CONVERTERS = {
    "json": to_json,
    "tauq": to_tauq,
    "csv": to_csv,
    "markdown-kv": to_markdown_kv,
    "markdown-table": to_markdown_table,
    "yaml": to_yaml,
}


# ============================================================================
# System Prompts (Format-Specific Cheatsheets)
# ============================================================================

SYSTEM_PROMPTS = {
    "json": """You are a data analyst. You will receive employee data in JSON format and answer questions about it.

The data is an array of employee objects. Each employee has:
- id: Employee ID number
- name: Employee name (e.g., "Alice A001")
- age: Employee age in years
- city: City where they work
- department: Department name
- salary: Annual salary
- experience: Years of experience
- project_count: Number of active projects

Answer questions with ONLY the exact value requested. Do not include any explanation or units.

Examples:
- Q: "How old is Bob B042?" A: "35"
- Q: "What city does Alice A001 work in?" A: "NYC"
- Q: "What is Carol C123's salary?" A: "85000"
""",

    "tauq": """You are a data analyst answering questions about employee data in Tauq format.

# TAUQ FORMAT

The data has a schema line then data rows. Each row has 8 space-separated values:

!def Employee id name age city department salary experience project_count

Values in each row map to these positions:
  pos1=id  pos2=name  pos3=age  pos4=city  pos5=department  pos6=salary  pos7=experience  pos8=project_count

Example row:
  1 "Alice A001" 30 NYC Engineering 85000 5 10
  means: id=1, name="Alice A001", age=30, city=NYC, department=Engineering, salary=85000, experience=5, project_count=10

# HOW TO ANSWER

1. Find the row containing the person's name (in quotes)
2. Use this mapping to get the value:
   - age → position 3
   - city → position 4
   - department → position 5
   - salary → position 6
   - experience → position 7
   - project_count → position 8

Answer with ONLY the value. No explanation.

Examples:
- Q: "How old is Bob B002?" (Find "Bob B002" row, position 3) → A: 28
- Q: "What city does Alice A001 work in?" (position 4) → A: NYC
- Q: "What is Alice A001's salary?" (position 6) → A: 85000
""",

    "csv": """You are a data analyst. You will receive employee data in CSV format and answer questions about it.

The CSV has a header row followed by data rows. The columns are:
- id: Employee ID number
- name: Employee name (e.g., "Alice A001")
- age: Employee age in years
- city: City where they work
- department: Department name
- salary: Annual salary
- experience: Years of experience
- project_count: Number of active projects

To find a value:
1. Locate the row with the employee's name
2. Find the column for the requested field
3. Return the value at that intersection

Answer questions with ONLY the exact value requested. Do not include any explanation or units.

Examples:
- Q: "How old is Bob B042?" A: "35"
- Q: "What city does Alice A001 work in?" A: "NYC"
- Q: "What is Carol C123's salary?" A: "85000"
""",

    "markdown-kv": """You are a data analyst. You will receive employee data in Markdown key-value format and answer questions about it.

Each employee is listed under their own section with key-value pairs:
- **id**: Employee ID number
- **name**: Employee name
- **age**: Employee age in years
- **city**: City where they work
- **department**: Department name
- **salary**: Annual salary
- **experience**: Years of experience
- **project_count**: Number of active projects

To find a value:
1. Find the section containing the employee's name
2. Locate the key-value pair for the requested field
3. Return the value after the colon

Answer questions with ONLY the exact value requested. Do not include any explanation or units.

Examples:
- Q: "How old is Bob B042?" A: "35"
- Q: "What city does Alice A001 work in?" A: "NYC"
- Q: "What is Carol C123's salary?" A: "85000"
""",

    "markdown-table": """You are a data analyst. You will receive employee data in a Markdown table and answer questions about it.

The table columns are:
| id | name | age | city | department | salary | experience | project_count |

To find a value:
1. Locate the row with the employee's name
2. Find the column for the requested field
3. Return the value at that intersection

Answer questions with ONLY the exact value requested. Do not include any explanation or units.

Examples:
- Q: "How old is Bob B042?" A: "35"
- Q: "What city does Alice A001 work in?" A: "NYC"
- Q: "What is Carol C123's salary?" A: "85000"
""",

    "yaml": """You are a data analyst. You will receive employee data in YAML format and answer questions about it.

The data is a list of employees under the `employees` key. Each employee has:
- id: Employee ID number
- name: Employee name (in quotes)
- age: Employee age in years
- city: City where they work (in quotes)
- department: Department name (in quotes)
- salary: Annual salary
- experience: Years of experience
- project_count: Number of active projects

To find a value:
1. Find the employee entry with the matching name
2. Locate the field line for the requested attribute
3. Return the value after the colon

Answer questions with ONLY the exact value requested. Do not include any explanation or units.

Examples:
- Q: "How old is Bob B042?" A: "35"
- Q: "What city does Alice A001 work in?" A: "NYC"
- Q: "What is Carol C123's salary?" A: "85000"
""",
}


# ============================================================================
# LM Studio Client
# ============================================================================

@dataclass
class LLMResponse:
    """Response from LLM."""
    answer: str
    latency_ms: float
    tokens_prompt: int
    tokens_completion: int
    error: Optional[str] = None


def query_lm_studio(
    system_prompt: str,
    user_prompt: str,
    config: BenchmarkConfig
) -> LLMResponse:
    """Query LM Studio with OpenAI-compatible API."""
    start = time.time()

    try:
        response = requests.post(
            config.lm_studio_url,
            headers={"Content-Type": "application/json"},
            json={
                "model": config.model,
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}
                ],
                "temperature": config.temperature,
                "max_tokens": config.max_tokens,
                "stream": False
            },
            timeout=config.timeout
        )
        response.raise_for_status()
        result = response.json()

        latency = (time.time() - start) * 1000

        # Extract answer
        choice = result.get("choices", [{}])[0]
        message = choice.get("message", {})
        answer = message.get("content", "").strip()

        # Get token counts if available
        usage = result.get("usage", {})

        return LLMResponse(
            answer=answer,
            latency_ms=latency,
            tokens_prompt=usage.get("prompt_tokens", 0),
            tokens_completion=usage.get("completion_tokens", 0)
        )

    except requests.exceptions.ConnectionError:
        return LLMResponse(
            answer="",
            latency_ms=0,
            tokens_prompt=0,
            tokens_completion=0,
            error="Connection refused - is LM Studio running?"
        )
    except Exception as e:
        return LLMResponse(
            answer="",
            latency_ms=(time.time() - start) * 1000,
            tokens_prompt=0,
            tokens_completion=0,
            error=str(e)
        )


# ============================================================================
# Answer Validation
# ============================================================================

def normalize_string(s: str) -> str:
    """Normalize string for comparison."""
    return s.lower().strip().strip('"\'.,;:')


def validate_answer(
    predicted: str,
    expected: Any,
    answer_type: str,
    config: BenchmarkConfig
) -> bool:
    """Validate answer with type-aware comparison."""
    if not predicted:
        return False

    pred_normalized = normalize_string(predicted)
    exp_normalized = normalize_string(str(expected))

    # Exact match
    if pred_normalized == exp_normalized:
        return True

    # Numeric validation
    if answer_type == "number":
        try:
            pred_num = float(pred_normalized.replace(',', ''))
            exp_num = float(expected)
            # Check within tolerance
            if exp_num == 0:
                return abs(pred_num) < config.numeric_tolerance
            return abs(pred_num - exp_num) / abs(exp_num) <= config.numeric_tolerance
        except (ValueError, TypeError):
            pass

    # String substring match (model might include extra text)
    if exp_normalized in pred_normalized:
        return True

    return False


# ============================================================================
# Statistical Analysis
# ============================================================================

def wilson_score_interval(successes: int, total: int, confidence: float = 0.95) -> Tuple[float, float]:
    """Calculate Wilson score confidence interval for binomial proportion."""
    if total == 0:
        return (0.0, 0.0)

    # Z-score for confidence level (95% = 1.96)
    z = {0.90: 1.645, 0.95: 1.96, 0.99: 2.576}.get(confidence, 1.96)

    p = successes / total
    denominator = 1 + z**2 / total
    center = (p + z**2 / (2 * total)) / denominator
    margin = z * math.sqrt((p * (1 - p) + z**2 / (4 * total)) / total) / denominator

    return (max(0, center - margin), min(1, center + margin))


@dataclass
class FormatResult:
    """Results for a single format."""
    format_name: str
    total_questions: int
    correct: int
    accuracy: float
    ci_lower: float
    ci_upper: float
    total_tokens: int
    avg_latency_ms: float
    accuracy_per_1k_tokens: float
    errors: int

    def to_dict(self) -> Dict:
        return asdict(self)


# ============================================================================
# Benchmark Runner
# ============================================================================

class AccuracyBenchmark:
    """Main benchmark orchestrator."""

    def __init__(self, config: BenchmarkConfig):
        self.config = config
        self.employees: List[Employee] = []
        self.questions: List[Question] = []
        self.formatted_data: Dict[str, str] = {}
        self.results: Dict[str, FormatResult] = {}

    def setup(self):
        """Generate employees and questions."""
        print(f"Generating {self.config.num_records} employees...")
        self.employees = generate_employees(
            self.config.num_records,
            self.config.seed
        )

        print(f"Generating {self.config.num_questions} questions...")
        self.questions = generate_questions(
            self.employees,
            self.config.num_questions,
            self.config.seed
        )

        print(f"Questions distribution by field:")
        field_counts = {}
        for q in self.questions:
            field_counts[q.field_name] = field_counts.get(q.field_name, 0) + 1
        for field, count in sorted(field_counts.items()):
            print(f"  {field}: {count}")

    def format_data(self, formats: List[str]):
        """Convert employee data to all requested formats."""
        print("\nFormatting data...")
        for fmt in formats:
            if fmt not in FORMAT_CONVERTERS:
                print(f"  WARNING: Unknown format '{fmt}', skipping")
                continue

            try:
                self.formatted_data[fmt] = FORMAT_CONVERTERS[fmt](self.employees)
                tokens = count_tokens(self.formatted_data[fmt])
                print(f"  {fmt}: {len(self.formatted_data[fmt]):,} chars, {tokens:,} tokens")
            except Exception as e:
                print(f"  ERROR formatting {fmt}: {e}")

    def run_format(self, format_name: str) -> FormatResult:
        """Run benchmark for a single format."""
        data = self.formatted_data[format_name]
        system_prompt = SYSTEM_PROMPTS.get(format_name, SYSTEM_PROMPTS["json"])
        data_tokens = count_tokens(data)

        correct = 0
        total = 0
        errors = 0
        latencies = []

        for i, question in enumerate(self.questions):
            user_prompt = f"{data}\n\nQuestion: {question.text}\nAnswer:"

            response = query_lm_studio(system_prompt, user_prompt, self.config)

            if response.error:
                errors += 1
                if errors == 1:
                    print(f"\n  ERROR: {response.error}")
                if errors >= 3:
                    print(f"\n  Too many errors, stopping {format_name}")
                    break
                continue

            is_correct = validate_answer(
                response.answer,
                question.expected_answer,
                question.answer_type,
                self.config
            )

            if is_correct:
                correct += 1
            total += 1
            latencies.append(response.latency_ms)

            # Progress update
            if (i + 1) % 100 == 0 or i == len(self.questions) - 1:
                acc = correct / total if total > 0 else 0
                avg_lat = statistics.mean(latencies) if latencies else 0
                print(f"  Progress: {i+1}/{len(self.questions)} | Accuracy: {acc:.1%} | Avg latency: {avg_lat:.0f}ms")

        # Calculate final metrics
        accuracy = correct / total if total > 0 else 0
        ci_lower, ci_upper = wilson_score_interval(correct, total)
        avg_latency = statistics.mean(latencies) if latencies else 0
        acc_per_1k = (accuracy * 1000) / data_tokens if data_tokens > 0 else 0

        return FormatResult(
            format_name=format_name,
            total_questions=total,
            correct=correct,
            accuracy=accuracy,
            ci_lower=ci_lower,
            ci_upper=ci_upper,
            total_tokens=data_tokens,
            avg_latency_ms=avg_latency,
            accuracy_per_1k_tokens=acc_per_1k,
            errors=errors
        )

    def run(self, formats: List[str]) -> Dict[str, FormatResult]:
        """Run complete benchmark."""
        self.setup()
        self.format_data(formats)

        print("\n" + "=" * 80)
        print("RUNNING LLM ACCURACY BENCHMARK")
        print("=" * 80)
        print(f"Model: {self.config.model}")
        print(f"Records: {self.config.num_records}")
        print(f"Questions: {self.config.num_questions}")
        print(f"Formats: {', '.join(formats)}")
        print("=" * 80)

        for fmt in formats:
            if fmt not in self.formatted_data:
                continue

            print(f"\n{'='*80}")
            print(f"Testing format: {fmt}")
            print(f"{'='*80}")

            for run_num in range(self.config.num_runs):
                if self.config.num_runs > 1:
                    print(f"\n--- Run {run_num + 1}/{self.config.num_runs} ---")

                result = self.run_format(fmt)

                # Store best result (or only result)
                if fmt not in self.results or result.accuracy > self.results[fmt].accuracy:
                    self.results[fmt] = result

                print(f"\n  Results: {result.accuracy:.1%} accuracy ({result.correct}/{result.total_questions})")
                print(f"  95% CI: [{result.ci_lower:.1%}, {result.ci_upper:.1%}]")
                print(f"  Tokens: {result.total_tokens:,}")
                print(f"  Acc/1k tokens: {result.accuracy_per_1k_tokens:.4f}")

        return self.results

    def save_results(self):
        """Save results to JSON and generate report."""
        self.config.output_dir.mkdir(parents=True, exist_ok=True)

        # Save raw results
        results_data = {
            "config": {
                "num_records": self.config.num_records,
                "num_questions": self.config.num_questions,
                "num_runs": self.config.num_runs,
                "seed": self.config.seed,
                "model": self.config.model,
                "timestamp": datetime.now().isoformat()
            },
            "results": {name: result.to_dict() for name, result in self.results.items()}
        }

        results_file = self.config.output_dir / "results.json"
        with open(results_file, 'w') as f:
            json.dump(results_data, f, indent=2)

        print(f"\nResults saved to: {results_file}")

    def print_summary(self):
        """Print formatted summary table."""
        print("\n" + "=" * 100)
        print("FINAL RESULTS")
        print("=" * 100)

        # Header
        print(f"{'Format':<15} {'Accuracy':<12} {'95% CI':<20} {'Tokens':<10} {'Acc/1k Tok':<12}")
        print("-" * 100)

        # Sort by accuracy descending
        sorted_results = sorted(self.results.values(), key=lambda r: r.accuracy, reverse=True)

        for r in sorted_results:
            ci_str = f"[{r.ci_lower:.1%}, {r.ci_upper:.1%}]"
            print(f"{r.format_name:<15} {r.accuracy:>6.1%}       {ci_str:<20} {r.total_tokens:>10,} {r.accuracy_per_1k_tokens:>12.4f}")

        print("=" * 100)

        # Winner analysis
        if sorted_results:
            winner = sorted_results[0]
            print(f"\nBest accuracy: {winner.format_name} at {winner.accuracy:.1%}")

            # Token efficiency winner
            token_winner = min(sorted_results, key=lambda r: r.total_tokens)
            print(f"Most token-efficient: {token_winner.format_name} with {token_winner.total_tokens:,} tokens")

            # Best accuracy per token
            eff_winner = max(sorted_results, key=lambda r: r.accuracy_per_1k_tokens)
            print(f"Best accuracy/token ratio: {eff_winner.format_name} at {eff_winner.accuracy_per_1k_tokens:.4f}")


# ============================================================================
# Main Entry Point
# ============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Tauq LLM Accuracy Benchmark",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    # Quick test
    python3 llm_accuracy_benchmark.py --dry-run 10 --formats tauq json

    # Full benchmark with all formats
    python3 llm_accuracy_benchmark.py --formats json tauq csv markdown-kv yaml

    # Custom settings
    python3 llm_accuracy_benchmark.py --records 500 --questions 500 --runs 3
        """
    )

    parser.add_argument(
        "--formats",
        nargs="+",
        default=["json", "tauq", "csv", "markdown-kv"],
        choices=["json", "tauq", "csv", "markdown-kv", "markdown-table", "yaml"],
        help="Formats to benchmark"
    )
    parser.add_argument("--records", type=int, default=1000, help="Number of employee records")
    parser.add_argument("--questions", type=int, default=1000, help="Number of questions")
    parser.add_argument("--runs", type=int, default=1, help="Number of runs per format")
    parser.add_argument("--dry-run", type=int, help="Quick test with N questions")
    parser.add_argument("--model", default="local-model", help="LM Studio model name")
    parser.add_argument("--url", default="http://localhost:1234/v1/chat/completions", help="LM Studio API URL")
    parser.add_argument("--seed", type=int, default=42, help="Random seed for reproducibility")
    parser.add_argument("--output", type=str, default="outputs/accuracy", help="Output directory")

    args = parser.parse_args()

    # Build config
    config = BenchmarkConfig(
        num_records=args.records,
        num_questions=args.dry_run if args.dry_run else args.questions,
        num_runs=args.runs,
        seed=args.seed,
        model=args.model,
        lm_studio_url=args.url,
        output_dir=Path(args.output)
    )

    if args.dry_run:
        print(f"DRY RUN MODE: Testing with {args.dry_run} questions only")

    # Run benchmark
    benchmark = AccuracyBenchmark(config)
    benchmark.run(args.formats)
    benchmark.print_summary()
    benchmark.save_results()


if __name__ == "__main__":
    main()
