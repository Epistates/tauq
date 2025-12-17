#!/usr/bin/env python3
"""Manual curl-style test following the user's template"""

import requests
import json

# Tauq format cheat sheet (comprehensive)
TAUQ_CHEATSHEET = """You are analyzing data in Tauq format - a token-efficient notation.

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

# Test data
JSON_DATA = {
    "employees": [
        {"id": 1, "name": "Alice", "department": "Engineering"},
        {"id": 2, "name": "Bob", "department": "Sales"},
        {"id": 3, "name": "Carol", "department": "Engineering"},
        {"id": 4, "name": "Dave", "department": "HR"},
        {"id": 5, "name": "Eve", "department": "Engineering"}
    ]
}

TAUQ_DATA = """!def Employee id name department
---
employees [
  !use Employee
  1 Alice Engineering
  2 Bob Sales
  3 Carol Engineering
  4 Dave HR
  5 Eve Engineering
]"""

TAUQ_NO_SCHEMAS_DATA = """employees [
  { id 1 name Alice department Engineering }
  { id 2 name Bob department Sales }
  { id 3 name Carol department Engineering }
  { id 4 name Dave department HR }
  { id 5 name Eve department Engineering }
]"""

def test_format(name, system_prompt, data_str, question, expected):
    """Test a specific format"""
    print(f"\n{'=' * 80}")
    print(f"TEST: {name}")
    print(f"{'=' * 80}")
    print(f"Question: {question}")
    print(f"Expected: {expected}")
    print(f"Data preview: {data_str[:200]}...")
    print()

    user_content = f"""Data:
{data_str}

Question: {question}

Provide ONLY the final answer. No explanation needed."""

    # Follow the user's curl template exactly with specified sampling params
    response = requests.post(
        "http://localhost:1234/v1/chat/completions",
        headers={"Content-Type": "application/json"},
        json={
            "model": "gpt-oss-120b",
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": user_content
                }
            ],
            "temperature": 0.8,
            "max_tokens": -1,
            "top_k": 40,
            "top_p": 0.95,
            "min_p": 0.05,
            "repeat_penalty": 1.1,
            "stream": False
        },
        timeout=60
    )

    result = response.json()

    if "error" in result:
        print(f"ERROR: {result['error']}")
        return

    # Extract response
    choice = result["choices"][0]
    message = choice["message"]
    content = message.get("content", "").strip()
    reasoning = message.get("reasoning", "").strip()
    finish_reason = choice.get("finish_reason", "unknown")

    # Show both content and reasoning
    print(f"Content: '{content}'")
    if reasoning:
        print(f"Reasoning: {reasoning[:150]}{'...' if len(reasoning) > 150 else ''}")
    print(f"Finish Reason: {finish_reason}")

    # Check correctness
    is_correct = content.lower() == str(expected).lower() or content == str(expected)
    print(f"\nResult: {'✓ CORRECT' if is_correct else '✗ WRONG'}")

    # Token usage
    usage = result.get("usage", {})
    print(f"Tokens: prompt={usage.get('prompt_tokens', '?')}, completion={usage.get('completion_tokens', '?')}")

# Run tests
print("=" * 80)
print("MANUAL CURL-STYLE TESTS (max_tokens=-1)")
print("=" * 80)

# Test 1: JSON
test_format(
    "JSON Format",
    "You are a data analysis assistant. Answer questions accurately and concisely.",
    json.dumps(JSON_DATA, indent=2),
    "How many employees are there?",
    "5"
)

# Test 2: Tauq with schemas + cheat sheet
test_format(
    "Tauq Format (with !def schemas + cheat sheet)",
    TAUQ_CHEATSHEET,
    TAUQ_DATA,
    "How many employees are there?",
    "5"
)

# Test 3: Tauq without schemas + cheat sheet
test_format(
    "Tauq Format (no schemas, inline objects + cheat sheet)",
    TAUQ_CHEATSHEET,
    TAUQ_NO_SCHEMAS_DATA,
    "How many employees are there?",
    "5"
)

# Test 4: Engineering count (more complex)
test_format(
    "Tauq Format - Complex Question",
    TAUQ_CHEATSHEET,
    TAUQ_DATA,
    "How many employees work in Engineering?",
    "3"
)

print(f"\n{'=' * 80}")
print("All tests complete!")
print(f"{'=' * 80}")
