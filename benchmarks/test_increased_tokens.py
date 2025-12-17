#!/usr/bin/env python3
"""Test with increased max_tokens to accommodate reasoning + answer"""

from benchmark_comprehensive import generate_employees, json_to_tauq
import json
import requests

# Generate test dataset
data = generate_employees(100)

# Format in different ways
formats = {
    "json": json.dumps(data, indent=2),
    "tauq": json_to_tauq(data, "default"),
    "tauq-no-schemas": json_to_tauq(data, "no-schemas"),
}

questions = [
    ("How many employees are there?", "100"),
    ("How many employees work in Engineering?", str(len([e for e in data["employees"] if e.get("department") == "Engineering"]))),
    ("What is the department of the employee with id 1?", "Engineering"),
]

# Tauq format cheat sheet
TAUQ_CHEATSHEET = """You are analyzing data in Tauq format.

Tauq Syntax Guide:
- `!def TypeName field1 field2 ...` defines a schema
- Lines after `!def` are data rows (one row per line)
- `!use TypeName` inside arrays activates a schema
- Count rows = count lines after the schema directive

Example:
!def User id name
1 Alice
2 Bob

This means: 2 users (2 lines of data)

When data is inside an array with `!use`:
users [
  !use User
  1 Alice
  2 Bob
]

Still 2 users (count the data lines)."""

print("=" * 80)
print("TESTING WITH INCREASED MAX_TOKENS (200 instead of 50)")
print("=" * 80)

for question, expected in questions:
    print(f"\n{'=' * 80}")
    print(f"Question: {question}")
    print(f"Expected: {expected}")
    print(f"{'=' * 80}")

    for fmt_name, fmt_data in formats.items():
        system_prompt = TAUQ_CHEATSHEET if fmt_name.startswith("tauq") else "You are a data analysis assistant."

        prompt = f"""Data:
{fmt_data}

Question: {question}

Provide ONLY the final answer. No explanation needed."""

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
                    "max_tokens": 200,  # Increased from 50
                    "stream": False
                },
                timeout=90
            )

            result = response.json()

            # Extract answer from content or reasoning
            if "choices" in result and len(result["choices"]) > 0:
                choice = result["choices"][0]
                message = choice["message"]

                # Try content first, fall back to reasoning
                answer = message.get("content", "").strip()
                reasoning = message.get("reasoning", "").strip()
                finish_reason = choice.get("finish_reason", "unknown")

                # If content is empty but reasoning exists, extract from reasoning
                if not answer and reasoning:
                    # Try to find just the number/answer at the end
                    import re
                    # Look for patterns like "So answer 100" or ending with number
                    match = re.search(r'answer\s+is\s+(\w+)|answer\s+(\w+)|So\s+(\w+)', reasoning, re.IGNORECASE)
                    if match:
                        answer = next(g for g in match.groups() if g)

                is_correct = (answer.lower() == expected.lower() or
                             answer == expected or
                             (expected.isdigit() and answer.isdigit() and answer == expected))

                indicator = "✓" if is_correct else "✗"
                print(f"{fmt_name:20} {indicator} '{answer}' (finish: {finish_reason})")

                if reasoning and not answer:
                    print(f"{'':20}   [Reasoning: {reasoning[:100]}...]")
            else:
                print(f"{fmt_name:20} ✗ No response")

        except Exception as e:
            print(f"{fmt_name:20} ✗ Error: {e}")

print(f"\n{'=' * 80}")
print("Done!")
print(f"{'=' * 80}")
