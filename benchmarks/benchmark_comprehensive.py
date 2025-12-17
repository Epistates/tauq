#!/usr/bin/env python3
"""
Comprehensive Tauq Benchmark Suite
===================================
Rigorous apples-to-apples comparison: Tauq vs TOON vs JSON vs CSV

Features:
- Uses tiktoken o200k_base (GPT-4o, Claude 3.5+, o1 models)
- TOON Spec v3.0 compliant implementation
- CSV support for flat tabular data
- Diverse dataset types with tabular eligibility metrics
- Statistical analysis and comprehensive reporting
- Addresses LocalLLaMA community critiques
"""

import json
import subprocess
import tempfile
import os
import random
import csv
import io
from pathlib import Path
from typing import Any, Dict, List, Optional
from dataclasses import dataclass
from datetime import datetime, timedelta

try:
    import tiktoken
except ImportError:
    print("ERROR: tiktoken not installed. Run: pip install tiktoken")
    exit(1)

# Initialize tokenizer (o200k_base is used by GPT-4o, Claude 3.5+, o1)
ENCODER = tiktoken.get_encoding("o200k_base")

# Path to tauq binary
TAUQ_BIN = Path(__file__).parent.parent / "target" / "release" / "tauq"

# Seed for reproducibility
RANDOM_SEED = 12345
random.seed(RANDOM_SEED)


@dataclass
class DatasetMetadata:
    """Metadata describing dataset characteristics"""
    supports_csv: bool
    structure_class: str  # uniform, nested, semi-uniform, deep, heterogeneous
    tabular_eligibility: int  # 0-100% of data that fits tabular format


@dataclass
class Dataset:
    """Test dataset with metadata"""
    name: str
    description: str
    data: Any
    metadata: DatasetMetadata


def count_tokens(text: str) -> int:
    """Count tokens using tiktoken o200k_base (GPT-4o, Claude 3.5+)"""
    return len(ENCODER.encode(text))


def json_to_tauq(data: Any, mode: str = "default") -> str:
    """Convert JSON to tauq format using the tauq CLI.

    Args:
        data: Data to convert
        mode: "default" (adaptive !def usage), "no-schemas" (pure key:value),
              "optimized" (comma-delimited), or "ultra" (compact)
    """
    with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
        json.dump(data, f)
        json_path = f.name

    try:
        cmd = [str(TAUQ_BIN), "format", json_path]
        if mode == "no-schemas":
            cmd.append("--no-schemas")
        elif mode == "optimized":
            cmd.append("--optimized")
        elif mode == "ultra":
            cmd.append("--ultra")

        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=True
        )
        return result.stdout
    except subprocess.CalledProcessError as e:
        return f"ERROR: {e.stderr}"
    finally:
        os.unlink(json_path)


def toon_quote_value(value: Any) -> str:
    """Quote a value for TOON if needed, per spec v3.0"""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        # Canonical number formatting
        if value == int(value):
            return str(int(value))
        return f"{value:g}"
    if isinstance(value, str):
        # Check if quoting needed
        needs_quote = (
            "," in value or ":" in value or "{" in value or "}" in value or
            "[" in value or "]" in value or '"' in value or "\n" in value or
            "\r" in value or "\t" in value or value.startswith(" ") or
            value.endswith(" ") or value in ("true", "false", "null") or
            value == ""
        )
        # Also quote if it looks like a number
        if not needs_quote:
            try:
                float(value)
                needs_quote = True
            except ValueError:
                pass

        if needs_quote:
            escaped = value.replace("\\", "\\\\").replace('"', '\\"')
            escaped = escaped.replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t")
            return f'"{escaped}"'
        return value
    return str(value)


def csv_encode(data: Any) -> str:
    """Encode data to CSV format (only works for flat tabular data)

    Returns empty string if data structure is not CSV-compatible.
    CSV is designed for flat tabular data only - nested structures are not supported.
    """
    output = io.StringIO()

    # Handle top-level object with array values
    if isinstance(data, dict):
        sections = []
        for key, value in data.items():
            if isinstance(value, list) and len(value) > 0:
                # Check if all items are flat dicts
                if all(isinstance(item, dict) for item in value):
                    # Check if all values are primitives
                    all_flat = all(
                        all(not isinstance(v, (dict, list)) for v in item.values())
                        for item in value
                    )
                    if all_flat:
                        section_output = io.StringIO()
                        writer = csv.DictWriter(section_output, fieldnames=value[0].keys())
                        writer.writeheader()
                        writer.writerows(value)
                        sections.append(f"# {key}\n{section_output.getvalue().strip()}")
        return "\n\n".join(sections)

    # Handle top-level array
    elif isinstance(data, list) and len(data) > 0:
        if all(isinstance(item, dict) for item in data):
            # Check if all values are primitives
            all_flat = all(
                all(not isinstance(v, (dict, list)) for v in item.values())
                for item in data
            )
            if all_flat and data:
                writer = csv.DictWriter(output, fieldnames=data[0].keys())
                writer.writeheader()
                writer.writerows(data)
                return output.getvalue().strip()

    return ""  # CSV not applicable


def toon_encode(data: Any, indent: int = 0) -> str:
    """Encode data to TOON format per spec v3.0

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
                        lines.append(f"{prefix}{key}[{len(value)}]{{{','.join(fields)}}}:")
                        for item in value:
                            row_values = [toon_quote_value(item.get(f, "")) for f in fields]
                            lines.append(f"{prefix}  {','.join(row_values)}")
                    else:
                        # Mixed array - use list format
                        lines.append(f"{prefix}{key}[{len(value)}]:")
                        for item in value:
                            item_lines = toon_encode(item, indent + 1).split("\n")
                            if item_lines:
                                lines.append(f"{prefix}  - {item_lines[0].strip()}")
                                for il in item_lines[1:]:
                                    if il.strip():
                                        lines.append(f"{prefix}    {il.strip()}")
                else:
                    # Primitive array
                    formatted = [toon_quote_value(v) for v in value]
                    lines.append(f"{prefix}{key}[{len(value)}]: {','.join(formatted)}")
            elif isinstance(value, dict):
                lines.append(f"{prefix}{key}:")
                nested = toon_encode(value, indent + 1)
                if nested:
                    lines.append(nested)
            else:
                lines.append(f"{prefix}{key}: {toon_quote_value(value)}")

    elif isinstance(data, list):
        if all(isinstance(item, dict) for item in data) and data:
            # Check if uniform
            first_keys = set(data[0].keys())
            if all(set(item.keys()) == first_keys for item in data):
                # Top-level tabular array
                fields = list(data[0].keys())
                lines.append(f"{prefix}[{len(data)}]{{{','.join(fields)}}}:")
                for item in data:
                    row_values = [toon_quote_value(item.get(f, "")) for f in fields]
                    lines.append(f"{prefix}  {','.join(row_values)}")
            else:
                # Mixed array
                lines.append(f"{prefix}[{len(data)}]:")
                for item in data:
                    item_lines = toon_encode(item, indent + 1).split("\n")
                    if item_lines:
                        lines.append(f"{prefix}  - {item_lines[0].strip()}")
                        for il in item_lines[1:]:
                            if il.strip():
                                lines.append(f"{prefix}    {il.strip()}")
        else:
            # Primitive array
            formatted = [toon_quote_value(v) for v in data]
            lines.append(f"{prefix}[{len(data)}]: {','.join(formatted)}")

    else:
        lines.append(f"{prefix}{toon_quote_value(data)}")

    return "\n".join(lines)


# ===== Dataset Generators =====

def generate_employees(count: int) -> Dict[str, List[Dict]]:
    """Generate uniform employee records (100% tabular)"""
    departments = ['Engineering', 'Sales', 'Marketing', 'HR', 'Operations', 'Finance']
    return {
        "employees": [
            {
                "id": i + 1,
                "name": f"Employee{i+1}",
                "email": f"employee{i+1}@company.com",
                "department": departments[i % len(departments)],
                "salary": 45000 + (i * 1000) % 105000,
                "yearsExperience": 1 + (i % 25),
                "active": i % 5 != 0
            }
            for i in range(count)
        ]
    }


def generate_analytics(days: int, start_date: str = "2025-01-01") -> Dict[str, List[Dict]]:
    """Generate time-series analytics data (100% tabular)"""
    date = datetime.fromisoformat(start_date)
    metrics = []

    for i in range(days):
        current_date = date + timedelta(days=i)
        base_views = 5000
        weekend_mult = 0.7 if current_date.weekday() >= 5 else 1.0
        views = int(base_views * weekend_mult + random.randint(-1000, 3000))
        clicks = int(views * random.uniform(0.02, 0.08))
        conversions = int(clicks * random.uniform(0.05, 0.15))
        revenue = round(conversions * random.uniform(49.99, 299.99), 2)

        metrics.append({
            "date": current_date.date().isoformat(),
            "views": max(0, views),
            "clicks": max(0, clicks),
            "conversions": max(0, conversions),
            "revenue": round(revenue, 2),
            "bounceRate": round(random.uniform(0.3, 0.7), 2)
        })

    return {"metrics": metrics}


def generate_orders(count: int) -> Dict[str, List[Dict]]:
    """Generate e-commerce orders with nested items (~33% tabular)"""
    products = ['Wireless Mouse', 'USB Cable', 'Laptop Stand', 'Keyboard', 'Webcam', 'Headphones']
    statuses = ['pending', 'processing', 'shipped', 'delivered', 'cancelled']

    orders = []
    for i in range(count):
        customer_id = (i % 20) + 1
        item_count = random.randint(1, 4)

        items = []
        for j in range(item_count):
            price = round(random.uniform(9.99, 199.99), 2)
            quantity = random.randint(1, 5)
            items.append({
                "sku": f"SKU-{random.randint(100000, 999999)}",
                "name": products[j % len(products)],
                "quantity": quantity,
                "price": price
            })

        subtotal = round(sum(item['price'] * item['quantity'] for item in items), 2)
        tax = round(subtotal * 0.08, 2)
        total = round(subtotal + tax, 2)

        orders.append({
            "orderId": f"ORD-{str(i+1).zfill(4)}",
            "customer": {
                "id": customer_id,
                "name": f"Customer{customer_id}",
                "email": f"customer{customer_id}@example.com",
                "phone": f"+1-555-{random.randint(1000, 9999)}"
            },
            "items": items,
            "subtotal": subtotal,
            "tax": tax,
            "total": total,
            "status": statuses[i % len(statuses)],
            "orderDate": (datetime(2025, 1, 1) + timedelta(days=i % 90)).date().isoformat()
        })

    return {"orders": orders}


def generate_event_logs(count: int) -> Dict[str, List[Dict]]:
    """Generate semi-uniform event logs (~50% with nested errors, 50% tabular)"""
    endpoints = ['/api/users', '/api/orders', '/api/products', '/api/auth', '/api/payments']
    levels = ['info', 'warn', 'error']
    errors = [
        'Database connection timeout',
        'Invalid authentication token',
        'Resource not found',
        'Internal server error',
        'Rate limit exceeded'
    ]

    logs = []
    for i in range(count):
        level = random.choice(levels)
        has_error = level == 'error' or (level == 'warn' and random.random() < 0.3)

        log = {
            "timestamp": (datetime(2025, 1, 1) + timedelta(hours=i)).isoformat(),
            "level": level,
            "endpoint": random.choice(endpoints),
            "statusCode": random.randint(400, 599) if has_error else random.randint(200, 299),
            "responseTime": random.randint(10, 5000),
            "userId": random.randint(1000, 9999)
        }

        if has_error:
            log["error"] = {
                "message": random.choice(errors),
                "stack": f"Error: Stack trace line 1\n  at function1\n  at function2",
                "retryable": random.random() < 0.6
            }

        logs.append(log)

    return {"logs": logs}


def generate_nested_config() -> Dict[str, Any]:
    """Generate deeply nested configuration (~0% tabular)"""
    return {
        "environment": "production",
        "version": "2.1.0",
        "database": {
            "host": "db.example.com",
            "port": 5432,
            "name": "production_db",
            "pool": {
                "min": 2,
                "max": 20,
                "idleTimeout": 30000
            },
            "replicas": [
                {"host": f"replica-{i}.example.com", "port": 5432, "priority": i+1}
                for i in range(3)
            ]
        },
        "features": {
            "darkMode": {
                "enabled": True,
                "rollout": 100,
                "variants": [
                    {"name": "default", "weight": 70, "config": {"theme": "dark", "animations": True}},
                    {"name": "minimal", "weight": 30, "config": {"theme": "dark", "animations": False}}
                ]
            },
            "analytics": {
                "enabled": True,
                "rollout": 100,
                "variants": [
                    {"name": "full", "weight": 100, "config": {"tracking": "all", "sampling": 1.0}}
                ]
            }
        },
        "authentication": {
            "providers": [
                {
                    "name": "oauth2",
                    "clientId": "abc123",
                    "scopes": ["read", "write", "admin"],
                    "config": {
                        "authUrl": "https://auth.example.com/oauth",
                        "tokenUrl": "https://auth.example.com/token"
                    }
                }
            ],
            "session": {
                "secret": "supersecretkey",
                "duration": 86400,
                "refreshThreshold": 3600
            }
        },
        "monitoring": {
            "metrics": {"enabled": True, "endpoint": "/metrics"},
            "tracing": {"enabled": True, "sampleRate": 0.1},
            "health": {"endpoint": "/health", "includeDetails": True}
        }
    }


def generate_wide_records(count: int) -> List[Dict]:
    """Generate records with many fields (100% tabular but tests schema efficiency)"""
    return [
        {
            "id": i + 1,
            **{f"field{j}": f"value{i}_{j}" for j in range(1, 11)},
            "num1": i * 10,
            "num2": i * 100,
            "bool1": i % 2 == 0,
            "bool2": i % 3 == 0
        }
        for i in range(count)
    ]


def generate_heterogeneous(count: int) -> List[Dict]:
    """Generate records with varying schemas (0% tabular)"""
    patterns = [
        lambda i: {"id": i, "name": f"User{i}", "role": "admin"},
        lambda i: {"id": i, "name": f"User{i}", "department": "Engineering"},
        lambda i: {"id": i, "name": f"User{i}", "role": "user", "tags": ["dev", "py"]},
        lambda i: {"id": i, "email": f"user{i}@example.com"},
        lambda i: {"id": i, "name": f"User{i}", "metadata": {"level": i % 10}},
        lambda i: {"id": i, "name": f"User{i}", "active": i % 2 == 0, "score": i * 10}
    ]

    return [patterns[i % len(patterns)](i + 1) for i in range(count)]


# ===== Test Datasets =====

DATASETS = [
    Dataset(
        name="tabular-100",
        description="Uniform employee records (100 rows)",
        data=generate_employees(100),
        metadata=DatasetMetadata(True, "uniform", 100)
    ),
    Dataset(
        name="tabular-2000",
        description="Uniform employee records (2000 rows)",
        data=generate_employees(2000),
        metadata=DatasetMetadata(True, "uniform", 100)
    ),
    Dataset(
        name="analytics-60",
        description="Time-series analytics (60 days)",
        data=generate_analytics(60),
        metadata=DatasetMetadata(True, "uniform", 100)
    ),
    Dataset(
        name="analytics-365",
        description="Time-series analytics (365 days)",
        data=generate_analytics(365),
        metadata=DatasetMetadata(True, "uniform", 100)
    ),
    Dataset(
        name="nested-50",
        description="E-commerce orders with nested items (50 orders)",
        data=generate_orders(50),
        metadata=DatasetMetadata(False, "nested", 33)
    ),
    Dataset(
        name="nested-500",
        description="E-commerce orders with nested items (500 orders)",
        data=generate_orders(500),
        metadata=DatasetMetadata(False, "nested", 33)
    ),
    Dataset(
        name="event-logs-75",
        description="Semi-uniform event logs (75 logs)",
        data=generate_event_logs(75),
        metadata=DatasetMetadata(False, "semi-uniform", 50)
    ),
    Dataset(
        name="event-logs-2000",
        description="Semi-uniform event logs (2000 logs)",
        data=generate_event_logs(2000),
        metadata=DatasetMetadata(False, "semi-uniform", 50)
    ),
    Dataset(
        name="nested-config",
        description="Deeply nested configuration",
        data=generate_nested_config(),
        metadata=DatasetMetadata(False, "deep", 0)
    ),
    Dataset(
        name="wide-records",
        description="Wide records (100 rows × 15 fields)",
        data=generate_wide_records(100),
        metadata=DatasetMetadata(True, "uniform", 100)
    ),
    Dataset(
        name="heterogeneous",
        description="Heterogeneous records (100 rows, varying schemas)",
        data=generate_heterogeneous(100),
        metadata=DatasetMetadata(False, "heterogeneous", 0)
    )
]


def run_benchmark():
    """Run the complete benchmark suite"""
    print("=" * 100)
    print("Tauq vs TOON vs JSON vs CSV - Comprehensive Token Efficiency Benchmark")
    print("Tokenizer: tiktoken o200k_base (GPT-4o, Claude 3.5+, o1 models)")
    print("Addresses LocalLLaMA community critiques")
    print("=" * 100)
    print()

    results = []

    for dataset in DATASETS:
        print(f"Processing: {dataset.name}...")

        # Generate all formats
        json_min = json.dumps(dataset.data, separators=(',', ':'))
        json_pretty = json.dumps(dataset.data, indent=2)

        try:
            tauq_std = json_to_tauq(dataset.data, "standard")
        except Exception as e:
            tauq_std = f"ERROR: {e}"
            print(f"  tauq standard error: {e}")

        try:
            tauq_opt = json_to_tauq(dataset.data, "optimized")
        except Exception as e:
            tauq_opt = f"ERROR: {e}"
            print(f"  tauq optimized error: {e}")

        try:
            tauq_ultra = json_to_tauq(dataset.data, "ultra")
        except Exception as e:
            tauq_ultra = f"ERROR: {e}"
            print(f"  tauq ultra error: {e}")

        try:
            toon = toon_encode(dataset.data)
        except Exception as e:
            toon = f"ERROR: {e}"
            print(f"  TOON error: {e}")

        # CSV only for flat tabular data
        csv_output = ""
        if dataset.metadata.supports_csv:
            try:
                csv_output = csv_encode(dataset.data)
            except Exception as e:
                print(f"  CSV error: {e}")

        # Count tokens
        json_min_tok = count_tokens(json_min)
        json_pretty_tok = count_tokens(json_pretty)
        tauq_std_tok = count_tokens(tauq_std) if not tauq_std.startswith("ERROR") else -1
        tauq_opt_tok = count_tokens(tauq_opt) if not tauq_opt.startswith("ERROR") else -1
        tauq_ultra_tok = count_tokens(tauq_ultra) if not tauq_ultra.startswith("ERROR") else -1
        toon_tok = count_tokens(toon) if not toon.startswith("ERROR") else -1
        csv_tok = count_tokens(csv_output) if csv_output else -1

        result = {
            "dataset": dataset.name,
            "description": dataset.description,
            "tabular_eligibility": dataset.metadata.tabular_eligibility,
            "structure_class": dataset.metadata.structure_class,
            "supports_csv": dataset.metadata.supports_csv,
            "json_min_tokens": json_min_tok,
            "json_pretty_tokens": json_pretty_tok,
            "tauq_std_tokens": tauq_std_tok,
            "tauq_opt_tokens": tauq_opt_tok,
            "tauq_ultra_tokens": tauq_ultra_tok,
            "toon_tokens": toon_tok,
            "csv_tokens": csv_tok
        }

        # Calculate percentages
        if tauq_std_tok > 0:
            result["tauq_std_vs_json"] = f"{((tauq_std_tok - json_min_tok) / json_min_tok * 100):+.1f}%"
            result["tauq_std_vs_toon"] = f"{((tauq_std_tok - toon_tok) / toon_tok * 100):+.1f}%" if toon_tok > 0 else "N/A"
            result["tauq_std_vs_csv"] = f"{((tauq_std_tok - csv_tok) / csv_tok * 100):+.1f}%" if csv_tok > 0 else "N/A"
        else:
            result["tauq_std_vs_json"] = "N/A"
            result["tauq_std_vs_toon"] = "N/A"
            result["tauq_std_vs_csv"] = "N/A"

        if toon_tok > 0:
            result["toon_vs_json"] = f"{((toon_tok - json_min_tok) / json_min_tok * 100):+.1f}%"
            result["toon_vs_csv"] = f"{((toon_tok - csv_tok) / csv_tok * 100):+.1f}%" if csv_tok > 0 else "N/A"
        else:
            result["toon_vs_json"] = "N/A"
            result["toon_vs_csv"] = "N/A"

        if csv_tok > 0:
            result["csv_vs_json"] = f"{((csv_tok - json_min_tok) / json_min_tok * 100):+.1f}%"
        else:
            result["csv_vs_json"] = "N/A"

        results.append(result)

        # Save outputs
        output_dir = Path(__file__).parent / "outputs"
        output_dir.mkdir(exist_ok=True)

        (output_dir / f"{dataset.name}.json").write_text(json_min)
        if not tauq_std.startswith("ERROR"):
            (output_dir / f"{dataset.name}.tqn").write_text(tauq_std)
        if not tauq_opt.startswith("ERROR"):
            (output_dir / f"{dataset.name}.opt.tqn").write_text(tauq_opt)
        if not tauq_ultra.startswith("ERROR"):
            (output_dir / f"{dataset.name}.ultra.tqn").write_text(tauq_ultra)
        if not toon.startswith("ERROR"):
            (output_dir / f"{dataset.name}.toon").write_text(toon)
        if csv_output:
            (output_dir / f"{dataset.name}.csv").write_text(csv_output)

    # Print results table
    print()
    print("=" * 170)
    print("RESULTS (Token Counts)")
    print("=" * 170)
    print(f"{'Dataset':<25} {'Tab%':<6} {'CSV?':<6} {'JSON':<8} {'CSV':<8} {'tauq':<8} {'TOON':<8} {'tauq vs JSON':<15} {'tauq vs CSV':<15} {'tauq vs TOON':<15}")
    print("-" * 170)

    for r in results:
        csv_str = str(r['csv_tokens']) if r['csv_tokens'] > 0 else "N/A"
        csv_flag = "✓" if r['supports_csv'] else ""
        print(f"{r['dataset']:<25} {r['tabular_eligibility']:<6} {csv_flag:<6} {r['json_min_tokens']:<8} {csv_str:<8} {r['tauq_std_tokens']:<8} {r['toon_tokens']:<8} {r['tauq_std_vs_json']:<15} {r.get('tauq_std_vs_csv', 'N/A'):<15} {r['tauq_std_vs_toon']:<15}")

    print()

    # Summary statistics
    valid = [r for r in results if r['tauq_std_tokens'] > 0 and r['toon_tokens'] > 0]
    if valid:
        total_json = sum(r['json_min_tokens'] for r in valid)
        total_tauq = sum(r['tauq_std_tokens'] for r in valid)
        total_toon = sum(r['toon_tokens'] for r in valid)

        print("=" * 80)
        print("SUMMARY (across all datasets)")
        print("=" * 80)
        print(f"Total JSON (minified):  {total_json:>10,} tokens (baseline)")
        print(f"Total tauq (standard):  {total_tauq:>10,} tokens ({((total_tauq - total_json) / total_json * 100):+.1f}% vs JSON)")
        print(f"Total TOON:             {total_toon:>10,} tokens ({((total_toon - total_json) / total_json * 100):+.1f}% vs JSON)")
        print()
        print(f"tauq vs TOON: {((total_tauq - total_toon) / total_toon * 100):+.1f}%")
        print()

        if total_tauq < total_toon:
            diff = total_toon - total_tauq
            pct = (diff / total_toon * 100)
            print(f">>> tauq WINS: saves {diff:,} tokens ({pct:.1f}% more efficient than TOON) <<<")
        elif total_tauq > total_toon:
            diff = total_tauq - total_toon
            pct = (diff / total_tauq * 100)
            print(f">>> TOON WINS: saves {diff:,} tokens ({pct:.1f}% more efficient than tauq) <<<")
        else:
            print(">>> TIE <<<")

        # Category breakdown
        print()
        print("=" * 80)
        print("CATEGORY BREAKDOWN")
        print("=" * 80)

        for structure_class in ["uniform", "nested", "semi-uniform", "deep", "heterogeneous"]:
            category_results = [r for r in valid if r['structure_class'] == structure_class]
            if category_results:
                cat_tauq = sum(r['tauq_std_tokens'] for r in category_results)
                cat_toon = sum(r['toon_tokens'] for r in category_results)
                cat_json = sum(r['json_min_tokens'] for r in category_results)

                print(f"\n{structure_class.capitalize()} structures ({len(category_results)} datasets):")
                print(f"  tauq: {cat_tauq:,} tokens ({((cat_tauq - cat_json) / cat_json * 100):+.1f}% vs JSON)")
                print(f"  TOON: {cat_toon:,} tokens ({((cat_toon - cat_json) / cat_json * 100):+.1f}% vs JSON)")
                print(f"  tauq vs TOON: {((cat_tauq - cat_toon) / cat_toon * 100):+.1f}%")

    # Save results
    output_dir = Path(__file__).parent / "outputs"
    (output_dir / "benchmark_results.json").write_text(
        json.dumps(results, indent=2)
    )

    print()
    print("=" * 80)
    print(f"Results saved to {output_dir}/")
    print("=" * 80)

    return results


if __name__ == "__main__":
    run_benchmark()
