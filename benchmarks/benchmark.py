#!/usr/bin/env python3
"""
Rigorous Apples-to-Apples Benchmark: tauq vs TOON vs JSON
Uses tiktoken cl100k_base (GPT-4/Claude tokenizer) for accurate token counts.
"""

import json
import subprocess
import tempfile
import os
from pathlib import Path
from typing import Any
import tiktoken

# Try to import toon-python
try:
    from toon_python import encode as toon_encode
    TOON_AVAILABLE = True
except ImportError:
    TOON_AVAILABLE = False
    print("WARNING: toon-python not available, will generate TOON manually")

# Initialize tokenizer (cl100k_base is used by GPT-4 and Claude)
ENCODER = tiktoken.get_encoding("cl100k_base")

# Path to tauq binary
TAUQ_BIN = "/app/tauq_src/target/release/tauq"


def count_tokens(text: str) -> int:
    """Count tokens using tiktoken cl100k_base."""
    return len(ENCODER.encode(text))


def json_to_tauq(data: dict | list, mode: str = "standard") -> str:
    """Convert JSON to tauq format using the tauq CLI.

    mode: "standard", "optimized", or "ultra"
    """
    with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
        json.dump(data, f)
        json_path = f.name

    try:
        cmd = [TAUQ_BIN, "format", json_path]
        if mode == "optimized":
            cmd.append("--optimized")
        elif mode == "ultra":
            cmd.append("--ultra")

        result = subprocess.run(
            cmd,
            capture_output=True, text=True, check=True
        )
        return result.stdout
    finally:
        os.unlink(json_path)


def json_to_toon(data: dict | list) -> str:
    """Convert to TOON format using toon-python or manual generation."""
    if TOON_AVAILABLE:
        return toon_encode(data)
    else:
        # Manual TOON generation for comparison
        return manual_toon_encode(data)


def manual_toon_encode(data: Any, indent: int = 0, context: str = "") -> str:
    """Manual TOON encoder matching spec v3.0 for benchmarking.

    TOON format (Token Oriented Object Notation):
    - Arrays of objects: key[N]{fields}: rows
    - Top-level arrays: [N]{fields}: rows
    - Nested objects: key: indented children
    - Primitives: key: value
    """
    prefix = "  " * indent
    lines = []

    if isinstance(data, dict):
        for key, value in data.items():
            if isinstance(value, list) and len(value) > 0:
                if all(isinstance(item, dict) for item in value):
                    # Tabular array of objects - TOON's sweet spot
                    # Get fields from first object (preserving order)
                    fields = list(value[0].keys()) if value else []
                    lines.append(f"{prefix}{key}[{len(value)}]{{{','.join(fields)}}}:")
                    for item in value:
                        row_values = [format_toon_value(item.get(f, "")) for f in fields]
                        lines.append(f"{prefix}  {','.join(row_values)}")
                else:
                    # Primitive array
                    formatted = [format_toon_value(v) for v in value]
                    lines.append(f"{prefix}{key}[{len(value)}]: {','.join(formatted)}")
            elif isinstance(value, dict):
                lines.append(f"{prefix}{key}:")
                lines.append(manual_toon_encode(value, indent + 1))
            else:
                lines.append(f"{prefix}{key}: {format_toon_value(value)}")
    elif isinstance(data, list):
        if all(isinstance(item, dict) for item in data) and data:
            # Top-level array of uniform objects
            # Check if all objects have same keys
            first_keys = set(data[0].keys())
            if all(set(item.keys()) == first_keys for item in data):
                fields = list(data[0].keys())
                lines.append(f"{prefix}[{len(data)}]{{{','.join(fields)}}}:")
                for item in data:
                    row_values = [format_toon_value(item.get(f, "")) for f in fields]
                    lines.append(f"{prefix}  {','.join(row_values)}")
            else:
                # Heterogeneous - fall back to individual objects
                for item in data:
                    lines.append(manual_toon_encode(item, indent))
        else:
            formatted = [format_toon_value(v) for v in data]
            lines.append(f"{prefix}[{len(data)}]: {','.join(formatted)}")
    else:
        lines.append(format_toon_value(data))

    return "\n".join(lines)


def format_toon_value(value: Any) -> str:
    """Format a value for TOON output."""
    if value is None:
        return "null"
    elif isinstance(value, bool):
        return "true" if value else "false"
    elif isinstance(value, (int, float)):
        return str(value)
    elif isinstance(value, str):
        # Quote if contains special chars
        if any(c in value for c in ',:{}[]"\\\n\r\t ') or value in ('true', 'false', 'null') or value == '':
            escaped = value.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n')
            return f'"{escaped}"'
        return value
    else:
        return str(value)


def generate_test_datasets():
    """Generate standardized test datasets of varying complexity."""
    datasets = {}

    # 1. Simple flat records (100 rows) - TOON/tauq sweet spot
    datasets["flat_100"] = [
        {"id": i, "name": f"User{i}", "email": f"user{i}@example.com", "age": 20 + (i % 50), "active": i % 2 == 0}
        for i in range(1, 101)
    ]

    # 2. Simple flat records (1000 rows) - scalability test
    datasets["flat_1000"] = [
        {"id": i, "name": f"User{i}", "email": f"user{i}@example.com", "age": 20 + (i % 50), "active": i % 2 == 0}
        for i in range(1, 1001)
    ]

    # 3. Mixed structure (nested + arrays)
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

    # 4. Deeply nested structure - potential weakness for schema-based formats
    # Multiple companies with deep nesting to properly test nested object handling
    datasets["deeply_nested"] = {
        "organizations": [
            {
                "id": i,
                "name": f"Company{i}",
                "founded": 2000 + i,
                "headquarters": {
                    "country": ["USA", "UK", "Germany", "Japan", "Canada"][i % 5],
                    "city": f"City{i}",
                    "address": {
                        "street": f"{100 + i} Main Street",
                        "building": f"Suite {i * 10}",
                        "postal": {
                            "code": f"{10000 + i}",
                            "region": f"Region{i % 10}"
                        }
                    },
                    "coordinates": {
                        "lat": 37.0 + (i * 0.1),
                        "lng": -122.0 + (i * 0.1)
                    }
                },
                "departments": [
                    {
                        "name": dept_name,
                        "budget": (j + 1) * 100000,
                        "head": {
                            "name": f"Manager{i}_{j}",
                            "email": f"manager{i}_{j}@company{i}.com",
                            "tenure_years": (i + j) % 20
                        },
                        "teams": [
                            {
                                "name": f"Team{k}",
                                "members": 5 + (k % 10),
                                "lead": f"Lead{i}_{j}_{k}",
                                "projects": [
                                    {"name": f"Project{m}", "status": ["active", "completed", "planned"][m % 3]}
                                    for m in range(3)
                                ]
                            }
                            for k in range(3)
                        ]
                    }
                    for j, dept_name in enumerate(["Engineering", "Sales", "Marketing", "Operations"][:2 + (i % 3)])
                ],
                "financials": {
                    "revenue": {
                        "q1": 1000000 + i * 50000,
                        "q2": 1100000 + i * 50000,
                        "q3": 1050000 + i * 50000,
                        "q4": 1200000 + i * 50000
                    },
                    "expenses": {
                        "q1": 800000 + i * 40000,
                        "q2": 850000 + i * 40000,
                        "q3": 820000 + i * 40000,
                        "q4": 900000 + i * 40000
                    }
                }
            }
            for i in range(1, 11)  # 10 organizations with deep nesting
        ]
    }

    # 5. Wide records (many fields) - tests schema efficiency
    datasets["wide_records"] = [
        {
            "id": i,
            "field1": f"value{i}_1",
            "field2": f"value{i}_2",
            "field3": f"value{i}_3",
            "field4": f"value{i}_4",
            "field5": f"value{i}_5",
            "field6": f"value{i}_6",
            "field7": f"value{i}_7",
            "field8": f"value{i}_8",
            "field9": f"value{i}_9",
            "field10": f"value{i}_10",
            "num1": i * 10,
            "num2": i * 100,
            "bool1": i % 2 == 0,
            "bool2": i % 3 == 0
        }
        for i in range(1, 101)
    ]

    # 6. Sparse/heterogeneous data - challenging for schema-based formats
    # 100 records with varying schemas to properly test heterogeneous handling
    heterogeneous_patterns = [
        lambda i: {"id": i, "name": f"User{i}", "role": "admin"},
        lambda i: {"id": i, "name": f"User{i}", "department": "Engineering"},
        lambda i: {"id": i, "name": f"User{i}", "role": "user", "tags": ["dev", "py"]},
        lambda i: {"id": i, "email": f"user{i}@example.com"},
        lambda i: {"id": i, "name": f"User{i}", "metadata": {"level": i % 10}},
        lambda i: {"id": i, "name": f"User{i}", "active": i % 2 == 0, "score": i * 10},
        lambda i: {"id": i, "title": f"Item{i}", "price": 9.99 + i},
        lambda i: {"id": i, "name": f"User{i}", "address": {"city": "City" + str(i % 10), "zip": str(10000 + i)}},
    ]
    datasets["heterogeneous"] = [
        heterogeneous_patterns[i % len(heterogeneous_patterns)](i)
        for i in range(1, 101)
    ]

    # 7. Time series data - common LLM use case
    datasets["timeseries"] = [
        {"timestamp": f"2025-01-{(i % 28) + 1:02d}T{(i % 24):02d}:00:00Z", "value": 100 + (i % 50), "sensor": f"S{(i % 5) + 1}"}
        for i in range(200)
    ]

    # 8. E-commerce orders - realistic mixed structure
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

    # 9. API response simulation
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

    # 10. Config file style (many top-level keys) - realistic application config
    datasets["config_style"] = {
        "database": {
            "primary": {
                "host": "db-primary.example.com",
                "port": 5432,
                "name": "production_db",
                "pool_size": 20,
                "ssl": True
            },
            "replica": {
                "host": "db-replica.example.com",
                "port": 5432,
                "name": "production_db",
                "pool_size": 10,
                "ssl": True
            },
            "migrations": {
                "auto_run": False,
                "directory": "./migrations",
                "table": "schema_versions"
            }
        },
        "cache": {
            "enabled": True,
            "ttl": 3600,
            "provider": "redis",
            "redis": {
                "host": "cache.example.com",
                "port": 6379,
                "db": 0,
                "password_env": "REDIS_PASSWORD"
            },
            "fallback": {
                "type": "memory",
                "max_size_mb": 256
            }
        },
        "logging": {
            "level": "info",
            "format": "json",
            "outputs": ["stdout", "file", "syslog"],
            "file": {
                "path": "/var/log/app/app.log",
                "max_size_mb": 100,
                "rotate_count": 5
            },
            "syslog": {
                "facility": "local0",
                "host": "logs.example.com"
            }
        },
        "features": {
            "feature_new_ui": True,
            "feature_dark_mode": True,
            "feature_beta_api": False,
            "feature_analytics": True,
            "feature_ab_testing": True,
            "feature_rate_limiting": True
        },
        "api": {
            "version": "v2",
            "base_url": "/api",
            "rate_limit": {
                "requests_per_minute": 100,
                "burst": 20
            },
            "cors": {
                "allowed_origins": ["https://app.example.com", "https://admin.example.com"],
                "allowed_methods": ["GET", "POST", "PUT", "DELETE"],
                "max_age": 86400
            }
        },
        "auth": {
            "provider": "oauth2",
            "jwt": {
                "secret_env": "JWT_SECRET",
                "expiry_hours": 24,
                "refresh_expiry_days": 30
            },
            "oauth": {
                "google": {"client_id_env": "GOOGLE_CLIENT_ID", "enabled": True},
                "github": {"client_id_env": "GITHUB_CLIENT_ID", "enabled": True},
                "microsoft": {"client_id_env": "MS_CLIENT_ID", "enabled": False}
            }
        },
        "monitoring": {
            "metrics": {
                "enabled": True,
                "endpoint": "/metrics",
                "prefix": "myapp"
            },
            "tracing": {
                "enabled": True,
                "sample_rate": 0.1,
                "exporter": "jaeger"
            },
            "health": {
                "endpoint": "/health",
                "include_details": True
            }
        }
    }

    return datasets


def run_benchmark():
    """Run the complete benchmark suite."""
    datasets = generate_test_datasets()
    results = []

    print("=" * 100)
    print("TAUQ vs TOON vs JSON - Rigorous Token Efficiency Benchmark")
    print("Tokenizer: tiktoken cl100k_base (GPT-4/Claude compatible)")
    print("=" * 100)
    print()

    for name, data in datasets.items():
        print(f"Processing: {name}...")

        # Generate all formats
        json_minified = json.dumps(data, separators=(',', ':'))
        json_pretty = json.dumps(data, indent=2)

        # Generate all three tauq modes
        try:
            tauq_standard = json_to_tauq(data, "standard")
        except Exception as e:
            tauq_standard = f"ERROR: {e}"
            print(f"  tauq standard error: {e}")

        try:
            tauq_optimized = json_to_tauq(data, "optimized")
        except Exception as e:
            tauq_optimized = f"ERROR: {e}"
            print(f"  tauq optimized error: {e}")

        try:
            tauq_ultra = json_to_tauq(data, "ultra")
        except Exception as e:
            tauq_ultra = f"ERROR: {e}"
            print(f"  tauq ultra error: {e}")

        try:
            toon_output = json_to_toon(data)
        except Exception as e:
            toon_output = f"ERROR: {e}"
            print(f"  TOON error: {e}")

        # Count tokens
        json_min_tokens = count_tokens(json_minified)
        json_pretty_tokens = count_tokens(json_pretty)
        tauq_std_tokens = count_tokens(tauq_standard) if not tauq_standard.startswith("ERROR") else -1
        tauq_opt_tokens = count_tokens(tauq_optimized) if not tauq_optimized.startswith("ERROR") else -1
        tauq_ultra_tokens = count_tokens(tauq_ultra) if not tauq_ultra.startswith("ERROR") else -1
        toon_tokens = count_tokens(toon_output) if not toon_output.startswith("ERROR") else -1

        results.append({
            "dataset": name,
            "json_min_tokens": json_min_tokens,
            "json_pretty_tokens": json_pretty_tokens,
            "tauq_std_tokens": tauq_std_tokens,
            "tauq_opt_tokens": tauq_opt_tokens,
            "tauq_ultra_tokens": tauq_ultra_tokens,
            "toon_tokens": toon_tokens,
            "tauq_std_vs_json": f"{((tauq_std_tokens - json_min_tokens) / json_min_tokens * 100):+.1f}%" if tauq_std_tokens > 0 else "N/A",
            "tauq_opt_vs_json": f"{((tauq_opt_tokens - json_min_tokens) / json_min_tokens * 100):+.1f}%" if tauq_opt_tokens > 0 else "N/A",
            "tauq_ultra_vs_json": f"{((tauq_ultra_tokens - json_min_tokens) / json_min_tokens * 100):+.1f}%" if tauq_ultra_tokens > 0 else "N/A",
            "toon_vs_json": f"{((toon_tokens - json_min_tokens) / json_min_tokens * 100):+.1f}%" if toon_tokens > 0 else "N/A",
            "tauq_opt_vs_toon": f"{((tauq_opt_tokens - toon_tokens) / toon_tokens * 100):+.1f}%" if tauq_opt_tokens > 0 and toon_tokens > 0 else "N/A",
            "tauq_std_vs_toon": f"{((tauq_std_tokens - toon_tokens) / toon_tokens * 100):+.1f}%" if tauq_std_tokens > 0 and toon_tokens > 0 else "N/A",
        })

        # Save sample outputs for inspection
        output_dir = Path("/app/outputs")
        output_dir.mkdir(exist_ok=True)

        with open(output_dir / f"{name}.json", 'w') as f:
            f.write(json_minified)
        if not tauq_standard.startswith("ERROR"):
            with open(output_dir / f"{name}.tqn", 'w') as f:
                f.write(tauq_standard)
        if not tauq_optimized.startswith("ERROR"):
            with open(output_dir / f"{name}.opt.tqn", 'w') as f:
                f.write(tauq_optimized)
        if not tauq_ultra.startswith("ERROR"):
            with open(output_dir / f"{name}.ultra.tqn", 'w') as f:
                f.write(tauq_ultra)
        if not toon_output.startswith("ERROR"):
            with open(output_dir / f"{name}.toon", 'w') as f:
                f.write(toon_output)

    # Print results table
    print()
    print("=" * 140)
    print("RESULTS (Token Counts)")
    print("=" * 140)
    print(f"{'Dataset':<18} {'JSON':<8} {'tauq(std)':<10} {'tauq(opt)':<10} {'TOON':<8} {'std vs JSON':<12} {'TOON vs JSON':<12} {'std vs TOON':<12}")
    print("-" * 110)

    for r in results:
        print(f"{r['dataset']:<18} {r['json_min_tokens']:<8} {r['tauq_std_tokens']:<10} {r['tauq_opt_tokens']:<10} {r['toon_tokens']:<8} {r['tauq_std_vs_json']:<12} {r['toon_vs_json']:<12} {r['tauq_std_vs_toon']:<12}")

    print()

    # Summary statistics
    valid_results = [r for r in results if r['tauq_opt_tokens'] > 0 and r['toon_tokens'] > 0]
    if valid_results:
        total_json = sum(r['json_min_tokens'] for r in valid_results)
        total_tauq_std = sum(r['tauq_std_tokens'] for r in valid_results)
        total_tauq_opt = sum(r['tauq_opt_tokens'] for r in valid_results)
        total_tauq_ultra = sum(r['tauq_ultra_tokens'] for r in valid_results)
        total_toon = sum(r['toon_tokens'] for r in valid_results)

        print("=" * 70)
        print("SUMMARY (across all valid datasets)")
        print("=" * 70)
        print(f"Total JSON (minified):  {total_json:,} tokens (baseline)")
        print(f"Total tauq (standard):  {total_tauq_std:,} tokens ({((total_tauq_std - total_json) / total_json * 100):+.1f}% vs JSON)")
        print(f"Total tauq (optimized): {total_tauq_opt:,} tokens ({((total_tauq_opt - total_json) / total_json * 100):+.1f}% vs JSON)")
        print(f"Total tauq (ultra):     {total_tauq_ultra:,} tokens ({((total_tauq_ultra - total_json) / total_json * 100):+.1f}% vs JSON)")
        print(f"Total TOON:             {total_toon:,} tokens ({((total_toon - total_json) / total_json * 100):+.1f}% vs JSON)")
        print()
        print(f"tauq (standard) vs TOON: {((total_tauq_std - total_toon) / total_toon * 100):+.1f}%")
        print()

        # Compare standard mode (space-delimited) which is actually most efficient
        if total_tauq_std < total_toon:
            diff = total_toon - total_tauq_std
            pct = (diff / total_toon * 100)
            print(f">>> tauq (standard) WINS: saves {diff:,} tokens ({pct:.1f}%) compared to TOON <<<")
        elif total_tauq_std > total_toon:
            diff = total_tauq_std - total_toon
            pct = (diff / total_tauq_std * 100)
            print(f">>> TOON WINS: saves {diff:,} tokens ({pct:.1f}%) compared to tauq (standard) <<<")
        else:
            print(f">>> TIE: tauq (standard) and TOON have identical token counts <<<")

        print()
        print("NOTE: Space-delimited (standard) mode is more token-efficient than")
        print("      comma-delimited (optimized) mode due to cl100k_base tokenization.")

    # Save results as JSON for further analysis
    with open("/app/outputs/benchmark_results.json", 'w') as f:
        json.dump(results, f, indent=2)

    print()
    print("Sample outputs saved to /app/outputs/")

    return results


if __name__ == "__main__":
    run_benchmark()
