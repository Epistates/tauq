#!/usr/bin/env python3
"""
Tauq Accuracy Benchmark - v2
Replicates improvingagents.com methodology exactly:
- 1,000 synthetic employee records with 8 attributes
- 1,000 lookup questions (retrieving specific field values)
- Simple numeric/string answers for deterministic validation
"""

import json
import random
import time
import requests
from typing import Any, Dict, List, Tuple
from pathlib import Path
from dataclasses import dataclass
import statistics

# Import utilities
from benchmark_comprehensive import json_to_tauq, count_tokens

# Seed for reproducibility
random.seed(42)

# Output directory
OUTPUT_DIR = Path("outputs/accuracy_v2")
OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

# Tauq format cheat sheet
TAUQ_FORMAT_GUIDE = """You are analyzing employee data in Tauq format - a token-efficient notation.

# Tauq Format Quick Reference

## Schemas (!def and !use)

**!def TypeName field1 field2 ...**
Defines a schema with field names.
After !def, each line is a DATA ROW with values matching the fields in order.

Example:
```
!def Employee id name age city
1 Alice 30 NYC
2 Bob 25 LA
```

This creates 2 Employee objects:
- Employee 1: {id: 1, name: "Alice", age: 30, city: "NYC"}
- Employee 2: {id: 2, name: "Bob", age: 25, city: "LA"}

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
]
```

## Finding Specific Records

To find an employee's data:
1. Look for their name or ID in the data rows
2. Read the values in field order
3. Match the field you need with its position

Example: "What is Alice's department?"
- Find row: `1 Alice Engineering`
- Fields are: id=1, name=Alice, department=Engineering
- Answer: Engineering

Your task: Read the data and answer questions with EXACT values only."""


@dataclass
class Employee:
    """Employee record matching improvingagents.com structure"""
    id: int
    name: str
    age: int
    city: str
    department: str
    salary: int
    experience: int  # years
    project_count: int


def generate_employee_dataset(count: int = 1000) -> List[Employee]:
    """Generate synthetic employee records"""

    first_names = ["Alice", "Bob", "Carol", "Dave", "Eve", "Frank", "Grace", "Hank", "Ivy", "Jack",
                   "Kate", "Liam", "Maya", "Noah", "Olivia", "Pete", "Quinn", "Rita", "Sam", "Tina"]

    cities = ["NYC", "LA", "Chicago", "Houston", "Phoenix", "Philadelphia", "San Antonio", "San Diego",
              "Dallas", "Austin", "Jacksonville", "San Jose", "Fort Worth", "Columbus", "Charlotte"]

    departments = ["Engineering", "Sales", "Marketing", "HR", "Finance", "Operations", "Support", "Legal"]

    employees = []

    for i in range(count):
        # Generate unique identifier (like "Alice W204")
        first_name = random.choice(first_names)
        suffix = f"{chr(65 + (i // 1000))}{i % 1000:03d}"  # A000, A001, etc.
        name = f"{first_name} {suffix}"

        employees.append(Employee(
            id=i + 1,
            name=name,
            age=random.randint(22, 65),
            city=random.choice(cities),
            department=random.choice(departments),
            salary=random.randint(40000, 180000),
            experience=random.randint(0, 30),
            project_count=random.randint(1, 50)
        ))

    return employees


def generate_lookup_questions(employees: List[Employee], count: int = 1000) -> List[Tuple[str, Any, str]]:
    """Generate lookup questions following improvingagents.com methodology

    Returns: List of (question, expected_answer, field_name)
    """
    questions = []
    fields = ["experience", "salary", "age", "city", "department", "project_count"]

    for _ in range(count):
        emp = random.choice(employees)
        field = random.choice(fields)

        if field == "experience":
            question = f"How many years of experience does {emp.name} have? (Return just the number, e.g. '12'.)"
            answer = emp.experience
        elif field == "salary":
            question = f"What is {emp.name}'s salary? (Return just the number, e.g. '85200'.)"
            answer = emp.salary
        elif field == "age":
            question = f"How old is {emp.name}? (Return just the number, e.g. '35'.)"
            answer = emp.age
        elif field == "city":
            question = f"What city does {emp.name} work in? (Return just the city name, e.g. 'NYC'.)"
            answer = emp.city
        elif field == "department":
            question = f"What department does {emp.name} work in? (Return just the department name, e.g. 'Engineering'.)"
            answer = emp.department
        elif field == "project_count":
            question = f"How many projects is {emp.name} working on? (Return just the number, e.g. '5'.)"
            answer = emp.project_count

        questions.append((question, answer, field))

    return questions


def format_employees_json(employees: List[Employee]) -> str:
    """Format as JSON (minified)"""
    data = [vars(emp) for emp in employees]
    return json.dumps(data)


def format_employees_tauq(employees: List[Employee], mode: str = "default") -> str:
    """Format as Tauq using CLI"""
    data = [vars(emp) for emp in employees]
    return json_to_tauq(data, mode)


def query_lmstudio(system_prompt: str, user_prompt: str, model: str = "gpt-oss-120b") -> Tuple[str, float]:
    """Query LM Studio with proper parameters"""
    start = time.time()

    try:
        response = requests.post(
            "http://localhost:1234/v1/chat/completions",
            headers={"Content-Type": "application/json"},
            json={
                "model": model,
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}
                ],
                "temperature": 0.1,  # Low temperature for deterministic answers
                "max_tokens": 2000,  # Enough for reasoning + answer
                "top_k": 40,
                "top_p": 0.95,
                "min_p": 0.05,
                "repeat_penalty": 1.1,
                "stream": False
            },
            timeout=120
        )
        response.raise_for_status()
        result = response.json()

        # Extract answer from content
        choice = result["choices"][0]
        message = choice["message"]
        answer = message.get("content", "").strip()

        # If content is empty, try reasoning field
        if not answer and "reasoning" in message:
            reasoning = message.get("reasoning", "").strip()
            # Extract last meaningful token
            if reasoning:
                words = reasoning.split()
                answer = words[-1].strip('.",;:\'"')

        latency = (time.time() - start) * 1000
        return answer, latency

    except Exception as e:
        print(f"Error querying LM Studio: {e}")
        return None, 0


def validate_answer(predicted: str, expected: Any) -> bool:
    """Validate answer with type-aware comparison"""
    if predicted is None:
        return False

    # Normalize strings
    pred_str = str(predicted).lower().strip().strip('",.:;\'')
    exp_str = str(expected).lower().strip()

    # Exact match
    if pred_str == exp_str:
        return True

    # For numbers, try parsing both
    if isinstance(expected, (int, float)):
        try:
            pred_num = float(pred_str.replace(',', ''))
            exp_num = float(exp_str)
            return abs(pred_num - exp_num) < 0.01
        except:
            pass

    # Substring match (model might add extra text)
    if exp_str in pred_str:
        return True

    return False


def run_benchmark(employees: List[Employee],
                  questions: List[Tuple[str, Any, str]],
                  formats: List[str],
                  sample_size: int = None) -> Dict[str, Any]:
    """Run the accuracy benchmark

    Args:
        employees: Employee dataset
        questions: List of (question, answer, field) tuples
        formats: List of format names to test
        sample_size: If set, only test first N questions (for dry runs)
    """

    if sample_size:
        questions = questions[:sample_size]
        print(f"DRY RUN: Testing with first {sample_size} questions")

    print(f"\n{'='*80}")
    print(f"TAUQ ACCURACY BENCHMARK (improvingagents.com methodology)")
    print(f"{'='*80}")
    print(f"Dataset: {len(employees)} employee records")
    print(f"Questions: {len(questions)} lookup queries")
    print(f"Formats: {', '.join(formats)}")
    print(f"{'='*80}\n")

    results = {}

    for format_name in formats:
        print(f"\n{'='*80}")
        print(f"Testing format: {format_name}")
        print(f"{'='*80}")

        # Format the data
        print("Formatting data...")
        if format_name == "json":
            formatted_data = format_employees_json(employees)
            system_prompt = "You are a data analysis assistant. Answer questions by reading employee data. Return ONLY the exact value requested, nothing else."
        elif format_name == "tauq":
            formatted_data = format_employees_tauq(employees, "default")
            system_prompt = TAUQ_FORMAT_GUIDE
        elif format_name == "tauq-no-schemas":
            formatted_data = format_employees_tauq(employees, "no-schemas")
            system_prompt = TAUQ_FORMAT_GUIDE
        else:
            print(f"Unknown format: {format_name}")
            continue

        # Count tokens
        tokens = count_tokens(formatted_data)
        print(f"Data size: {len(formatted_data):,} chars, {tokens:,} tokens")

        # Test questions
        correct = 0
        total = 0
        latencies = []
        errors = 0

        for i, (question, expected_answer, field) in enumerate(questions):
            user_prompt = f"{formatted_data}\n\n{question}"

            predicted_answer, latency = query_lmstudio(system_prompt, user_prompt)

            if predicted_answer is None:
                errors += 1
                continue

            is_correct = validate_answer(predicted_answer, expected_answer)

            if is_correct:
                correct += 1

            total += 1
            latencies.append(latency)

            # Progress update every 50 questions
            if (i + 1) % 50 == 0:
                current_accuracy = (correct / total * 100) if total > 0 else 0
                print(f"Progress: {i+1}/{len(questions)} | Accuracy: {current_accuracy:.1f}% | Avg latency: {statistics.mean(latencies):.0f}ms")

        # Calculate final metrics
        accuracy = correct / total if total > 0 else 0
        avg_latency = statistics.mean(latencies) if latencies else 0

        results[format_name] = {
            "format": format_name,
            "accuracy": accuracy,
            "correct": correct,
            "total": total,
            "errors": errors,
            "tokens": tokens,
            "avg_latency_ms": avg_latency,
            "accuracy_per_1k_tokens": (accuracy * 1000) / tokens if tokens > 0 else 0
        }

        print(f"\n{'='*80}")
        print(f"Results for {format_name}:")
        print(f"  Accuracy: {accuracy*100:.1f}% ({correct}/{total} correct)")
        print(f"  Errors: {errors}")
        print(f"  Avg latency: {avg_latency:.0f}ms")
        print(f"  Tokens: {tokens:,}")
        print(f"  Accuracy per 1k tokens: {results[format_name]['accuracy_per_1k_tokens']:.4f}")
        print(f"{'='*80}")

    return results


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Tauq Accuracy Benchmark v2")
    parser.add_argument("--employees", type=int, default=1000, help="Number of employee records")
    parser.add_argument("--questions", type=int, default=1000, help="Number of questions to generate")
    parser.add_argument("--formats", nargs="+", default=["json", "tauq", "tauq-no-schemas"])
    parser.add_argument("--dry-run", type=int, help="Test with N questions only (dry run)")
    parser.add_argument("--model", default="gpt-oss-120b", help="LM Studio model name")

    args = parser.parse_args()

    # Generate dataset
    print("Generating employee dataset...")
    employees = generate_employee_dataset(args.employees)

    print("Generating questions...")
    questions = generate_lookup_questions(employees, args.questions)

    # Run benchmark
    results = run_benchmark(
        employees,
        questions,
        args.formats,
        sample_size=args.dry_run
    )

    # Save results
    results_file = OUTPUT_DIR / "results.json"
    with open(results_file, "w") as f:
        json.dump(results, f, indent=2)

    print(f"\n{'='*80}")
    print("FINAL RESULTS")
    print(f"{'='*80}")
    print(f"\n{'Format':<20} {'Accuracy':<12} {'Tokens':<12} {'Acc/1k Tok':<12}")
    print("-" * 80)

    for format_name, data in results.items():
        print(f"{format_name:<20} {data['accuracy']*100:>6.1f}%      {data['tokens']:>10,}  {data['accuracy_per_1k_tokens']:>10.4f}")

    print(f"\nResults saved to: {results_file}")


if __name__ == "__main__":
    main()
