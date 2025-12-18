//! Real-world transaction dataset for codec testing
//!
//! Simulates customer transaction data with:
//! - Sequential transaction IDs (delta optimal)
//! - Monotonic timestamps (delta optimal)
//! - Repeated merchants (dictionary optimal)
//! - Repeated user IDs (dictionary optimal)
//! - Random amounts (raw)
//! - Categorical data (dictionary optimal)
//! - Boolean flags (RLE optimal for sequences)

use serde_json::{json, Value};
use rand::Rng;

/// Top merchants (real-world distribution: 80% of volume from 20% of merchants)
const TOP_MERCHANTS: &[&str] = &[
    "Amazon", "Walmart", "Target", "Costco", "Best Buy",
    "Starbucks", "McDonald's", "Chipotle", "Subway", "Whole Foods",
    "Apple Store", "Best Buy Online", "Nordstrom", "Sephora", "Nike",
    "H&M", "Zara", "Forever 21", "Urban Outfitters", "ASOS",
    "Uber Eats", "DoorDash", "Airbnb", "Booking.com", "Hotels.com",
    "Netflix", "Spotify", "Hulu", "Disney+", "Amazon Prime",
    "Shell Gas", "Chevron Gas", "Exxon Gas", "BP Gas", "Speedway",
    "CVS Pharmacy", "Walgreens", "Rite Aid", "Target Pharmacy", "Walmart Pharmacy",
    "Chase Bank", "Bank of America", "Wells Fargo", "Citi", "US Bank",
    "Trader Joe's", "Safeway", "Kroger", "Publix", "Albertsons",
    "United Airlines", "Delta Airlines", "American Airlines", "Southwest", "JetBlue",
    "Marriott", "Hilton", "Hyatt", "Sheraton", "Westin",
    "Pizza Hut", "Domino's", "Papa John's", "Little Caesars", "Blaze Pizza",
    "Gas Station 1", "Gas Station 2", "Gas Station 3", "Gas Station 4", "Gas Station 5",
];

/// Product categories
const CATEGORIES: &[&str] = &[
    "Groceries", "Restaurants", "Retail", "Utilities", "Travel",
    "Entertainment", "Healthcare", "Fuel", "Technology", "Clothing",
    "Home & Garden", "Sports", "Books", "Electronics", "Dining",
    "Lodging", "Transportation", "Subscriptions", "Insurance", "Finance",
];

/// Generate realistic transaction dataset
///
/// # Arguments
/// * `count` - Number of transactions to generate
/// * `seed` - Optional random seed for reproducibility
///
/// # Returns
/// Vec of transaction JSON values with realistic patterns
pub fn generate_transactions(count: usize) -> Vec<Value> {
    let mut rng = rand::thread_rng();
    let num_unique_users = (count as f64).sqrt() as usize;  // sqrt(count) unique users

    // Start from a realistic timestamp (Dec 17, 2025 00:00:00 UTC)
    let base_timestamp = 1766534400i64;

    (0..count)
        .map(|i| {
            let user_id = rng.gen_range(1..=num_unique_users as i32);
            let merchant_idx = if rng.gen_bool(0.8) {
                // 80% from top merchants (power law distribution)
                rng.gen_range(0..20usize)
            } else {
                // 20% from long tail
                rng.gen_range(20..TOP_MERCHANTS.len())
            };
            let merchant = TOP_MERCHANTS[merchant_idx];

            // Determine success rate based on merchant (most succeed, some fail)
            let success = rng.gen_bool(0.98);

            // Amounts vary by merchant
            let amount = match merchant_idx {
                // Groceries typically smaller amounts
                i if i < 5 => rng.gen_range(20.0..150.0),
                // Restaurants
                i if i < 10 => rng.gen_range(15.0..100.0),
                // Retail larger
                i if i < 20 => rng.gen_range(50.0..500.0),
                // Gas station
                i if i > 50 && i < 55 => rng.gen_range(30.0..80.0),
                // Default
                _ => rng.gen_range(10.0..300.0),
            };

            json!({
                "transaction_id": i as i64 + 1000000000,
                "timestamp": base_timestamp + (i as i64 * 60),  // One transaction per minute
                "user_id": user_id,
                "merchant": merchant,
                "amount": (amount as f64 * 100.0).round() / 100.0,  // 2 decimal places
                "category": CATEGORIES[rng.gen_range(0..CATEGORIES.len())],
                "success": success,
                "device": if rng.gen_bool(0.6) { "mobile" } else { "web" },
                "country": if rng.gen_bool(0.9) { "US" } else { "other" },
            })
        })
        .collect()
}

/// Generate transactions with specific characteristics
pub fn generate_transactions_with_seed(count: usize, seed: u64) -> Vec<Value> {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let num_unique_users = (count as f64).sqrt() as usize;
    let base_timestamp = 1766534400i64;

    (0..count)
        .map(|i| {
            let user_id = rng.gen_range(1..=num_unique_users as i32);
            let merchant_idx = if rng.gen_bool(0.8) {
                rng.gen_range(0..20usize)
            } else {
                rng.gen_range(20..TOP_MERCHANTS.len())
            };
            let merchant = TOP_MERCHANTS[merchant_idx];
            let success = rng.gen_bool(0.98);
            let amount = rng.gen_range(10.0..500.0);

            json!({
                "transaction_id": i as i64 + 1000000000,
                "timestamp": base_timestamp + (i as i64 * 60),
                "user_id": user_id,
                "merchant": merchant,
                "amount": (amount as f64 * 100.0).round() / 100.0,
                "category": CATEGORIES[rng.gen_range(0..CATEGORIES.len())],
                "success": success,
                "device": if rng.gen_bool(0.6) { "mobile" } else { "web" },
                "country": if rng.gen_bool(0.9) { "US" } else { "other" },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_transactions_small() {
        let data = generate_transactions(100);
        assert_eq!(data.len(), 100);

        // Verify structure
        for (i, tx) in data.iter().enumerate() {
            assert!(tx["transaction_id"].is_i64());
            assert!(tx["timestamp"].is_i64());
            assert!(tx["user_id"].is_i64());
            assert!(tx["merchant"].is_string());
            assert!(tx["amount"].is_f64());
            assert!(tx["category"].is_string());
            assert!(tx["success"].is_boolean());

            // Verify sequential properties
            if i > 0 {
                let prev_ts = data[i-1]["timestamp"].as_i64().unwrap();
                let curr_ts = tx["timestamp"].as_i64().unwrap();
                assert_eq!(curr_ts - prev_ts, 60);  // One minute apart
            }
        }
    }

    #[test]
    fn test_transactions_distribution() {
        let data = generate_transactions(10000);

        // Count merchant frequency
        let mut merchants = std::collections::HashMap::new();
        let mut total_amount = 0.0;
        let mut success_count = 0;

        for tx in &data {
            *merchants.entry(tx["merchant"].as_str().unwrap().to_string())
                .or_insert(0) += 1;

            if let Some(amount) = tx["amount"].as_f64() {
                total_amount += amount;
            }

            if tx["success"].as_bool().unwrap() {
                success_count += 1;
            }
        }

        // Verify power law distribution: top 20 merchants should have 80% of transactions
        let mut top_merchants: Vec<_> = merchants.values().copied().collect();
        top_merchants.sort_by(|a, b| b.cmp(a));
        let top_20_count: usize = top_merchants.iter().take(20).sum();
        let percentage = (top_20_count as f64 / data.len() as f64) * 100.0;

        // Should be around 60-80% for top 20
        assert!(percentage > 50.0 && percentage < 90.0);

        // Verify success rate around 98%
        let success_rate = (success_count as f64 / data.len() as f64) * 100.0;
        assert!(success_rate > 95.0);
    }

    #[test]
    fn test_transactions_reproducible() {
        let data1 = generate_transactions_with_seed(1000, 42);
        let data2 = generate_transactions_with_seed(1000, 42);

        for (d1, d2) in data1.iter().zip(data2.iter()) {
            assert_eq!(d1.to_string(), d2.to_string());
        }
    }

    #[test]
    fn test_transactions_delta_optimal() {
        let data = generate_transactions(100);

        // Timestamps should be monotonically increasing with small deltas
        let mut prev_ts = 0i64;
        for tx in &data {
            let ts = tx["timestamp"].as_i64().unwrap();
            assert!(ts > prev_ts);
            assert_eq!(ts - prev_ts, 60);  // Constant delta = 60 seconds
            prev_ts = ts;
        }
    }

    #[test]
    fn test_transactions_dictionary_optimal() {
        let data = generate_transactions(10000);

        // Count unique merchants and users
        let merchants: std::collections::HashSet<_> = data.iter()
            .map(|tx| tx["merchant"].as_str().unwrap())
            .collect();

        let users: std::collections::HashSet<_> = data.iter()
            .map(|tx| tx["user_id"].as_i64().unwrap())
            .collect();

        // Should have moderate cardinality (good for dictionary)
        assert!(merchants.len() < 100);  // Around 70 merchants
        assert!(users.len() > 50);  // sqrt(10000) = 100 unique users approx
        assert!(users.len() < 500);
    }
}
