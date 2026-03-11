//! Codec Performance Benchmarking Suite
//!
//! Benchmarks adaptive compression codecs for TBF:
//! - Delta encoding: Sorted/sequential integer sequences
//! - Dictionary encoding: Repeated string/value patterns
//! - RLE encoding: Constant regions and bitmaps
//! - Raw encoding: Incompatible data (baseline)
//!
//! Run with: cargo bench --bench codec_benchmark

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use rand::Rng;
use serde_json::{Value, json};
use tauq::tbf::{
    CodecAnalyzer, CodecDecodingContext, CodecEncodingContext, CodecMetadata, CompressionCodec,
    encode_varint,
};

// ============================================================================
// Test Data Generation
// ============================================================================

/// Generate sorted integer sequence (optimal for Delta)
fn generate_sorted_integers(count: usize) -> Vec<Value> {
    (0..count).map(|i| json!(i as i64)).collect()
}

/// Generate monotonically increasing integers with small deltas (optimal for Delta)
fn generate_monotonic_integers(count: usize) -> Vec<Value> {
    let mut rng = rand::thread_rng();
    let mut result = Vec::new();
    let mut current: i64 = 0;
    for _ in 0..count {
        current += rng.gen_range(1..10);
        result.push(json!(current));
    }
    result
}

/// Generate repeated strings (optimal for Dictionary)
fn generate_repeated_strings(count: usize) -> Vec<Value> {
    let cities = ["New York", "London", "Tokyo", "Paris", "Sydney"];
    let mut result = Vec::new();
    for i in 0..count {
        result.push(json!(cities[i % cities.len()]));
    }
    result
}

/// Generate boolean sequence with runs (optimal for RLE)
fn generate_boolean_runs(count: usize) -> Vec<Value> {
    let mut result = Vec::new();
    let mut value = true;
    let mut run_length = 10;
    let mut remaining = count;

    while remaining > 0 {
        let take = std::cmp::min(run_length, remaining);
        for _ in 0..take {
            result.push(json!(value));
        }
        remaining -= take;
        value = !value;
        run_length = if run_length < 50 { run_length + 5 } else { 10 };
    }

    result.truncate(count);
    result
}

/// Generate random uncompressible data (Raw codec)
fn generate_random_values(count: usize) -> Vec<Value> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..count)
        .map(|_| match rng.gen_range(0..4) {
            0 => json!(rng.gen_range(0i64..1000000)),
            1 => json!(format!("str_{}", rng.gen_range(0u32..100000))),
            2 => json!(rng.gen_range(0..2) == 0),
            _ => json!(rng.gen_range(0.0..1.0)),
        })
        .collect()
}

// ============================================================================
// Codec Selection Benchmarks
// ============================================================================

fn bench_codec_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_selection");

    for size in [100, 1000, 10000].iter() {
        // Delta: Sorted integers
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(
            BenchmarkId::new("delta_selection", size),
            size,
            |b, &size| {
                let data = black_box(generate_sorted_integers(size));
                b.iter(|| {
                    let mut analyzer = CodecAnalyzer::new(100);
                    for value in &data[..std::cmp::min(100, size)] {
                        analyzer.add_sample(Some(value.clone()));
                    }
                    black_box(analyzer.choose_codec())
                });
            },
        );

        // Dictionary: Repeated strings
        group.bench_with_input(
            BenchmarkId::new("dictionary_selection", size),
            size,
            |b, &size| {
                let data = black_box(generate_repeated_strings(size));
                b.iter(|| {
                    let mut analyzer = CodecAnalyzer::new(100);
                    for value in &data[..std::cmp::min(100, size)] {
                        analyzer.add_sample(Some(value.clone()));
                    }
                    black_box(analyzer.choose_codec())
                });
            },
        );

        // RLE: Boolean runs
        group.bench_with_input(BenchmarkId::new("rle_selection", size), size, |b, &size| {
            let data = black_box(generate_boolean_runs(size));
            b.iter(|| {
                let mut analyzer = CodecAnalyzer::new(100);
                for value in &data[..std::cmp::min(100, size)] {
                    analyzer.add_sample(Some(value.clone()));
                }
                black_box(analyzer.choose_codec())
            });
        });
    }

    group.finish();
}

// ============================================================================
// Codec Encoding Benchmarks
// ============================================================================

fn bench_codec_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_encoding");

    // Delta encoding
    group.throughput(Throughput::Elements(1000));
    group.bench_function("delta_encoding_1000", |b| {
        b.iter(|| {
            let mut ctx = CodecEncodingContext::new(100);
            let data = black_box(generate_sorted_integers(1000));
            for value in &data {
                ctx.add_sample(Some(value));
            }
            for value in &data {
                let _ = ctx.encode_value(value);
            }
            black_box(ctx)
        });
    });

    // Dictionary encoding
    group.bench_function("dictionary_encoding_1000", |b| {
        b.iter(|| {
            let mut ctx = CodecEncodingContext::new(100);
            let data = black_box(generate_repeated_strings(1000));
            for value in &data {
                ctx.add_sample(Some(value));
            }
            for value in &data {
                let _ = ctx.encode_value(value);
            }
            black_box(ctx)
        });
    });

    // RLE encoding
    group.bench_function("rle_encoding_1000", |b| {
        b.iter(|| {
            let mut ctx = CodecEncodingContext::new(100);
            let data = black_box(generate_boolean_runs(1000));
            for value in &data {
                ctx.add_sample(Some(value));
            }
            for value in &data {
                let _ = ctx.encode_value(value);
            }
            black_box(ctx)
        });
    });

    // Raw encoding (baseline)
    group.bench_function("raw_encoding_1000", |b| {
        b.iter(|| {
            let mut ctx = CodecEncodingContext::new(100);
            let data = black_box(generate_random_values(1000));
            for value in &data {
                ctx.add_sample(Some(value));
            }
            for value in &data {
                let _ = ctx.encode_value(value);
            }
            black_box(ctx)
        });
    });

    group.finish();
}

// ============================================================================
// Compression Ratio Benchmarks
// ============================================================================

fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");
    group.measurement_time(std::time::Duration::from_secs(10));

    // Test Delta on sorted integers
    group.bench_function("delta_compression_10k", |b| {
        b.iter(|| {
            let data = black_box(generate_sorted_integers(10000));
            let mut encoded = Vec::new();

            // Simulate delta encoding: store deltas instead of full values
            if let Some(first) = data.first()
                && let Some(first_i64) = first.as_i64()
            {
                encode_varint(first_i64 as u64, &mut encoded);
                let mut prev = first_i64;
                for value in &data[1..] {
                    if let Some(curr) = value.as_i64() {
                        let delta = curr - prev;
                        encode_varint(delta as u64, &mut encoded);
                        prev = curr;
                    }
                }
            }
            black_box(encoded.len())
        });
    });

    // Test Dictionary on repeated strings
    group.bench_function("dictionary_compression_10k", |b| {
        b.iter(|| {
            let data = black_box(generate_repeated_strings(10000));
            let mut encoded = Vec::new();
            let mut dictionary = std::collections::HashMap::new();
            let mut dict_counter = 0u32;

            // Build dictionary and encode indices
            for value in &data {
                if let Some(s) = value.as_str() {
                    if !dictionary.contains_key(s) {
                        dictionary.insert(s.to_string(), dict_counter);
                        dict_counter += 1;
                    }
                    if let Some(idx) = dictionary.get(s) {
                        encode_varint(*idx as u64, &mut encoded);
                    }
                }
            }
            black_box(encoded.len())
        });
    });

    group.finish();
}

// ============================================================================
// Metadata Serialization Benchmarks
// ============================================================================

fn bench_metadata_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_serialization");

    group.bench_function("delta_metadata_encode", |b| {
        b.iter(|| {
            let metadata = black_box(CodecMetadata::Delta {
                initial_value: 12345,
            });
            let encoded = metadata.encode();
            black_box(encoded.len())
        });
    });

    group.bench_function("dictionary_metadata_encode", |b| {
        b.iter(|| {
            let metadata = black_box(CodecMetadata::Dictionary {
                dictionary_size: 512,
            });
            let encoded = metadata.encode();
            black_box(encoded.len())
        });
    });

    group.bench_function("rle_metadata_encode", |b| {
        b.iter(|| {
            let metadata = black_box(CodecMetadata::RLE);
            let encoded = metadata.encode();
            black_box(encoded.len())
        });
    });

    group.finish();
}

// ============================================================================
// Codec Decoding Benchmarks
// ============================================================================

fn bench_codec_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_decoding");

    // Delta decoding
    group.bench_function("delta_decoding_1000", |b| {
        b.iter(|| {
            let data = black_box(generate_sorted_integers(1000));
            let mut ctx = CodecDecodingContext::from_metadata(
                CompressionCodec::Delta,
                CodecMetadata::Delta { initial_value: 0 },
            );
            ctx.initialize_decoders();

            for value in &data {
                let _ = ctx.decode_value(value);
            }
            black_box(ctx)
        });
    });

    // Dictionary decoding
    group.bench_function("dictionary_decoding_1000", |b| {
        b.iter(|| {
            let data = black_box(generate_repeated_strings(1000));
            let mut ctx = CodecDecodingContext::from_metadata(
                CompressionCodec::Dictionary,
                CodecMetadata::Dictionary {
                    dictionary_size: 10,
                },
            );
            ctx.initialize_decoders();

            for value in &data {
                let _ = ctx.decode_value(value);
            }
            black_box(ctx)
        });
    });

    // Raw decoding (baseline)
    group.bench_function("raw_decoding_1000", |b| {
        b.iter(|| {
            let data = black_box(generate_random_values(1000));
            let mut ctx =
                CodecDecodingContext::from_metadata(CompressionCodec::Raw, CodecMetadata::None);
            ctx.initialize_decoders();

            for value in &data {
                let _ = ctx.decode_value(value);
            }
            black_box(ctx)
        });
    });

    group.finish();
}

// ============================================================================
// Real-world Data Pattern Benchmarks
// ============================================================================

fn bench_real_world_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_patterns");

    // Time-series data (Delta optimal)
    group.throughput(Throughput::Elements(10000));
    group.bench_function("timeseries_delta_10k", |b| {
        b.iter(|| {
            let data = black_box(generate_monotonic_integers(10000));
            let mut ctx = CodecEncodingContext::new(100);

            for value in &data {
                ctx.add_sample(Some(value));
            }

            for value in &data {
                let _ = ctx.encode_value(value);
            }

            black_box((ctx.get_selected_codec(), data.len()))
        });
    });

    // User location data (Dictionary optimal)
    group.bench_function("location_dictionary_10k", |b| {
        b.iter(|| {
            let data = black_box(generate_repeated_strings(10000));
            let mut ctx = CodecEncodingContext::new(100);

            for value in &data {
                ctx.add_sample(Some(value));
            }

            for value in &data {
                let _ = ctx.encode_value(value);
            }

            black_box((ctx.get_selected_codec(), data.len()))
        });
    });

    // Feature flags/bitmaps (RLE optimal)
    group.bench_function("flags_rle_10k", |b| {
        b.iter(|| {
            let data = black_box(generate_boolean_runs(10000));
            let mut ctx = CodecEncodingContext::new(100);

            for value in &data {
                ctx.add_sample(Some(value));
            }

            for value in &data {
                let _ = ctx.encode_value(value);
            }

            black_box((ctx.get_selected_codec(), data.len()))
        });
    });

    group.finish();
}

// ============================================================================
// Codec Overhead Benchmarks
// ============================================================================

fn bench_codec_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_overhead");

    // Sampling overhead (should be < 1% of total encoding time)
    group.bench_function("sampling_overhead_1000", |b| {
        b.iter(|| {
            let data = black_box(generate_sorted_integers(1000));
            let mut analyzer = CodecAnalyzer::new(100);

            // Sampling phase
            for value in &data[..100] {
                analyzer.add_sample(Some(value.clone()));
            }

            let codec = analyzer.choose_codec();
            black_box(codec)
        });
    });

    // Metadata encoding overhead (should be < 5% of total output)
    group.bench_function("metadata_overhead", |b| {
        b.iter(|| {
            let mut total_size = 0;

            let delta_meta = CodecMetadata::Delta {
                initial_value: 100000,
            };
            total_size += delta_meta.encode().len();

            let dict_meta = CodecMetadata::Dictionary {
                dictionary_size: 1000,
            };
            total_size += dict_meta.encode().len();

            let rle_meta = CodecMetadata::RLE;
            total_size += rle_meta.encode().len();

            black_box(total_size)
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Setup
// ============================================================================

criterion_group!(
    benches,
    bench_codec_selection,
    bench_codec_encoding,
    bench_compression_ratio,
    bench_metadata_serialization,
    bench_codec_decoding,
    bench_real_world_patterns,
    bench_codec_overhead,
);

criterion_main!(benches);
