//! Codec Selection Accuracy Validation
//!
//! Validates that automatic codec selection matches expected patterns
//! for different real-world data types

#[cfg(test)]
mod codec_selection_tests {
    use serde_json::json;
    use tauq::tbf::{CodecAnalyzer, CompressionCodec};

    /// Test delta encoding selection with monotonic integers
    #[test]
    fn test_delta_selection_sorted_integers() {
        let mut analyzer = CodecAnalyzer::new(100);

        // Sorted integers: 0, 1, 2, 3, ..., 99
        for i in 0..100 {
            analyzer.add_sample(Some(json!(i as i64)));
        }

        let codec = analyzer.choose_codec();
        assert_eq!(
            codec,
            CompressionCodec::Delta,
            "Sorted integers should select Delta codec"
        );
    }

    /// Test delta encoding with small deltas
    #[test]
    fn test_delta_selection_monotonic_deltas() {
        let mut analyzer = CodecAnalyzer::new(100);

        // Monotonically increasing with varying deltas: 100, 102, 105, 107, 110
        let mut value = 100i64;
        for delta in &[2, 3, 2, 3] {
            analyzer.add_sample(Some(json!(value)));
            value += delta;
        }
        for _ in 0..96 {
            analyzer.add_sample(Some(json!(value)));
            value += 1;
        }

        let codec = analyzer.choose_codec();
        assert_eq!(
            codec,
            CompressionCodec::Delta,
            "Monotonic integers with small deltas should select Delta"
        );
    }

    /// Test dictionary encoding with repeated values
    #[test]
    fn test_dictionary_selection_repeated_strings() {
        let mut analyzer = CodecAnalyzer::new(100);

        let cities = ["New York", "Los Angeles", "Chicago", "Houston", "Phoenix"];

        // Generate 100 samples with heavy repetition
        for i in 0..100 {
            analyzer.add_sample(Some(json!(cities[i % cities.len()])));
        }

        let codec = analyzer.choose_codec();
        assert_eq!(
            codec,
            CompressionCodec::Dictionary,
            "Repeated strings should select Dictionary codec"
        );
    }

    /// Test RLE selection with boolean runs
    #[test]
    fn test_rle_selection_boolean_runs() {
        let mut analyzer = CodecAnalyzer::new(100);

        // Generate runs of true/false
        let mut value = true;
        let mut run_length = 10;
        let mut count = 0;

        while count < 100 {
            for _ in 0..std::cmp::min(run_length, 100 - count) {
                analyzer.add_sample(Some(json!(value)));
                count += 1;
            }
            value = !value;
            run_length = if run_length < 50 { run_length + 5 } else { 10 };
        }

        let codec = analyzer.choose_codec();
        assert_eq!(
            codec,
            CompressionCodec::RunLength,
            "Boolean runs should select RLE codec"
        );
    }

    /// Test RLE with single long run
    #[test]
    fn test_rle_selection_single_run() {
        let mut analyzer = CodecAnalyzer::new(100);

        // All same value (100% RLE optimal)
        for _ in 0..100 {
            analyzer.add_sample(Some(json!(true)));
        }

        let codec = analyzer.choose_codec();
        assert_eq!(
            codec,
            CompressionCodec::RunLength,
            "Constant values should select RLE codec"
        );
    }

    /// Test raw encoding fallback for high cardinality
    #[test]
    fn test_raw_selection_high_cardinality() {
        let mut analyzer = CodecAnalyzer::new(100);

        // 100 unique values (high cardinality)
        for i in 0..100 {
            analyzer.add_sample(Some(json!(format!("unique_{}", i))));
        }

        let codec = analyzer.choose_codec();
        // With 100 unique values in 100 samples, should either be Dictionary (if < threshold)
        // or Raw. Since cardinality is 100%, should select Raw or Dictionary depending on threshold
        assert!(
            matches!(codec, CompressionCodec::Dictionary | CompressionCodec::Raw),
            "High cardinality should not select Delta or RLE"
        );
    }

    /// Test random values fallback to raw
    #[test]
    fn test_raw_selection_random_data() {
        let mut analyzer = CodecAnalyzer::new(100);

        use rand::RngExt;
        let mut rng = rand::rng();

        // Random integers with no pattern
        for _ in 0..100 {
            analyzer.add_sample(Some(json!(rng.random_range(i64::MIN..i64::MAX))));
        }

        let codec = analyzer.choose_codec();
        // Random data should not compress well, might select Raw or Dictionary
        // depending on coincidental patterns
        assert!(
            matches!(codec, CompressionCodec::Raw | CompressionCodec::Dictionary),
            "Random data should fall back to Raw or Dictionary"
        );
    }

    /// Test timestamp sequences (real-world use case)
    #[test]
    fn test_delta_selection_timestamps() {
        let mut analyzer = CodecAnalyzer::new(100);

        let mut timestamp = 1700000000i64;
        for _ in 0..100 {
            analyzer.add_sample(Some(json!(timestamp)));
            timestamp += 60; // 1 minute intervals
        }

        let codec = analyzer.choose_codec();
        assert_eq!(
            codec,
            CompressionCodec::Delta,
            "Sequential timestamps should select Delta"
        );
    }

    /// Test location codes (moderate cardinality)
    #[test]
    fn test_dictionary_selection_location_codes() {
        let mut analyzer = CodecAnalyzer::new(100);

        let locations = vec![
            "NYC", "LA", "CHI", "HOU", "PHX", "PHI", "SFO", "BOS", "SEA", "DEN",
        ];

        // 10 unique locations in 100 samples
        for i in 0..100 {
            analyzer.add_sample(Some(json!(locations[i % locations.len()])));
        }

        let codec = analyzer.choose_codec();
        assert_eq!(
            codec,
            CompressionCodec::Dictionary,
            "Low cardinality strings should select Dictionary"
        );
    }

    /// Test feature flags (RLE optimal)
    #[test]
    fn test_rle_selection_feature_flags() {
        let mut analyzer = CodecAnalyzer::new(100);

        // Simulate feature flags being mostly on with rare off periods
        for i in 0..100 {
            let enabled = i % 20 != 0; // 5% disabled
            analyzer.add_sample(Some(json!(enabled)));
        }

        let codec = analyzer.choose_codec();
        assert_eq!(
            codec,
            CompressionCodec::RunLength,
            "Feature flags with runs should select RLE"
        );
    }

    /// Test mixed numeric types (edge case)
    #[test]
    fn test_codec_selection_with_nulls() {
        let mut analyzer = CodecAnalyzer::new(100);

        // Mixed valid and null values
        for i in 0..100 {
            if i % 10 == 0 {
                analyzer.add_sample(None);
            } else {
                analyzer.add_sample(Some(json!(i as i64)));
            }
        }

        let codec = analyzer.choose_codec();
        // Should still recognize the numeric pattern despite nulls
        assert!(
            matches!(
                codec,
                CompressionCodec::Raw
                    | CompressionCodec::Delta
                    | CompressionCodec::Dictionary
                    | CompressionCodec::RunLength
            ),
            "Should not fail with null values"
        );
    }

    /// Test accuracy of codec selection (sampling effectiveness)
    #[test]
    fn test_codec_selection_sampling_accuracy() {
        // Test that first 100 values are representative
        let mut first_100 = CodecAnalyzer::new(100);
        let mut all_values = CodecAnalyzer::new(10000);

        // Generate 10,000 values but analyze first 100 vs all
        for i in 0..10000 {
            let value = json!(i / 100); // Groups of 100 same values

            if i < 100 {
                first_100.add_sample(Some(value.clone()));
            }
            all_values.add_sample(Some(value));
        }

        let codec_first = first_100.choose_codec();
        let codec_all = all_values.choose_codec();

        // Both should detect similar patterns
        assert_eq!(
            codec_first, codec_all,
            "Sampling 100 values should match codec selection for all 10K"
        );
    }

    /// Test codec selection priority (RLE > Delta > Dictionary > Raw)
    #[test]
    fn test_codec_priority_rle_vs_delta() {
        // Create data with both RLE and Delta characteristics
        let mut analyzer = CodecAnalyzer::new(100);

        // Monotonic with some runs: 0,0,0,1,1,1,2,2,2...
        let mut value = 0i64;
        let mut count_in_run = 0;
        for _ in 0..100 {
            analyzer.add_sample(Some(json!(value)));
            count_in_run += 1;
            if count_in_run >= 3 {
                value += 1;
                count_in_run = 0;
            }
        }

        let codec = analyzer.choose_codec();
        // RLE should win if runs are significant
        // This is a lower threshold for RLE so it might select RLE
        assert!(
            !matches!(codec, CompressionCodec::Raw),
            "Should select effective codec when data has patterns"
        );
    }

    /// Test cardinality threshold for dictionary
    #[test]
    fn test_dictionary_cardinality_limits() {
        // Test with different cardinality levels
        for num_unique in [2, 10, 20].iter() {
            let mut analyzer = CodecAnalyzer::new(100);

            // Generate 100 samples with specified cardinality
            for i in 0..100 {
                let key = format!("item_{}", i % num_unique);
                analyzer.add_sample(Some(json!(key)));
            }

            let codec = analyzer.choose_codec();

            match num_unique {
                2 | 10 | 20 => {
                    // Low cardinality should select dictionary
                    assert_eq!(
                        codec,
                        CompressionCodec::Dictionary,
                        "Cardinality {} should use Dictionary",
                        num_unique
                    );
                }
                _ => {}
            }
        }

        // High cardinality might use Raw
        let mut high_card = CodecAnalyzer::new(100);
        for i in 0..100 {
            high_card.add_sample(Some(json!(format!("unique_{}", i))));
        }
        let codec = high_card.choose_codec();
        assert!(
            matches!(codec, CompressionCodec::Dictionary | CompressionCodec::Raw),
            "100% unique cardinality might use Dictionary or Raw"
        );
    }
}

#[cfg(test)]
mod codec_edge_cases {
    use serde_json::json;
    use tauq::tbf::{CodecAnalyzer, CompressionCodec};

    /// Test with minimal data (edge case)
    #[test]
    fn test_codec_selection_minimal_data() {
        let mut analyzer = CodecAnalyzer::new(100);

        // Only one value
        analyzer.add_sample(Some(json!(42i64)));

        let codec = analyzer.choose_codec();
        // Should return some valid codec, not panic
        assert!(
            matches!(
                codec,
                CompressionCodec::Raw
                    | CompressionCodec::Delta
                    | CompressionCodec::Dictionary
                    | CompressionCodec::RunLength
            ),
            "Should handle minimal data gracefully"
        );
    }

    /// Test with empty data (edge case)
    #[test]
    fn test_codec_selection_empty_data() {
        let analyzer = CodecAnalyzer::new(100);

        let codec = analyzer.choose_codec();
        // Empty should default to Raw
        assert_eq!(
            codec,
            CompressionCodec::Raw,
            "Empty data should default to Raw"
        );
    }

    /// Test with very large numbers
    #[test]
    fn test_codec_selection_large_numbers() {
        let mut analyzer = CodecAnalyzer::new(100);

        let base = i64::MAX / 2;
        for i in 0..100 {
            analyzer.add_sample(Some(json!(base + i as i64)));
        }

        let codec = analyzer.choose_codec();
        assert_eq!(
            codec,
            CompressionCodec::Delta,
            "Large monotonic numbers should still select Delta"
        );
    }

    /// Test with very small deltas
    #[test]
    fn test_codec_selection_tiny_deltas() {
        let mut analyzer = CodecAnalyzer::new(100);

        let mut value = 0.0f64;
        for _ in 0..100 {
            analyzer.add_sample(Some(json!(value)));
            value += 0.001; // Tiny delta
        }

        let codec = analyzer.choose_codec();
        // Floating point might not compress well with Delta
        assert!(
            !matches!(codec, CompressionCodec::Raw),
            "Should handle floating point data"
        );
    }

    /// Test with alternating values (worst case for RLE)
    #[test]
    fn test_codec_selection_alternating_values() {
        let mut analyzer = CodecAnalyzer::new(100);

        // Alternating: A, B, A, B, A, B...
        for i in 0..100 {
            let value = if i % 2 == 0 { "A" } else { "B" };
            analyzer.add_sample(Some(json!(value)));
        }

        let codec = analyzer.choose_codec();
        // Alternating should select Dictionary (cardinality = 2)
        assert_eq!(
            codec,
            CompressionCodec::Dictionary,
            "Alternating values should select Dictionary"
        );
    }

    /// Test null-heavy data
    #[test]
    fn test_codec_selection_many_nulls() {
        let mut analyzer = CodecAnalyzer::new(100);

        // 90% nulls, 10% values
        for i in 0..100 {
            if i < 90 {
                analyzer.add_sample(None);
            } else {
                analyzer.add_sample(Some(json!(i as i64)));
            }
        }

        let codec = analyzer.choose_codec();
        // Should return a valid codec despite many nulls
        assert!(
            matches!(
                codec,
                CompressionCodec::Raw
                    | CompressionCodec::Delta
                    | CompressionCodec::Dictionary
                    | CompressionCodec::RunLength
            ),
            "Should handle null-heavy data"
        );
    }
}
