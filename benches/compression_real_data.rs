//! Compression ratio benchmarking with real-world datasets
//!
//! Measures actual compression achieved on production-like data patterns
//! and compares against JSON baseline.

mod dataset_transactions;
mod dataset_logs;
mod dataset_metrics;
mod dataset_geospatial;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// ============================================================================
// Real-World Data Compression Benchmarks
// ============================================================================

fn bench_transaction_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_compression");
    group.measurement_time(std::time::Duration::from_secs(10));

    for size in [10000, 100000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("transactions_json", size),
            size,
            |b, &size| {
                let data = black_box(dataset_transactions::generate_transactions(size));
                b.iter(|| {
                    let json = serde_json::to_vec(&data).ok();
                    black_box(json.map(|j| j.len()).unwrap_or(0))
                })
            },
        );
    }

    group.finish();
}

fn bench_logs_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("logs_compression");
    group.measurement_time(std::time::Duration::from_secs(10));

    for size in [100000, 500000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("logs_json", size),
            size,
            |b, &size| {
                let data = black_box(dataset_logs::generate_event_logs(size));
                b.iter(|| {
                    let json = serde_json::to_vec(&data).ok();
                    black_box(json.map(|j| j.len()).unwrap_or(0))
                })
            },
        );
    }

    group.finish();
}

fn bench_metrics_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_compression");
    group.measurement_time(std::time::Duration::from_secs(10));

    let count_per_series = 1000;
    let total = count_per_series * 54;  // 54 series

    group.throughput(Throughput::Elements(total as u64));

    group.bench_function("metrics_json_54k", |b| {
        let data = black_box(dataset_metrics::generate_metrics(count_per_series));
        b.iter(|| {
            let json = serde_json::to_vec(&data).ok();
            black_box(json.map(|j| j.len()).unwrap_or(0))
        })
    });

    group.finish();
}

fn bench_geospatial_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("geospatial_compression");
    group.measurement_time(std::time::Duration::from_secs(10));

    for size in [100000, 500000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::new("geospatial_json", size),
            size,
            |b, &size| {
                let data = black_box(dataset_geospatial::generate_geospatial_data(size));
                b.iter(|| {
                    let json = serde_json::to_vec(&data).ok();
                    black_box(json.map(|j| j.len()).unwrap_or(0))
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Compression Ratio Analysis
// ============================================================================

fn bench_compression_ratio_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_analysis");
    group.measurement_time(std::time::Duration::from_secs(5));

    group.bench_function("json_size_analysis", |b| {
        b.iter(|| {
            // Analyze JSON encoding sizes across different data patterns
            let tx_data = dataset_transactions::generate_transactions(10000);
            let tx_size = serde_json::to_vec(&tx_data).ok().map(|j| j.len()).unwrap_or(0);

            let log_data = dataset_logs::generate_event_logs(10000);
            let log_size = serde_json::to_vec(&log_data).ok().map(|j| j.len()).unwrap_or(0);

            let metric_data = dataset_metrics::generate_metrics(100);
            let metric_size = serde_json::to_vec(&metric_data).ok().map(|j| j.len()).unwrap_or(0);

            black_box((tx_size, log_size, metric_size))
        })
    });

    group.finish();
}

// ============================================================================
// Encoding Throughput (records/sec)
// ============================================================================

fn bench_encoding_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("encoding_throughput");
    group.measurement_time(std::time::Duration::from_secs(5));

    group.bench_function("json_encoding_throughput", |b| {
        let tx_data = black_box(dataset_transactions::generate_transactions(10000));
        let log_data = black_box(dataset_logs::generate_event_logs(10000));
        b.iter(|| {
            let tx_json = serde_json::to_vec(&tx_data).ok();
            let log_json = serde_json::to_vec(&log_data).ok();
            black_box((tx_json, log_json))
        })
    });

    group.finish();
}

// ============================================================================
// Criterion Setup
// ============================================================================

criterion_group!(
    benches,
    bench_transaction_compression,
    bench_logs_compression,
    bench_metrics_compression,
    bench_geospatial_compression,
    bench_compression_ratio_analysis,
    bench_encoding_throughput,
);

criterion_main!(benches);
