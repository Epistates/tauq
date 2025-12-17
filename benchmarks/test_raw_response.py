#!/usr/bin/env python3
"""Check raw responses from LM Studio to understand empty answers"""

from benchmark_comprehensive import generate_employees, json_to_tauq
import json
import requests

# Generate small test dataset
data = generate_employees(100)

# Format in tauq
tauq_data = json_to_tauq(data, "default")
tauq_no_schemas_data = json_to_tauq(data, "no-schemas")

print("=" * 80)
print("TAUQ FORMAT (first 500 chars):")
print("=" * 80)
print(tauq_data[:500])
print("...")

print("\n" + "=" * 80)
print("TAUQ NO-SCHEMAS FORMAT (first 500 chars):")
print("=" * 80)
print(tauq_no_schemas_data[:500])
print("...")

question = "How many employees are there?"

# Tauq format cheat sheet
TAUQ_CHEATSHEET = """You are analyzing data in Tauq format.

Tauq Format Guide:
- `!def TypeName field1 field2 ...` defines a schema with field names
- After `!def`, each line is ONE data row with values matching the fields
- Count rows by counting lines after `!def` (until next directive or end)

Example:
!def User id name
1 Alice
2 Bob
3 Carol

This means 3 users total.

IMPORTANT: Each line after !def = 1 complete record."""

for fmt_name, fmt_data in [("TAUQ", tauq_data), ("TAUQ-NO-SCHEMAS", tauq_no_schemas_data)]:
    print(f"\n{'=' * 80}")
    print(f"Testing {fmt_name}")
    print(f"{'=' * 80}")

    prompt = f"""Data:
{fmt_data}

Question: {question}

Answer with ONLY the number."""

    try:
        response = requests.post(
            "http://localhost:1234/v1/chat/completions",
            headers={"Content-Type": "application/json"},
            json={
                "model": "gpt-oss-120b",
                "messages": [
                    {"role": "system", "content": TAUQ_CHEATSHEET if fmt_name.startswith("TAUQ") else "You are a data analysis assistant. Answer questions concisely."},
                    {"role": "user", "content": prompt}
                ],
                "temperature": 0.0,
                "max_tokens": 50,
                "stream": False
            },
            timeout=90
        )

        result = response.json()

        print(f"\nHTTP Status: {response.status_code}")
        print(f"Full Response:")
        print(json.dumps(result, indent=2))

        if "choices" in result and len(result["choices"]) > 0:
            answer = result["choices"][0]["message"]["content"].strip()
            finish_reason = result["choices"][0].get("finish_reason", "unknown")
            print(f"\nExtracted Answer: '{answer}'")
            print(f"Finish Reason: {finish_reason}")
        else:
            print("\nNo choices in response!")

    except Exception as e:
        print(f"\nException: {e}")
        import traceback
        traceback.print_exc()

print(f"\n{'=' * 80}")
print("Done!")
print(f"{'=' * 80}")
