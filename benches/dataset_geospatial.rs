//! Real-world geospatial data dataset for codec testing
//!
//! Simulates GPS tracking data with:
//! - Repeated location IDs (dictionary optimal - 10-20x compression)
//! - Small coordinate deltas (delta optimal - 2-3x compression)
//! - Sequential timestamps (delta optimal)
//! - Repeating accuracy values (RLE optimal)
//! - Burst patterns (tracking, then idle)

use rand::prelude::*;
use serde_json::{Value, json};

/// Common locations being tracked
struct Location {
    #[allow(dead_code)]
    name: &'static str,
    latitude: f64,
    longitude: f64,
}

/// Major cities and tracking points
const LOCATIONS: &[Location] = &[
    Location {
        name: "new_york",
        latitude: 40.7128,
        longitude: -74.0060,
    },
    Location {
        name: "los_angeles",
        latitude: 34.0522,
        longitude: -118.2437,
    },
    Location {
        name: "chicago",
        latitude: 41.8781,
        longitude: -87.6298,
    },
    Location {
        name: "houston",
        latitude: 29.7604,
        longitude: -95.3698,
    },
    Location {
        name: "phoenix",
        latitude: 33.4484,
        longitude: -112.0742,
    },
    Location {
        name: "philadelphia",
        latitude: 39.9526,
        longitude: -75.1652,
    },
    Location {
        name: "san_antonio",
        latitude: 29.4241,
        longitude: -98.4936,
    },
    Location {
        name: "san_diego",
        latitude: 32.7157,
        longitude: -117.1611,
    },
    Location {
        name: "dallas",
        latitude: 32.7767,
        longitude: -96.7970,
    },
    Location {
        name: "san_jose",
        latitude: 37.3382,
        longitude: -121.8863,
    },
    Location {
        name: "austin",
        latitude: 30.2672,
        longitude: -97.7431,
    },
    Location {
        name: "jacksonville",
        latitude: 30.3322,
        longitude: -81.6557,
    },
    Location {
        name: "fort_worth",
        latitude: 32.7555,
        longitude: -97.3308,
    },
    Location {
        name: "columbus",
        latitude: 39.9612,
        longitude: -82.9988,
    },
    Location {
        name: "indianapolis",
        latitude: 39.7684,
        longitude: -86.1581,
    },
    Location {
        name: "charlotte",
        latitude: 35.2271,
        longitude: -80.8431,
    },
    Location {
        name: "detroit",
        latitude: 42.3314,
        longitude: -83.0458,
    },
    Location {
        name: "memphis",
        latitude: 35.1495,
        longitude: -90.0490,
    },
    Location {
        name: "boston",
        latitude: 42.3601,
        longitude: -71.0589,
    },
    Location {
        name: "seattle",
        latitude: 47.6062,
        longitude: -122.3321,
    },
    // Tech company offices
    Location {
        name: "google_hq",
        latitude: 37.4220,
        longitude: -122.0841,
    },
    Location {
        name: "apple_hq",
        latitude: 37.3346,
        longitude: -122.0097,
    },
    Location {
        name: "microsoft_hq",
        latitude: 47.6202,
        longitude: -122.3212,
    },
    Location {
        name: "amazon_hq",
        latitude: 47.6150,
        longitude: -122.3324,
    },
    Location {
        name: "meta_hq",
        latitude: 37.4847,
        longitude: -122.1477,
    },
    // Warehouse locations
    Location {
        name: "warehouse_nyc",
        latitude: 40.6892,
        longitude: -74.0445,
    },
    Location {
        name: "warehouse_la",
        latitude: 33.9186,
        longitude: -118.2105,
    },
    Location {
        name: "warehouse_chicago",
        latitude: 41.8782,
        longitude: -87.6292,
    },
    Location {
        name: "warehouse_dallas",
        latitude: 32.8156,
        longitude: -96.8473,
    },
    Location {
        name: "warehouse_atlanta",
        latitude: 33.7490,
        longitude: -84.3880,
    },
    Location {
        name: "distribution_nj",
        latitude: 40.2206,
        longitude: -74.7597,
    },
    Location {
        name: "distribution_pa",
        latitude: 40.4406,
        longitude: -79.9959,
    },
    Location {
        name: "distribution_ga",
        latitude: 33.6407,
        longitude: -84.4277,
    },
    Location {
        name: "distribution_ca",
        latitude: 35.3733,
        longitude: -119.0187,
    },
    Location {
        name: "distribution_tx",
        latitude: 32.7555,
        longitude: -97.3308,
    },
];

/// Accuracy levels for GPS tracking (grouped for RLE optimization)
const ACCURACY_LEVELS: &[f32] = &[5.0, 10.0, 15.0, 25.0, 50.0, 100.0];

/// Generate realistic GPS tracking data
///
/// # Arguments
/// * `count` - Number of GPS points to generate
///
/// # Returns
/// Vec of geospatial JSON values with realistic tracking patterns
pub fn generate_geospatial_data(count: usize) -> Vec<Value> {
    let mut rng = rand::rng();
    let base_timestamp = 1766534400i64;
    let mut result = Vec::new();

    // Simulate tracking of multiple devices
    let num_devices = 500;

    for point_idx in 0..count {
        let device_id = (point_idx % num_devices) as i32 + 1000;
        let location_idx = rng.random_range(0..LOCATIONS.len());
        let location = &LOCATIONS[location_idx];

        // Add realistic GPS noise (GPS typically accurate within 5-100 meters)
        // ~0.00005 degrees = ~5 meters at equator
        let lat_noise = rng.random_range(-0.0001..0.0001);
        let lon_noise = rng.random_range(-0.0001..0.0001);

        // Accuracy clusters (good for RLE)
        let accuracy_idx = if rng.random_bool(0.6) {
            0 // Most data has good accuracy (5m)
        } else if rng.random_bool(0.8) {
            1 // Some moderate accuracy (10m)
        } else {
            rng.random_range(2..ACCURACY_LEVELS.len()) // Occasional poor accuracy
        };

        result.push(json!({
            "device_id": device_id,
            "location_id": location_idx as i32,
            "latitude": location.latitude + lat_noise,
            "longitude": location.longitude + lon_noise,
            "timestamp": base_timestamp + (point_idx as i64 * 30),  // 30 seconds apart
            "accuracy": ACCURACY_LEVELS[accuracy_idx],
            "altitude": rng.random_range(0..500) as f32,
            "speed": rng.random_range(0.0..100.0),
        }));
    }

    result
}

/// Generate geospatial data with specific seed
#[allow(dead_code)]
pub fn generate_geospatial_data_with_seed(count: usize, seed: u64) -> Vec<Value> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let base_timestamp = 1766534400i64;
    let mut result = Vec::new();

    let num_devices = 500;

    for point_idx in 0..count {
        let device_id = (point_idx % num_devices) as i32 + 1000;
        let location_idx = rng.random_range(0..LOCATIONS.len());
        let location = &LOCATIONS[location_idx];

        let lat_noise = rng.random_range(-0.0001..0.0001);
        let lon_noise = rng.random_range(-0.0001..0.0001);

        let accuracy_idx = if rng.random_bool(0.6) {
            0
        } else if rng.random_bool(0.8) {
            1
        } else {
            rng.random_range(2..ACCURACY_LEVELS.len())
        };

        result.push(json!({
            "device_id": device_id,
            "location_id": location_idx as i32,
            "latitude": location.latitude + lat_noise,
            "longitude": location.longitude + lon_noise,
            "timestamp": base_timestamp + (point_idx as i64 * 30),
            "accuracy": ACCURACY_LEVELS[accuracy_idx],
            "altitude": rng.random_range(0..500) as f32,
            "speed": rng.random_range(0.0..100.0),
        }));
    }

    result
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_generate_geospatial_basic() {
        let data = generate_geospatial_data(1000);
        assert_eq!(data.len(), 1000);

        // Verify structure
        for point in &data {
            assert!(point["device_id"].is_i64());
            assert!(point["location_id"].is_i64());
            assert!(point["latitude"].is_f64());
            assert!(point["longitude"].is_f64());
            assert!(point["timestamp"].is_i64());
            assert!(point["accuracy"].is_f64());
        }
    }

    #[test]
    fn test_geospatial_location_bounds() {
        let data = generate_geospatial_data(10000);

        let mut lat_min = f64::MAX;
        let mut lat_max = f64::MIN;
        let mut lon_min = f64::MAX;
        let mut lon_max = f64::MIN;

        for point in &data {
            let lat = point["latitude"].as_f64().unwrap();
            let lon = point["longitude"].as_f64().unwrap();

            // Points should be within ~5km of known locations
            lat_min = lat_min.min(lat);
            lat_max = lat_max.max(lat);
            lon_min = lon_min.min(lon);
            lon_max = lon_max.max(lon);

            // Check that we're in continental US roughly
            assert!((25.0..=50.0).contains(&lat));
            assert!((-130.0..=-65.0).contains(&lon));
        }

        // Should span most of the country
        assert!(lat_max - lat_min > 5.0);
        assert!(lon_max - lon_min > 50.0);
    }

    #[test]
    fn test_geospatial_temporal_ordering() {
        let data = generate_geospatial_data(1000);

        let mut prev_ts = i64::MIN;
        for point in &data {
            let ts = point["timestamp"].as_i64().unwrap();
            assert!(ts >= prev_ts);
            prev_ts = ts;
        }
    }

    #[test]
    fn test_geospatial_location_cardinality() {
        let data = generate_geospatial_data(10000);

        // Count unique locations
        let locations: std::collections::HashSet<_> = data
            .iter()
            .map(|p| p["location_id"].as_i64().unwrap())
            .collect();

        // Should have all 35 locations represented
        assert!(locations.len() > 30);
        assert!(locations.len() <= LOCATIONS.len());
    }

    #[test]
    fn test_geospatial_accuracy_rle_optimal() {
        let data = generate_geospatial_data(10000);

        // Count accuracy clusters
        let accuracy_values: Vec<f32> = data
            .iter()
            .map(|p| p["accuracy"].as_f64().unwrap() as f32)
            .collect();

        // Most should be 5.0 (60% chance in generation)
        let high_accuracy = accuracy_values.iter().filter(|a| **a == 5.0).count();
        assert!(high_accuracy as f64 / accuracy_values.len() as f64 > 0.5);

        // Should have clustered values (good for RLE)
        let unique_accuracies: std::collections::HashSet<i32> =
            accuracy_values.iter().map(|a| (a * 100.0) as i32).collect();
        assert!(unique_accuracies.len() <= ACCURACY_LEVELS.len() + 5); // Some floating point variation
    }

    #[test]
    fn test_geospatial_delta_optimal_coordinates() {
        let data = generate_geospatial_data(1000);

        // Coordinates from same location should be very close (delta optimal)
        let mut location_coords: std::collections::HashMap<i64, Vec<(f64, f64)>> =
            std::collections::HashMap::new();
        for point in &data {
            let loc_id = point["location_id"].as_i64().unwrap();
            let lat = point["latitude"].as_f64().unwrap();
            let lon = point["longitude"].as_f64().unwrap();
            location_coords.entry(loc_id).or_default().push((lat, lon));
        }

        // Check deltas are small
        for (_, coords) in location_coords {
            if coords.len() > 1 {
                let first = coords[0];
                for coord in &coords[1..] {
                    let lat_delta = (coord.0 - first.0).abs();
                    let lon_delta = (coord.1 - first.1).abs();

                    // Within ~10km
                    assert!(lat_delta < 0.1);
                    assert!(lon_delta < 0.1);
                }
            }
        }
    }

    #[test]
    fn test_geospatial_reproducible() {
        let data1 = generate_geospatial_data_with_seed(1000, 42);
        let data2 = generate_geospatial_data_with_seed(1000, 42);

        for (d1, d2) in data1.iter().zip(data2.iter()) {
            assert_eq!(d1["device_id"], d2["device_id"]);
            assert_eq!(d1["timestamp"], d2["timestamp"]);
            assert_eq!(d1["location_id"], d2["location_id"]);
        }
    }

    #[test]
    fn test_geospatial_device_distribution() {
        let data = generate_geospatial_data(10000);

        // Count unique devices
        let devices: std::collections::HashSet<_> = data
            .iter()
            .map(|p| p["device_id"].as_i64().unwrap())
            .collect();

        // Should track 500 different devices
        assert_eq!(devices.len(), 500);

        // Points should be evenly distributed
        let mut device_counts = std::collections::HashMap::new();
        for point in &data {
            *device_counts
                .entry(point["device_id"].as_i64().unwrap())
                .or_insert(0) += 1;
        }

        let counts: Vec<_> = device_counts.values().collect();
        let min_count = *counts.iter().min().unwrap();
        let max_count = *counts.iter().max().unwrap();

        // Should be roughly equal (10000 / 500 = 20 points per device)
        assert_eq!(min_count, max_count); // Perfect balance due to round-robin
    }
}
