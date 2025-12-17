//! Binary Format Benchmark
//!
//! Comprehensive benchmark comparing Tauq's binary serialization formats
//! against JSON, Parquet, and Protocol Buffers.
//!
//! Run with: cargo bench --bench binary_format_benchmark --features binary

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use serde::{Deserialize, Serialize};
use rand::Rng;
use std::io::Cursor;

// ============================================================================
// Test Data Structures
// ============================================================================

/// Employee record matching our benchmark schema
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Employee {
    pub id: u32,
    pub name: String,
    pub age: u32,
    pub city: String,
    pub department: String,
    pub salary: u32,
    pub experience: u32,
    pub project_count: u32,
}

/// Generate synthetic employee dataset
fn generate_employees(count: usize, seed: u64) -> Vec<Employee> {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    let first_names = [
        "Alice", "Bob", "Carol", "Dave", "Eve", "Frank", "Grace", "Hank",
        "Ivy", "Jack", "Kate", "Liam", "Maya", "Noah", "Olivia", "Pete",
    ];
    let cities = [
        "NYC", "LA", "Chicago", "Houston", "Phoenix", "Philadelphia",
        "Seattle", "Denver", "Boston", "Austin",
    ];
    let departments = [
        "Engineering", "Sales", "Marketing", "HR", "Finance",
        "Operations", "Support", "Legal", "Product", "Design",
    ];

    (0..count)
        .map(|i| {
            let first_name = first_names[rng.gen_range(0..first_names.len())];
            let suffix = format!("{}{:03}", (b'A' + (i / 1000) as u8 % 26) as char, i % 1000);
            Employee {
                id: i as u32 + 1,
                name: format!("{} {}", first_name, suffix),
                age: rng.gen_range(22..65),
                city: cities[rng.gen_range(0..cities.len())].to_string(),
                department: departments[rng.gen_range(0..departments.len())].to_string(),
                salary: rng.gen_range(40000..180000),
                experience: rng.gen_range(0..35),
                project_count: rng.gen_range(1..50),
            }
        })
        .collect()
}

/// Convert employees to Tauq format
fn employees_to_tauq(employees: &[Employee]) -> String {
    let mut lines = vec!["!def Employee id name age city department salary experience project_count".to_string()];
    for emp in employees {
        lines.push(format!(
            "{} \"{}\" {} {} {} {} {} {}",
            emp.id, emp.name, emp.age, emp.city, emp.department, emp.salary, emp.experience, emp.project_count
        ));
    }
    lines.join("\n")
}

/// Convert employees to JSON
fn employees_to_json(employees: &[Employee]) -> String {
    serde_json::to_string(employees).unwrap()
}

// ============================================================================
// Tauq Binary Format Benchmarks
// ============================================================================

#[cfg(feature = "bitcode")]
fn bench_bitcode(c: &mut Criterion) {
    let mut group = c.benchmark_group("bitcode");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let bytes = tauq::binary::to_bitcode(&employees).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| {
                b.iter(|| tauq::binary::to_bitcode(black_box(data)).unwrap())
            },
        );

        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &bytes,
            |b, data| {
                b.iter(|| {
                    let _: Vec<Employee> = tauq::binary::from_bitcode(black_box(data)).unwrap();
                })
            },
        );
    }

    group.finish();
}

#[cfg(feature = "bincode")]
fn bench_bincode(c: &mut Criterion) {
    let mut group = c.benchmark_group("bincode");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let bytes = tauq::binary::to_bincode(&employees).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| {
                b.iter(|| tauq::binary::to_bincode(black_box(data)).unwrap())
            },
        );

        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &bytes,
            |b, data| {
                b.iter(|| {
                    let _: Vec<Employee> = tauq::binary::from_bincode(black_box(data)).unwrap();
                })
            },
        );
    }

    group.finish();
}

#[cfg(feature = "postcard")]
fn bench_postcard(c: &mut Criterion) {
    let mut group = c.benchmark_group("postcard");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let bytes = tauq::binary::to_postcard(&employees).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| {
                b.iter(|| tauq::binary::to_postcard(black_box(data)).unwrap())
            },
        );

        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &bytes,
            |b, data| {
                b.iter(|| {
                    let _: Vec<Employee> = tauq::binary::from_postcard(black_box(data)).unwrap();
                })
            },
        );
    }

    group.finish();
}

#[cfg(feature = "rmp-serde")]
fn bench_msgpack(c: &mut Criterion) {
    let mut group = c.benchmark_group("msgpack");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let bytes = tauq::binary::to_msgpack(&employees).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| {
                b.iter(|| tauq::binary::to_msgpack(black_box(data)).unwrap())
            },
        );

        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &bytes,
            |b, data| {
                b.iter(|| {
                    let _: Vec<Employee> = tauq::binary::from_msgpack(black_box(data)).unwrap();
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Comparison: JSON
// ============================================================================

fn bench_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("json");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let json_str = serde_json::to_string(&employees).unwrap();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &employees,
            |b, data| {
                b.iter(|| serde_json::to_string(black_box(data)).unwrap())
            },
        );

        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &json_str,
            |b, data| {
                b.iter(|| {
                    let _: Vec<Employee> = serde_json::from_str(black_box(data)).unwrap();
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Comparison: Tauq Text Format
// ============================================================================

fn bench_tauq_text(c: &mut Criterion) {
    let mut group = c.benchmark_group("tauq_text");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);
        let tauq_str = employees_to_tauq(&employees);

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("format", size),
            &employees,
            |b, data| {
                b.iter(|| employees_to_tauq(black_box(data)))
            },
        );

        group.bench_with_input(
            BenchmarkId::new("parse", size),
            &tauq_str,
            |b, data| {
                b.iter(|| tauq::compile_tauq(black_box(data)).unwrap())
            },
        );
    }

    group.finish();
}

// ============================================================================
// Comparison: Parquet (via Polars)
// ============================================================================

fn bench_parquet(c: &mut Criterion) {
    use polars::prelude::*;

    let mut group = c.benchmark_group("parquet");

    for size in [100, 1000, 10000] {
        let employees = generate_employees(size, 42);

        // Convert to DataFrame
        let ids: Vec<u32> = employees.iter().map(|e| e.id).collect();
        let names: Vec<&str> = employees.iter().map(|e| e.name.as_str()).collect();
        let ages: Vec<u32> = employees.iter().map(|e| e.age).collect();
        let cities: Vec<&str> = employees.iter().map(|e| e.city.as_str()).collect();
        let departments: Vec<&str> = employees.iter().map(|e| e.department.as_str()).collect();
        let salaries: Vec<u32> = employees.iter().map(|e| e.salary).collect();
        let experiences: Vec<u32> = employees.iter().map(|e| e.experience).collect();
        let project_counts: Vec<u32> = employees.iter().map(|e| e.project_count).collect();

        let df = DataFrame::new(vec![
            Series::new("id".into(), ids.clone()),
            Series::new("name".into(), names.clone()),
            Series::new("age".into(), ages.clone()),
            Series::new("city".into(), cities.clone()),
            Series::new("department".into(), departments.clone()),
            Series::new("salary".into(), salaries.clone()),
            Series::new("experience".into(), experiences.clone()),
            Series::new("project_count".into(), project_counts.clone()),
        ]).unwrap();

        // Serialize to parquet bytes
        let mut parquet_bytes = Vec::new();
        {
            let cursor = Cursor::new(&mut parquet_bytes);
            ParquetWriter::new(cursor).finish(&mut df.clone()).unwrap();
        }

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("serialize", size),
            &df,
            |b, data| {
                b.iter(|| {
                    let mut bytes = Vec::new();
                    let cursor = Cursor::new(&mut bytes);
                    ParquetWriter::new(cursor).finish(&mut data.clone()).unwrap();
                    bytes
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("deserialize", size),
            &parquet_bytes,
            |b, data| {
                b.iter(|| {
                    let cursor = Cursor::new(black_box(data));
                    ParquetReader::new(cursor).finish().unwrap()
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Size Comparison
// ============================================================================

fn bench_size_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("size_comparison");

    for size in [100, 1000] {
        let employees = generate_employees(size, 42);

        // Calculate sizes
        let json_size = serde_json::to_string(&employees).unwrap().len();
        let tauq_size = employees_to_tauq(&employees).len();

        #[cfg(feature = "bitcode")]
        let bitcode_size = tauq::binary::to_bitcode(&employees).unwrap().len();
        #[cfg(feature = "bincode")]
        let bincode_size = tauq::binary::to_bincode(&employees).unwrap().len();
        #[cfg(feature = "postcard")]
        let postcard_size = tauq::binary::to_postcard(&employees).unwrap().len();
        #[cfg(feature = "rmp-serde")]
        let msgpack_size = tauq::binary::to_msgpack(&employees).unwrap().len();

        // Parquet size
        use polars::prelude::*;
        let ids: Vec<u32> = employees.iter().map(|e| e.id).collect();
        let names: Vec<&str> = employees.iter().map(|e| e.name.as_str()).collect();
        let ages: Vec<u32> = employees.iter().map(|e| e.age).collect();
        let cities: Vec<&str> = employees.iter().map(|e| e.city.as_str()).collect();
        let departments: Vec<&str> = employees.iter().map(|e| e.department.as_str()).collect();
        let salaries: Vec<u32> = employees.iter().map(|e| e.salary).collect();
        let experiences: Vec<u32> = employees.iter().map(|e| e.experience).collect();
        let project_counts: Vec<u32> = employees.iter().map(|e| e.project_count).collect();

        let df = DataFrame::new(vec![
            Series::new("id".into(), ids),
            Series::new("name".into(), names),
            Series::new("age".into(), ages),
            Series::new("city".into(), cities),
            Series::new("department".into(), departments),
            Series::new("salary".into(), salaries),
            Series::new("experience".into(), experiences),
            Series::new("project_count".into(), project_counts),
        ]).unwrap();

        let mut parquet_bytes = Vec::new();
        {
            let cursor = Cursor::new(&mut parquet_bytes);
            ParquetWriter::new(cursor).finish(&mut df.clone()).unwrap();
        }
        let parquet_size = parquet_bytes.len();

        println!("\n=== Size Comparison ({} records) ===", size);
        println!("JSON:     {:>8} bytes (baseline)", json_size);
        println!("Tauq:     {:>8} bytes ({:.1}% of JSON)", tauq_size, (tauq_size as f64 / json_size as f64) * 100.0);
        #[cfg(feature = "bitcode")]
        println!("Bitcode:  {:>8} bytes ({:.1}% of JSON)", bitcode_size, (bitcode_size as f64 / json_size as f64) * 100.0);
        #[cfg(feature = "bincode")]
        println!("Bincode:  {:>8} bytes ({:.1}% of JSON)", bincode_size, (bincode_size as f64 / json_size as f64) * 100.0);
        #[cfg(feature = "postcard")]
        println!("Postcard: {:>8} bytes ({:.1}% of JSON)", postcard_size, (postcard_size as f64 / json_size as f64) * 100.0);
        #[cfg(feature = "rmp-serde")]
        println!("MsgPack:  {:>8} bytes ({:.1}% of JSON)", msgpack_size, (msgpack_size as f64 / json_size as f64) * 100.0);
        println!("Parquet:  {:>8} bytes ({:.1}% of JSON)", parquet_size, (parquet_size as f64 / json_size as f64) * 100.0);

        // Dummy benchmark just to have something in the group
        group.bench_function(BenchmarkId::new("size_calc", size), |b| {
            b.iter(|| {
                black_box(json_size + tauq_size)
            })
        });
    }

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

#[cfg(feature = "bitcode")]
criterion_group!(
    bitcode_benches,
    bench_bitcode,
);

#[cfg(feature = "bincode")]
criterion_group!(
    bincode_benches,
    bench_bincode,
);

#[cfg(feature = "postcard")]
criterion_group!(
    postcard_benches,
    bench_postcard,
);

#[cfg(feature = "rmp-serde")]
criterion_group!(
    msgpack_benches,
    bench_msgpack,
);

criterion_group!(
    comparison_benches,
    bench_json,
    bench_tauq_text,
    bench_parquet,
    bench_size_comparison,
);

#[cfg(all(feature = "bitcode", feature = "bincode", feature = "postcard", feature = "rmp-serde"))]
criterion_main!(
    bitcode_benches,
    bincode_benches,
    postcard_benches,
    msgpack_benches,
    comparison_benches,
);

#[cfg(not(all(feature = "bitcode", feature = "bincode", feature = "postcard", feature = "rmp-serde")))]
criterion_main!(comparison_benches);
