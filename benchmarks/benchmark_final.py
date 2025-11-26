#!/usr/bin/env python3
"""
Rigorous Apples-to-Apples Benchmark: tauq vs TOON vs JSON
Uses tiktoken cl100k_base (GPT-4/Claude tokenizer) for accurate token counts.

TOON encoding follows spec v3.0 from https://toonformat.dev/reference/spec.html
- UTF-8, LF line endings
- 2-space indentation
- Array header: key[N]{fields}: or [N]{fields}: for top-level
- Comma-separated values in tabular rows
- Canonical number formatting
"""

import json
import subprocess
import tempfile
import os
from pathlib import Path
from typing import Any
import tiktoken

# Initialize tokenizer (cl100k_base is used by GPT-4 and Claude)
ENCODER = tiktoken.get_encoding("cl100k_base")

# Path to tauq binary
TAUQ_BIN = "/Users/nickpaterno/work/tauq/target/release/tauq"


def count_tokens(text: str) -> int:
    """Count tokens using tiktoken cl100k_base."""
    return len(ENCODER.encode(text))


def json_to_tauq(data: dict | list) -> str:
    """Convert JSON to tauq format using the tauq CLI."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
        json.dump(data, f)
        json_path = f.name

    try:
        result = subprocess.run(
            [TAUQ_BIN, "format", json_path],
            capture_output=True, text=True, check=True
        )
        return result.stdout
    except subprocess.CalledProcessError as e:
        return f"ERROR: {e.stderr}"
    finally:
        os.unlink(json_path)


def toon_quote_value(value: Any, delimiter: str = ",") -> str:
    """Quote a value for TOON if needed, per spec v3.0.

    Quoting required when string contains:
    - Active delimiter (comma by default)
    - Colons or structural characters
    - Leading/trailing whitespace
    """
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        # Canonical number formatting: no exponent, no leading/trailing zeros
        if value == int(value):
            return str(int(value))
        return f"{value:g}"
    if isinstance(value, str):
        # Check if quoting needed
        needs_quote = (
            delimiter in value or
            ":" in value or
            "{" in value or
            "}" in value or
            "[" in value or
            "]" in value or
            '"' in value or
            "\n" in value or
            "\r" in value or
            "\t" in value or
            value.startswith(" ") or
            value.endswith(" ") or
            value in ("true", "false", "null") or
            value == ""
        )
        # Also quote if it looks like a number
        try:
            float(value)
            needs_quote = True
        except ValueError:
            pass

        if needs_quote:
            escaped = value.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t")
            return f'"{escaped}"'
        return value
    return str(value)


def toon_encode(data: Any, indent: int = 0, delimiter: str = ",") -> str:
    """Encode data to TOON format per spec v3.0.

    - UTF-8, LF line endings
    - 2-space indentation
    - Array header: [N]{fields}: for tabular arrays
    - Comma-separated values in rows
    """
    prefix = "  " * indent
    lines = []

    if isinstance(data, dict):
        for key, value in data.items():
            if isinstance(value, list) and len(value) > 0:
                if all(isinstance(item, dict) for item in value):
                    # Check if uniform (same keys)
                    first_keys = set(value[0].keys())
                    if all(set(item.keys()) == first_keys for item in value):
                        # Tabular array - TOON's sweet spot
                        fields = list(value[0].keys())
                        lines.append(f"{prefix}{key}[{len(value)}]{{{delimiter.join(fields)}}}:")
                        for item in value:
                            row_values = [toon_quote_value(item.get(f, ""), delimiter) for f in fields]
                            lines.append(f"{prefix}  {delimiter.join(row_values)}")
                    else:
                        # Mixed array - use list format with hyphens
                        lines.append(f"{prefix}{key}[{len(value)}]:")
                        for item in value:
                            item_lines = toon_encode(item, indent + 1, delimiter).split("\n")
                            if item_lines:
                                lines.append(f"{prefix}  - {item_lines[0].strip()}")
                                for il in item_lines[1:]:
                                    lines.append(f"{prefix}    {il.strip()}")
                else:
                    # Primitive array
                    formatted = [toon_quote_value(v, delimiter) for v in value]
                    lines.append(f"{prefix}{key}[{len(value)}]: {delimiter.join(formatted)}")
            elif isinstance(value, dict):
                lines.append(f"{prefix}{key}:")
                nested = toon_encode(value, indent + 1, delimiter)
                if nested:
                    lines.append(nested)
            else:
                lines.append(f"{prefix}{key}: {toon_quote_value(value, delimiter)}")

    elif isinstance(data, list):
        if all(isinstance(item, dict) for item in data) and data:
            # Check if uniform
            first_keys = set(data[0].keys())
            if all(set(item.keys()) == first_keys for item in data):
                # Top-level tabular array
                fields = list(data[0].keys())
                lines.append(f"{prefix}[{len(data)}]{{{delimiter.join(fields)}}}:")
                for item in data:
                    row_values = [toon_quote_value(item.get(f, ""), delimiter) for f in fields]
                    lines.append(f"{prefix}  {delimiter.join(row_values)}")
            else:
                # Mixed array
                lines.append(f"{prefix}[{len(data)}]:")
                for item in data:
                    item_lines = toon_encode(item, indent + 1, delimiter).split("\n")
                    if item_lines:
                        lines.append(f"{prefix}  - {item_lines[0].strip()}")
                        for il in item_lines[1:]:
                            lines.append(f"{prefix}    {il.strip()}")
        else:
            # Primitive array
            formatted = [toon_quote_value(v, delimiter) for v in data]
            lines.append(f"{prefix}[{len(data)}]: {delimiter.join(formatted)}")

    else:
        lines.append(f"{prefix}{toon_quote_value(data, delimiter)}")

    return "\n".join(lines)


def generate_test_datasets():
    """Generate standardized test datasets."""
    datasets = {}

    # 1. Flat records - the core benchmark
    datasets["flat_100"] = [
        {"id": i, "name": f"User{i}", "email": f"user{i}@example.com", "age": 20 + (i % 50), "active": i % 2 == 0}
        for i in range(1, 101)
    ]

    datasets["flat_1000"] = [
        {"id": i, "name": f"User{i}", "email": f"user{i}@example.com", "age": 20 + (i % 50), "active": i % 2 == 0}
        for i in range(1, 1001)
    ]

    # 2. Mixed structure (nested + arrays)
    datasets["mixed_structure"] = {
        "metadata": {
            "version": "1.0",
            "generated": "2025-11-25",
            "source": "benchmark"
        },
        "users": [
            {"id": i, "name": f"User{i}", "role": "admin" if i % 10 == 0 else "user"}
            for i in range(1, 51)
        ],
        "settings": {
            "theme": "dark",
            "notifications": True,
            "language": "en"
        }
    }

    # 3. Wide records (many fields)
    datasets["wide_records"] = [
        {
            "id": i,
            "field1": f"value{i}_1",
            "field2": f"value{i}_2",
            "field3": f"value{i}_3",
            "field4": f"value{i}_4",
            "field5": f"value{i}_5",
            "num1": i * 10,
            "num2": i * 100,
            "bool1": i % 2 == 0,
            "bool2": i % 3 == 0
        }
        for i in range(1, 101)
    ]

    # 4. Heterogeneous data
    datasets["heterogeneous"] = [
        {"id": 1, "name": "Alice", "role": "admin"},
        {"id": 2, "name": "Bob", "department": "Engineering"},
        {"id": 3, "name": "Carol", "role": "user", "tags": ["dev", "py"]},
        {"id": 4, "email": "dave@example.com"},
        {"id": 5, "name": "Eve", "metadata": {"level": 5}},
    ]

    # 5. Time series data
    datasets["timeseries"] = [
        {"timestamp": f"2025-01-{(i % 28) + 1:02d}T{(i % 24):02d}:00:00Z", "value": 100 + (i % 50), "sensor": f"S{(i % 5) + 1}"}
        for i in range(200)
    ]

    # 6. E-commerce orders
    datasets["ecommerce"] = {
        "orders": [
            {
                "order_id": f"ORD-{i:04d}",
                "customer": f"CUST-{i % 100:03d}",
                "total": round(50 + (i * 7.5) % 500, 2),
                "status": ["pending", "shipped", "delivered"][i % 3],
                "items": i % 5 + 1
            }
            for i in range(100)
        ]
    }

    # 7. API response
    datasets["api_response"] = {
        "status": "success",
        "data": {
            "pagination": {
                "page": 1,
                "per_page": 50,
                "total": 500
            },
            "results": [
                {"id": i, "title": f"Item {i}", "price": round(9.99 + i * 0.5, 2), "in_stock": i % 4 != 0}
                for i in range(50)
            ]
        },
        "meta": {
            "request_id": "abc123",
            "took_ms": 42
        }
    }

    # 8. Config style
    datasets["config_style"] = {
        "database": {
            "host": "localhost",
            "port": 5432,
            "name": "mydb"
        },
        "cache": {
            "enabled": True,
            "ttl": 3600,
            "provider": "redis"
        },
        "logging": {
            "level": "info",
            "format": "json",
            "outputs": ["stdout", "file"]
        }
    }

    return datasets


def run_benchmark():
    """Run the complete benchmark suite."""
    datasets = generate_test_datasets()
    results = []

    print("=" * 100)
    print("TAUQ vs TOON vs JSON - Rigorous Apples-to-Apples Benchmark")
    print("Tokenizer: tiktoken cl100k_base (GPT-4/Claude compatible)")
    print("TOON: Spec v3.0 compliant encoding")
    print("=" * 100)
    print()

    for name, data in datasets.items():
        print(f"Processing: {name}...")

        # Generate all formats
        json_minified = json.dumps(data, separators=(',', ':'))

        try:
            tauq_output = json_to_tauq(data)
        except Exception as e:
            tauq_output = f"ERROR: {e}"
            print(f"  tauq error: {e}")

        try:
            toon_output = toon_encode(data)
        except Exception as e:
            toon_output = f"ERROR: {e}"
            print(f"  TOON error: {e}")

        # Count tokens
        json_tokens = count_tokens(json_minified)
        tauq_tokens = count_tokens(tauq_output) if not tauq_output.startswith("ERROR") else -1
        toon_tokens = count_tokens(toon_output) if not toon_output.startswith("ERROR") else -1

        results.append({
            "dataset": name,
            "json_tokens": json_tokens,
            "tauq_tokens": tauq_tokens,
            "toon_tokens": toon_tokens,
            "tauq_vs_json": f"{((tauq_tokens - json_tokens) / json_tokens * 100):+.1f}%" if tauq_tokens > 0 else "N/A",
            "toon_vs_json": f"{((toon_tokens - json_tokens) / json_tokens * 100):+.1f}%" if toon_tokens > 0 else "N/A",
            "tauq_vs_toon": f"{((tauq_tokens - toon_tokens) / toon_tokens * 100):+.1f}%" if tauq_tokens > 0 and toon_tokens > 0 else "N/A",
        })

        # Save outputs for inspection
        output_dir = Path("outputs")
        output_dir.mkdir(exist_ok=True)

        with open(output_dir / f"{name}.json", 'w') as f:
            f.write(json_minified)
        if not tauq_output.startswith("ERROR"):
            with open(output_dir / f"{name}.tqn", 'w') as f:
                f.write(tauq_output)
        if not toon_output.startswith("ERROR"):
            with open(output_dir / f"{name}.toon", 'w') as f:
                f.write(toon_output)

    # Print results
    print()
    print("=" * 110)
    print("RESULTS (Token Counts)")
    print("=" * 110)
    print(f"{'Dataset':<20} {'JSON':<10} {'tauq':<10} {'TOON':<10} {'tauq vs JSON':<14} {'TOON vs JSON':<14} {'tauq vs TOON':<14}")
    print("-" * 110)

    for r in results:
        print(f"{r['dataset']:<20} {r['json_tokens']:<10} {r['tauq_tokens']:<10} {r['toon_tokens']:<10} {r['tauq_vs_json']:<14} {r['toon_vs_json']:<14} {r['tauq_vs_toon']:<14}")

    print()

    # Summary
    valid = [r for r in results if r['tauq_tokens'] > 0 and r['toon_tokens'] > 0]
    if valid:
        total_json = sum(r['json_tokens'] for r in valid)
        total_tauq = sum(r['tauq_tokens'] for r in valid)
        total_toon = sum(r['toon_tokens'] for r in valid)

        print("=" * 70)
        print("SUMMARY (across all datasets)")
        print("=" * 70)
        print(f"Total JSON (minified):  {total_json:>8,} tokens (baseline)")
        print(f"Total tauq:             {total_tauq:>8,} tokens ({((total_tauq - total_json) / total_json * 100):+.1f}% vs JSON)")
        print(f"Total TOON:             {total_toon:>8,} tokens ({((total_toon - total_json) / total_json * 100):+.1f}% vs JSON)")
        print()

        tauq_vs_toon_pct = (total_tauq - total_toon) / total_toon * 100
        print(f"tauq vs TOON: {tauq_vs_toon_pct:+.1f}%")
        print()

        if total_tauq < total_toon:
            diff = total_toon - total_tauq
            print(f">>> tauq WINS: saves {diff:,} tokens ({diff/total_toon*100:.1f}% more efficient than TOON) <<<")
        elif total_tauq > total_toon:
            diff = total_tauq - total_toon
            print(f">>> TOON WINS: saves {diff:,} tokens ({diff/total_tauq*100:.1f}% more efficient than tauq) <<<")
        else:
            print(">>> TIE <<<")

        print()

        # Category breakdown
        print("=" * 70)
        print("CATEGORY BREAKDOWN")
        print("=" * 70)

        flat_data = [r for r in valid if r['dataset'].startswith('flat_')]
        if flat_data:
            flat_tauq = sum(r['tauq_tokens'] for r in flat_data)
            flat_toon = sum(r['toon_tokens'] for r in flat_data)
            print(f"Flat tabular data: tauq {(flat_tauq-flat_toon)/flat_toon*100:+.1f}% vs TOON")

        nested_data = [r for r in valid if r['dataset'] in ('mixed_structure', 'api_response', 'ecommerce')]
        if nested_data:
            nested_tauq = sum(r['tauq_tokens'] for r in nested_data)
            nested_toon = sum(r['toon_tokens'] for r in nested_data)
            print(f"Nested structures: tauq {(nested_tauq-nested_toon)/nested_toon*100:+.1f}% vs TOON")

        hetero = [r for r in valid if r['dataset'] == 'heterogeneous']
        if hetero:
            print(f"Heterogeneous:     tauq {hetero[0]['tauq_vs_toon']}")

    # Save results
    with open("outputs/benchmark_results.json", 'w') as f:
        json.dump(results, f, indent=2)

    print()
    print("Outputs saved to outputs/")

    return results


if __name__ == "__main__":
    run_benchmark()
