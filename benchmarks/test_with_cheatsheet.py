#!/usr/bin/env python3
"""Test LM Studio with Tauq format cheat sheet in system prompt"""

from accuracy_benchmark import *
from benchmark_comprehensive import generate_employees, json_to_tauq
import json
import requests

# Generate test dataset
data = generate_employees(100)
employees = data["employees"]

# Create test questions
questions = [
    ("How many employees are there?", 100),
    ("How many employees work in Engineering?", len([e for e in employees if e.get("department") == "Engineering"])),
    ("What is the department of the employee with id 1?", "Engineering"),
]

# Format data in different formats
formats = {
    "json": json.dumps(data, indent=2),
    "tauq": json_to_tauq(data, "default"),
    "tauq-no-schemas": json_to_tauq(data, "no-schemas"),
}

# Comprehensive Tauq format cheat sheet for LLM
TAUQ_CHEATSHEET = """
# Tauq Format Cheat Sheet

Tauq is a token-efficient data format. Here's how to read it:

## Basic Syntax
- **Key-value pairs**: `key value` (space-separated, no colons or commas)
  Example: `name Alice` means {"name": "Alice"}

- **Strings**: Can be quoted `"hello"` or barewords `hello`
- **Numbers**: Written as-is: `42`, `3.14`
- **Booleans**: `true`, `false`
- **Arrays**: Square brackets with space-separated elements: `[1 2 3]`

## Schema Definitions (!def and !use)

When you see `!def` followed by a name and field names:
```
!def User id name email
```
This defines a schema called "User" with fields: id, name, email

After `!def`, subsequent lines are DATA ROWS matching those fields:
```
!def User id name email
1 Alice alice@example.com
2 Bob bob@example.com
```
This means:
- Row 1: {"id": 1, "name": "Alice", "email": "alice@example.com"}
- Row 2: {"id": 2, "name": "Bob", "email": "bob@example.com"}

**IMPORTANT**:
- Each line after `!def` is ONE complete row with values in field order
- Count ALL lines following `!def` until you see another `!def`, `!use`, or end of data
- Each line = 1 record/object

## Schema in Arrays
Inside arrays, `!use` activates a schema:
```
!def User id name
---
users [
  !use User
  1 Alice
  2 Bob
]
```
The `---` separator ends the implicit schema scope.

## Reading Tauq Step-by-Step

1. If you see `!def Type field1 field2 ...`:
   - Remember the field names in order
   - Each following line is a row with values for those fields
   - Each row becomes one object: {field1: val1, field2: val2, ...}

2. Count objects by counting lines after `!def` (until next directive)

3. For inline objects without schemas:
   - `key value` means {"key": "value"}
   - `key [a b c]` means {"key": ["a", "b", "c"]}

## Example with 3 employees:
```
!def Employee id name department
1 Alice Engineering
2 Bob Sales
3 Carol Engineering
```

This is an array of 3 objects:
[
  {"id": 1, "name": "Alice", "department": "Engineering"},
  {"id": 2, "name": "Bob", "department": "Sales"},
  {"id": 3, "name": "Carol", "department": "Engineering"}
]

To answer "How many employees?": Count the lines after !def = 3
To answer "How many in Engineering?": Count lines where department field is "Engineering" = 2
""".strip()

# System prompt with cheat sheet
SYSTEM_PROMPT_WITH_CHEATSHEET = f"""You are a data analysis assistant.

{TAUQ_CHEATSHEET}

Your task: Answer questions by reading the provided data in its given format.

CRITICAL RULES:
1. Read the data carefully according to the format rules above
2. Give ONLY the final answer - no explanation, no reasoning, no work shown
3. For counting questions, count ALL matching records
4. For lookup questions, find the exact value requested
5. Be precise and accurate
""".strip()

SYSTEM_PROMPT_SIMPLE = "You are a data analysis assistant. Answer questions by reading the provided data. Give ONLY the final answer with no explanation."

print("=" * 80)
print("TESTING WITH VS WITHOUT TAUQ CHEAT SHEET")
print("=" * 80)

for question, expected in questions:
    print(f"\n{'=' * 80}")
    print(f"Question: {question}")
    print(f"Expected: {expected}")
    print(f"{'=' * 80}")

    for fmt_name, formatted_data in formats.items():
        print(f"\n{fmt_name.upper()}:")

        # Choose system prompt based on format
        if fmt_name.startswith("tauq"):
            system_prompt = SYSTEM_PROMPT_WITH_CHEATSHEET
        else:
            system_prompt = SYSTEM_PROMPT_SIMPLE

        prompt = f"""You are given data in a specific format. Answer the question based solely on the provided data.

Data:
{formatted_data}

Question: {question}

IMPORTANT: Provide ONLY the final answer. Do not show your work or reasoning. Just the answer."""

        try:
            response = requests.post(
                "http://localhost:1234/v1/chat/completions",
                headers={"Content-Type": "application/json"},
                json={
                    "model": "gpt-oss-120b",
                    "messages": [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": prompt}
                    ],
                    "temperature": 0.0,
                    "max_tokens": 50,
                    "stream": False
                },
                timeout=60
            )

            result = response.json()
            answer = result["choices"][0]["message"]["content"].strip()

            # Validate
            from accuracy_benchmark import AnswerValidator
            is_correct = AnswerValidator.validate(answer, expected)

            # Show result with indicator
            indicator = "✓" if is_correct else "✗"
            print(f"  {indicator} Model: '{answer}' | Expected: '{expected}'")

        except Exception as e:
            print(f"  Error: {e}")

print(f"\n{'=' * 80}")
print("Test complete!")
print(f"{'=' * 80}")
