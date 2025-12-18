//! Real-world time-series metrics dataset for codec testing
//!
//! Simulates system monitoring metrics with:
//! - Repeated series IDs (dictionary optimal)
//! - Sequential timestamps (delta optimal)
//! - Small value deltas (delta optimal)
//! - Normalized ranges 0-100 (good for compression)
//! - Natural patterns (burst/idle cycles)

use serde_json::{json, Value};
use rand::Rng;

/// Common series names in a metrics system
const SERIES: &[&str] = &[
    "cpu.us.east.server01", "cpu.us.east.server02", "cpu.us.east.server03",
    "cpu.us.west.server01", "cpu.us.west.server02", "cpu.us.west.server03",
    "cpu.eu.server01", "cpu.eu.server02", "cpu.eu.server03",
    "memory.us.east.server01", "memory.us.east.server02", "memory.us.east.server03",
    "memory.us.west.server01", "memory.us.west.server02", "memory.us.west.server03",
    "memory.eu.server01", "memory.eu.server02", "memory.eu.server03",
    "disk.us.east.server01", "disk.us.east.server02", "disk.us.east.server03",
    "disk.us.west.server01", "disk.us.west.server02", "disk.us.west.server03",
    "disk.eu.server01", "disk.eu.server02", "disk.eu.server03",
    "network.us.east.server01", "network.us.east.server02", "network.us.east.server03",
    "network.us.west.server01", "network.us.west.server02", "network.us.west.server03",
    "network.eu.server01", "network.eu.server02", "network.eu.server03",
    "latency.api", "latency.database", "latency.cache",
    "requests.api", "requests.web", "requests.auth",
    "errors.api", "errors.database", "errors.network",
    "cache.hits", "cache.misses", "cache.evictions",
    "queue.depth", "queue.latency", "queue.throughput",
    "db.connections", "db.slow_queries", "db.locks",
];

/// Metric metadata for realistic generation
struct MetricParams {
    min: f32,
    max: f32,
    typical_value: f32,
    volatility: f32,  // Standard deviation of changes
    burst_probability: f32,
}

fn get_metric_params(series: &str) -> MetricParams {
    if series.starts_with("cpu") {
        MetricParams {
            min: 0.0,
            max: 100.0,
            typical_value: 30.0,
            volatility: 5.0,
            burst_probability: 0.05,
        }
    } else if series.starts_with("memory") {
        MetricParams {
            min: 0.0,
            max: 100.0,
            typical_value: 60.0,
            volatility: 3.0,
            burst_probability: 0.02,
        }
    } else if series.starts_with("disk") {
        MetricParams {
            min: 0.0,
            max: 100.0,
            typical_value: 70.0,
            volatility: 1.0,
            burst_probability: 0.01,
        }
    } else if series.starts_with("network") {
        MetricParams {
            min: 0.0,
            max: 10000.0,
            typical_value: 2000.0,
            volatility: 500.0,
            burst_probability: 0.1,
        }
    } else if series.starts_with("latency") {
        MetricParams {
            min: 0.0,
            max: 5000.0,
            typical_value: 50.0,
            volatility: 10.0,
            burst_probability: 0.05,
        }
    } else if series.starts_with("requests") {
        MetricParams {
            min: 0.0,
            max: 100000.0,
            typical_value: 5000.0,
            volatility: 500.0,
            burst_probability: 0.1,
        }
    } else if series.starts_with("errors") {
        MetricParams {
            min: 0.0,
            max: 1000.0,
            typical_value: 10.0,
            volatility: 5.0,
            burst_probability: 0.02,
        }
    } else if series.starts_with("cache") {
        MetricParams {
            min: 0.0,
            max: 1000000.0,
            typical_value: 50000.0,
            volatility: 10000.0,
            burst_probability: 0.05,
        }
    } else if series.starts_with("queue") {
        MetricParams {
            min: 0.0,
            max: 10000.0,
            typical_value: 100.0,
            volatility: 50.0,
            burst_probability: 0.1,
        }
    } else {
        MetricParams {
            min: 0.0,
            max: 1000.0,
            typical_value: 100.0,
            volatility: 50.0,
            burst_probability: 0.05,
        }
    }
}

/// Generate realistic time-series metrics data
///
/// # Arguments
/// * `count_per_series` - Number of data points per series
///
/// # Returns
/// Vec of metric JSON values across all series
pub fn generate_metrics(count_per_series: usize) -> Vec<Value> {
    let mut rng = rand::thread_rng();
    let base_timestamp = 1766534400i64;  // Dec 17, 2025 00:00:00 UTC
    let mut result = Vec::new();

    for series in SERIES {
        let params = get_metric_params(series);
        let mut current_value = params.typical_value;

        for i in 0..count_per_series {
            // Simulate realistic metric behavior
            let change = if rng.gen_bool(params.burst_probability as f64) {
                // Burst: large sudden change
                rng.gen_range(-100.0..100.0) * params.volatility
            } else {
                // Normal: small random walk
                (rng.gen_range(-1.0..1.0) * params.volatility) as f32
            };

            current_value = (current_value + change)
                .max(params.min)
                .min(params.max);

            result.push(json!({
                "series_id": series,
                "timestamp": base_timestamp + (i as i64 * 60),  // 1 minute intervals
                "value": (current_value * 100.0).round() / 100.0,  // 2 decimal places
                "host": if series.contains("us.east") { "us-east-1" }
                        else if series.contains("us.west") { "us-west-1" }
                        else { "eu-west-1" },
            }));
        }
    }

    result
}

/// Generate metrics with specific seed
pub fn generate_metrics_with_seed(count_per_series: usize, seed: u64) -> Vec<Value> {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let base_timestamp = 1766534400i64;
    let mut result = Vec::new();

    for series in SERIES {
        let params = get_metric_params(series);
        let mut current_value = params.typical_value;

        for i in 0..count_per_series {
            let change = if rng.gen_bool(params.burst_probability as f64) {
                rng.gen_range(-100.0..100.0) * params.volatility
            } else {
                (rng.gen_range(-1.0..1.0) * params.volatility) as f32
            };

            current_value = (current_value + change)
                .max(params.min)
                .min(params.max);

            result.push(json!({
                "series_id": series,
                "timestamp": base_timestamp + (i as i64 * 60),
                "value": (current_value * 100.0).round() / 100.0,
                "host": if series.contains("us.east") { "us-east-1" }
                        else if series.contains("us.west") { "us-west-1" }
                        else { "eu-west-1" },
            }));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_metrics_basic() {
        let data = generate_metrics(100);

        // 54 series × 100 points each = 5400 total
        assert_eq!(data.len(), SERIES.len() * 100);

        // Verify structure
        for metric in &data {
            assert!(metric["series_id"].is_string());
            assert!(metric["timestamp"].is_i64());
            assert!(metric["value"].is_f64());
            assert!(metric["host"].is_string());
        }
    }

    #[test]
    fn test_metrics_temporal_ordering_per_series() {
        let data = generate_metrics(500);

        // Group by series
        let mut series_data: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
        for metric in &data {
            let series_id = metric["series_id"].as_str().unwrap().to_string();
            series_data.entry(series_id).or_default().push(metric);
        }

        // Verify each series is ordered by timestamp
        for (_, metrics) in series_data {
            let mut prev_ts = i64::MIN;
            for metric in metrics {
                let ts = metric["timestamp"].as_i64().unwrap();
                assert!(ts > prev_ts);
                prev_ts = ts;
            }
        }
    }

    #[test]
    fn test_metrics_value_bounds() {
        let data = generate_metrics(1000);

        for metric in &data {
            let series_id = metric["series_id"].as_str().unwrap();
            let value = metric["value"].as_f64().unwrap() as f32;
            let params = get_metric_params(series_id);

            // Values should stay within bounds
            assert!(value >= params.min - 1.0);  // Allow small margin for float precision
            assert!(value <= params.max + 1.0);
        }
    }

    #[test]
    fn test_metrics_cardinality() {
        let data = generate_metrics(1000);

        // Count unique series and hosts
        let series: std::collections::HashSet<_> = data.iter()
            .map(|m| m["series_id"].as_str().unwrap())
            .collect();

        let hosts: std::collections::HashSet<_> = data.iter()
            .map(|m| m["host"].as_str().unwrap())
            .collect();

        // Should have expected cardinality
        assert_eq!(series.len(), SERIES.len());
        assert_eq!(hosts.len(), 3);  // 3 regions
    }

    #[test]
    fn test_metrics_delta_optimal() {
        let data = generate_metrics(100);

        // Group by series to test each separately
        let mut series_data: std::collections::HashMap<String, Vec<f32>> = std::collections::HashMap::new();
        for metric in &data {
            let series_id = metric["series_id"].as_str().unwrap().to_string();
            let value = metric["value"].as_f64().unwrap() as f32;
            series_data.entry(series_id).or_default().push(value);
        }

        // Check that deltas are small (good for delta encoding)
        for (_, values) in series_data {
            let mut deltas = Vec::new();
            for i in 1..values.len() {
                deltas.push((values[i] - values[i-1]).abs());
            }

            // Most deltas should be small (< 10% of typical value)
            let small_deltas = deltas.iter().filter(|d| **d < 100.0).count();
            assert!(small_deltas as f64 / deltas.len() as f64 > 0.5);
        }
    }

    #[test]
    fn test_metrics_dictionary_optimal_series() {
        let data = generate_metrics(10000);

        // Count unique series (should be small)
        let series: std::collections::HashSet<_> = data.iter()
            .map(|m| m["series_id"].as_str().unwrap())
            .collect();

        // 54 unique series is good for dictionary encoding
        assert_eq!(series.len(), 54);

        // Count occurrences - should be balanced across series
        let mut counts = std::collections::HashMap::new();
        for metric in &data {
            *counts.entry(metric["series_id"].as_str().unwrap().to_string())
                .or_insert(0) += 1;
        }

        let min_count = *counts.values().min().unwrap();
        let max_count = *counts.values().max().unwrap();

        // Should be roughly equal distribution
        assert_eq!(min_count, max_count);
    }

    #[test]
    fn test_metrics_reproducible() {
        let data1 = generate_metrics_with_seed(100, 42);
        let data2 = generate_metrics_with_seed(100, 42);

        for (d1, d2) in data1.iter().zip(data2.iter()) {
            assert_eq!(d1["series_id"], d2["series_id"]);
            assert_eq!(d1["timestamp"], d2["timestamp"]);
            assert_eq!(d1["value"], d2["value"]);
        }
    }
}
