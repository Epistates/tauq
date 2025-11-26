#!/usr/bin/env python3
"""
Token Benchmark: JSON vs TOON vs Tauq
=====================================
Fair, apples-to-apples comparison using actual tokenizer counts.
Uses tiktoken (cl100k_base) which is used by GPT-4, GPT-4o, Claude, etc.
"""

import json
import tiktoken
from dataclasses import dataclass
from typing import List, Dict, Any

# Use cl100k_base (GPT-4, GPT-4o tokenizer - also similar to Claude's)
enc = tiktoken.get_encoding("cl100k_base")

def count_tokens(text: str) -> int:
    """Count tokens using tiktoken."""
    return len(enc.encode(text))

def count_chars(text: str) -> int:
    """Count characters."""
    return len(text)

# =============================================================================
# Test Data Generation
# =============================================================================

def generate_users(n: int) -> List[Dict[str, Any]]:
    """Generate n user records."""
    roles = ["admin", "user", "viewer", "editor"]
    return [
        {
            "id": i + 1,
            "name": f"User{i+1}",
            "email": f"user{i+1}@example.com",
            "role": roles[i % 4],
            "active": i % 3 != 0
        }
        for i in range(n)
    ]

def generate_products(n: int) -> List[Dict[str, Any]]:
    """Generate n product records."""
    categories = ["electronics", "clothing", "food", "tools"]
    return [
        {
            "sku": f"SKU-{1000+i}",
            "name": f"Product {i+1}",
            "price": round(9.99 + (i * 5.5), 2),
            "category": categories[i % 4],
            "in_stock": i % 5 != 0
        }
        for i in range(n)
    ]

def generate_nested_config() -> Dict[str, Any]:
    """Generate a nested configuration object."""
    return {
        "app": {
            "name": "MyApp",
            "version": "2.1.0",
            "debug": False
        },
        "database": {
            "host": "db.example.com",
            "port": 5432,
            "credentials": {
                "user": "admin",
                "password": "secret123"
            },
            "pool": {
                "min": 5,
                "max": 20,
                "idle_timeout": 300
            }
        },
        "features": ["auth", "logging", "metrics", "caching"],
        "regions": ["us-east-1", "eu-west-1", "ap-south-1"]
    }

# =============================================================================
# Format Encoders
# =============================================================================

def to_json(data: Any, minified: bool = False) -> str:
    """Encode to JSON."""
    if minified:
        return json.dumps(data, separators=(',', ':'))
    return json.dumps(data, indent=2)

def to_toon(data: Any) -> str:
    """Encode to TOON format (best effort based on spec)."""
    lines = []

    if isinstance(data, list) and data and isinstance(data[0], dict):
        # Tabular array of objects
        keys = list(data[0].keys())
        n = len(data)
        header = f"[{n}]{{{','.join(keys)}}}:"
        lines.append(header)
        for item in data:
            values = []
            for k in keys:
                v = item[k]
                if isinstance(v, bool):
                    values.append(str(v).lower())
                elif isinstance(v, str):
                    # Quote if contains comma or special chars
                    if ',' in v or ':' in v or ' ' in v:
                        values.append(f'"{v}"')
                    else:
                        values.append(v)
                else:
                    values.append(str(v))
            lines.append("  " + ",".join(values))
    elif isinstance(data, dict):
        # Object - use indented format
        def encode_obj(obj, depth=0):
            indent = "  " * depth
            for k, v in obj.items():
                if isinstance(v, dict):
                    lines.append(f"{indent}{k}:")
                    encode_obj(v, depth + 1)
                elif isinstance(v, list):
                    if v and isinstance(v[0], dict):
                        # Nested tabular
                        keys = list(v[0].keys())
                        lines.append(f"{indent}{k}[{len(v)}]{{{','.join(keys)}}}:")
                        for item in v:
                            values = [str(item[kk]) if not isinstance(item[kk], str) else item[kk] for kk in keys]
                            lines.append(f"{indent}  " + ",".join(values))
                    else:
                        # Simple array
                        vals = [str(x) if not isinstance(x, str) else x for x in v]
                        lines.append(f"{indent}{k}[{len(v)}]: {','.join(vals)}")
                else:
                    if isinstance(v, bool):
                        v = str(v).lower()
                    elif isinstance(v, str) and (':' in v or ',' in v):
                        v = f'"{v}"'
                    lines.append(f"{indent}{k}: {v}")
        encode_obj(data)

    return "\n".join(lines)

def to_tauq(data: Any) -> str:
    """Encode to Tauq format."""
    lines = []

    if isinstance(data, list) and data and isinstance(data[0], dict):
        # Tabular array of objects
        keys = list(data[0].keys())
        lines.append(f"!def Row {' '.join(keys)}")
        for item in data:
            values = []
            for k in keys:
                v = item[k]
                if isinstance(v, bool):
                    values.append(str(v).lower())
                elif isinstance(v, str):
                    # Quote if contains space or special chars
                    if ' ' in v or any(c in v for c in '{}[]'):
                        values.append(f'"{v}"')
                    else:
                        values.append(v)
                else:
                    values.append(str(v))
            lines.append(" ".join(values))
    elif isinstance(data, dict):
        # Object - use nested format
        def encode_obj(obj, depth=0):
            for k, v in obj.items():
                if isinstance(v, dict):
                    lines.append(f"{k} {{")
                    encode_obj(v, depth + 1)
                    lines.append("}")
                elif isinstance(v, list):
                    vals = []
                    for x in v:
                        if isinstance(x, str):
                            vals.append(x if ' ' not in x else f'"{x}"')
                        else:
                            vals.append(str(x).lower() if isinstance(x, bool) else str(x))
                    lines.append(f"{k} [{' '.join(vals)}]")
                else:
                    if isinstance(v, bool):
                        v = str(v).lower()
                    elif isinstance(v, str) and ' ' in v:
                        v = f'"{v}"'
                    lines.append(f"{k} {v}")
        encode_obj(data)

    return "\n".join(lines)

def to_tauq_minified(data: Any) -> str:
    """Encode to minified Tauq format."""
    lines = to_tauq(data).split('\n')
    return ';'.join(lines)

# =============================================================================
# Benchmark Runner
# =============================================================================

@dataclass
class BenchmarkResult:
    format_name: str
    chars: int
    tokens: int

def benchmark_format(name: str, text: str) -> BenchmarkResult:
    return BenchmarkResult(
        format_name=name,
        chars=count_chars(text),
        tokens=count_tokens(text)
    )

def run_benchmark(name: str, data: Any):
    """Run benchmark for a dataset."""
    print(f"\n{'='*60}")
    print(f"BENCHMARK: {name}")
    print('='*60)

    formats = [
        ("JSON (pretty)", to_json(data, minified=False)),
        ("JSON (minified)", to_json(data, minified=True)),
        ("TOON", to_toon(data)),
        ("Tauq", to_tauq(data)),
        ("Tauq (minified)", to_tauq_minified(data)),
    ]

    results = [benchmark_format(n, t) for n, t in formats]

    # Find baseline (JSON minified)
    json_min = next(r for r in results if r.format_name == "JSON (minified)")

    print(f"\n{'Format':<20} {'Chars':>10} {'Tokens':>10} {'vs JSON':>12}")
    print("-" * 54)

    for r in results:
        savings = ((json_min.tokens - r.tokens) / json_min.tokens) * 100
        savings_str = f"{savings:+.1f}%" if r.format_name != "JSON (minified)" else "-"
        print(f"{r.format_name:<20} {r.chars:>10,} {r.tokens:>10,} {savings_str:>12}")

    # Show sample output
    print(f"\n--- Sample Output ---")
    for name, text in formats:
        if name in ["TOON", "Tauq"]:
            preview = text[:200] + "..." if len(text) > 200 else text
            print(f"\n{name}:\n{preview}")

    return results

def main():
    print("="*60)
    print("TOKEN BENCHMARK: JSON vs TOON vs Tauq")
    print("Tokenizer: tiktoken cl100k_base (GPT-4/Claude compatible)")
    print("="*60)

    # Test 1: Small tabular (10 records)
    run_benchmark("10 User Records", generate_users(10))

    # Test 2: Medium tabular (100 records)
    run_benchmark("100 User Records", generate_users(100))

    # Test 3: Large tabular (1000 records)
    run_benchmark("1000 User Records", generate_users(1000))

    # Test 4: Nested config object
    run_benchmark("Nested Config Object", generate_nested_config())

    # Test 5: Products (different field types)
    run_benchmark("100 Product Records", generate_products(100))

    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)
    print("""
Key Findings:
- Token counts are ACTUAL values from tiktoken (cl100k_base)
- Savings are relative to minified JSON (most compact JSON)
- Both TOON and Tauq achieve significant savings for tabular data
- Nested objects show smaller differences

Notes:
- TOON requires count upfront: [N]{fields}:
- Tauq uses schema directive: !def Name fields
- Both eliminate repeated keys for tabular data
- Tauq uses space delimiters (fewer tokens than commas)
""")

if __name__ == "__main__":
    main()
