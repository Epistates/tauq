#!/usr/bin/env python3
"""Debug script to see what the model is actually answering"""

from accuracy_benchmark import *
from benchmark_comprehensive import generate_employees, json_to_tauq
import json
import requests

# Generate small test dataset
data = generate_employees(5)

# Create a simple question
question = "How many employees are there?"
expected_answer = 5

# Test with each format
formats = {
    "json": json.dumps(data, indent=2),
    "tauq": json_to_tauq(data, "default"),
    "tauq-no-schemas": json_to_tauq(data, "no-schemas"),
}

print("="*80)
print("TESTING MODEL RESPONSES")
print("="*80)

for fmt_name, formatted_data in formats.items():
    print(f"\n{'='*80}")
    print(f"FORMAT: {fmt_name}")
    print(f"{'='*80}")
    print(f"Data (first 300 chars):\n{formatted_data[:300]}...")

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
                    {"role": "system", "content": "You are a data analysis assistant. Answer questions by reading the provided data. Give ONLY the final answer with no explanation."},
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

        # Try to validate
        from accuracy_benchmark import AnswerValidator
        is_correct = AnswerValidator.validate(answer, expected_answer)

        print(f"\nModel answer: '{answer}'")
        print(f"Expected: '{expected_answer}'")
        print(f"Correct: {is_correct}")

    except Exception as e:
        print(f"\nError: {e}")

print(f"\n{'='*80}")
print("Done!")
print(f"{'='*80}")
