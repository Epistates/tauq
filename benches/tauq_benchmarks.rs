use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tauq::{json_to_tauq, json_to_tauq_optimized};
use serde_json::json;

/// Generate sample datasets for benchmarking
mod datasets {
    use serde_json::{json, Value};

    pub fn small_flat() -> Value {
        json!([
            {"id": 1, "name": "Alice", "age": 30, "active": true},
            {"id": 2, "name": "Bob", "age": 25, "active": false},
            {"id": 3, "name": "Carol", "age": 35, "active": true},
        ])
    }

    pub fn medium_flat() -> Value {
        let departments = vec!["Engineering", "Sales", "Marketing", "HR"];
        let employees: Vec<_> = (1..=100).map(|i| {
            json!({
                "id": i,
                "name": format!("Employee{}", i),
                "email": format!("employee{}@company.com", i),
                "department": departments[(i - 1) % 4],
                "salary": 45000 + (i * 1000),
                "active": i % 3 != 0
            })
        }).collect();
        json!({"employees": employees})
    }

    pub fn large_flat() -> Value {
        let records: Vec<_> = (1..=1000).map(|i| {
            json!({
                "id": i,
                "timestamp": format!("2025-01-01T{:02}:00:00Z", i % 24),
                "value": 100 + (i % 50),
                "status": if i % 2 == 0 { "active" } else { "inactive" }
            })
        }).collect();
        json!(records)
    }

    pub fn nested_structure() -> Value {
        let orders: Vec<_> = (1..=50).map(|i| {
            json!({
                "orderId": format!("ORD-{:04}", i),
                "customer": {
                    "id": i,
                    "name": format!("Customer{}", i),
                    "email": format!("customer{}@example.com", i)
                },
                "items": [
                    {"sku": format!("SKU-{}", i * 10), "name": "Product A", "quantity": 2, "price": 29.99},
                    {"sku": format!("SKU-{}", i * 10 + 1), "name": "Product B", "quantity": 1, "price": 49.99}
                ],
                "total": 109.97
            })
        }).collect();
        json!({"orders": orders})
    }

    pub fn deep_nesting() -> Value {
        json!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": {
                            "level5": {
                                "data": [1, 2, 3, 4, 5],
                                "metadata": {
                                    "created": "2025-01-01",
                                    "author": "System"
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    pub fn heterogeneous() -> Value {
        json!([
            {"id": 1, "name": "User1", "role": "admin"},
            {"id": 2, "name": "User2", "department": "Engineering"},
            {"id": 3, "name": "User3", "tags": ["dev", "py"]},
            {"id": 4, "email": "user4@example.com"},
            {"id": 5, "name": "User5", "metadata": {"level": 5}},
        ])
    }
}

/// Benchmark parsing performance (JSON string -> Tauq value)
fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    // Small flat dataset
    let small_json = serde_json::to_string(&datasets::small_flat()).unwrap();
    group.bench_function("small_flat", |b| {
        b.iter(|| {
            let value: serde_json::Value = serde_json::from_str(black_box(&small_json)).unwrap();
            black_box(value);
        });
    });

    // Medium flat dataset
    let medium_json = serde_json::to_string(&datasets::medium_flat()).unwrap();
    group.bench_function("medium_flat", |b| {
        b.iter(|| {
            let value: serde_json::Value = serde_json::from_str(black_box(&medium_json)).unwrap();
            black_box(value);
        });
    });

    // Large flat dataset
    let large_json = serde_json::to_string(&datasets::large_flat()).unwrap();
    group.bench_function("large_flat", |b| {
        b.iter(|| {
            let value: serde_json::Value = serde_json::from_str(black_box(&large_json)).unwrap();
            black_box(value);
        });
    });

    group.finish();
}

/// Benchmark formatting performance (Tauq value -> string)
fn bench_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("format");

    // Small flat dataset
    let small = datasets::small_flat();
    group.bench_function("small_flat", |b| {
        b.iter(|| {
            let output = json_to_tauq(black_box(&small));
            black_box(output);
        });
    });

    // Medium flat dataset
    let medium = datasets::medium_flat();
    group.bench_function("medium_flat", |b| {
        b.iter(|| {
            let output = json_to_tauq(black_box(&medium));
            black_box(output);
        });
    });

    // Large flat dataset
    let large = datasets::large_flat();
    group.bench_function("large_flat", |b| {
        b.iter(|| {
            let output = json_to_tauq(black_box(&large));
            black_box(output);
        });
    });

    // Nested structure
    let nested = datasets::nested_structure();
    group.bench_function("nested_structure", |b| {
        b.iter(|| {
            let output = json_to_tauq(black_box(&nested));
            black_box(output);
        });
    });

    // Deep nesting
    let deep = datasets::deep_nesting();
    group.bench_function("deep_nesting", |b| {
        b.iter(|| {
            let output = json_to_tauq(black_box(&deep));
            black_box(output);
        });
    });

    // Heterogeneous
    let hetero = datasets::heterogeneous();
    group.bench_function("heterogeneous", |b| {
        b.iter(|| {
            let output = json_to_tauq(black_box(&hetero));
            black_box(output);
        });
    });

    group.finish();
}

/// Benchmark optimized formatting (compact mode)
fn bench_format_optimized(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_optimized");

    // Medium flat dataset
    let medium = datasets::medium_flat();
    group.bench_function("medium_flat", |b| {
        b.iter(|| {
            let output = json_to_tauq_optimized(black_box(&medium));
            black_box(output);
        });
    });

    // Nested structure
    let nested = datasets::nested_structure();
    group.bench_function("nested_structure", |b| {
        b.iter(|| {
            let output = json_to_tauq_optimized(black_box(&nested));
            black_box(output);
        });
    });

    group.finish();
}

/// Benchmark round-trip performance (JSON -> Tauq -> back)
fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    let medium = datasets::medium_flat();
    group.bench_function("medium_flat", |b| {
        b.iter(|| {
            let json_str = serde_json::to_string(black_box(&medium)).unwrap();
            let tauq_str = json_to_tauq(black_box(&medium));
            black_box((json_str, tauq_str));
        });
    });

    let nested = datasets::nested_structure();
    group.bench_function("nested_structure", |b| {
        b.iter(|| {
            let json_str = serde_json::to_string(black_box(&nested)).unwrap();
            let tauq_str = json_to_tauq(black_box(&nested));
            black_box((json_str, tauq_str));
        });
    });

    group.finish();
}

/// Benchmark different dataset sizes to measure scalability
fn bench_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalability");

    for size in [10, 50, 100, 500, 1000].iter() {
        let data: Vec<_> = (1..=*size).map(|i| {
            json!({
                "id": i,
                "name": format!("Item{}", i),
                "value": i * 10,
                "active": i % 2 == 0
            })
        }).collect();
        let value = json!(data);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let output = json_to_tauq(black_box(&value));
                black_box(output);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_format,
    bench_format_optimized,
    bench_roundtrip,
    bench_scalability
);
criterion_main!(benches);
