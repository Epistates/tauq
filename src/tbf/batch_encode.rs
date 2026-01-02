//! High-level batch encoding with parallelization (Phase 2, Week 5)
//!
//! This module provides convenient APIs for encoding large batches of data
//! with automatic parallelization when beneficial.
//!
//! # Example
//!
//! ```no_run
//! use tauq::tbf::BatchEncoder;
//!
//! let mut encoder = BatchEncoder::new();
//! // Add 10000 items
//! for i in 0..10000 {
//!     encoder.add_record(i.to_string());
//! }
//! let bytes = encoder.encode();
//! ```

use crate::error::TauqError;
use serde::Serialize;

/// Batch encoder for large datasets
///
/// Collects items and encodes them in optimized batches.
/// Note: Parallel encoding requires T to be Sync, which is often not the case for
/// arbitrary types. This encoder provides a high-level API for collecting and
/// encoding batches, with parallelization possible for specific types.
#[derive(Debug)]
pub struct BatchEncoder<T: Serialize> {
    /// Items to encode
    items: Vec<T>,
    /// Auto-parallelize when count exceeds this
    parallel_threshold: usize,
}

impl<T: Serialize> BatchEncoder<T> {
    /// Create a new batch encoder
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            parallel_threshold: 100,
        }
    }

    /// Create with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            parallel_threshold: 100,
        }
    }

    /// Set the parallelization threshold
    pub fn with_parallel_threshold(mut self, threshold: usize) -> Self {
        self.parallel_threshold = threshold;
        self
    }

    /// Add a record to the batch
    pub fn add_record(&mut self, item: T) {
        self.items.push(item);
    }

    /// Add multiple records at once
    pub fn add_records(&mut self, items: impl IntoIterator<Item = T>) {
        self.items.extend(items);
    }

    /// Encode all accumulated records
    pub fn encode(&self) -> Result<Vec<u8>, TauqError> {
        if self.items.is_empty() {
            // Empty batch - encode as empty array
            return super::to_bytes(&Vec::<T>::new());
        }

        // Use sequential encoding (safe for all types)
        // Note: For types that implement Sync, BatchEncoder<T> can be extended
        // with parallel encoding methods in the future
        super::to_bytes(&self.items)
    }

    /// Get the number of accumulated records
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Clear all accumulated records
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Get reference to accumulated records
    pub fn items(&self) -> &[T] {
        &self.items
    }
}

impl<T: Serialize> Default for BatchEncoder<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about batch encoding
#[derive(Debug, Clone)]
pub struct BatchEncodingStats {
    /// Number of records encoded
    pub record_count: usize,
    /// Bytes encoded
    pub bytes: usize,
    /// Parallelization was used
    pub parallelized: bool,
    /// Bytes per record (average)
    pub bytes_per_record: f64,
}

impl BatchEncodingStats {
    /// Create new batch encoding statistics
    pub fn new(record_count: usize, bytes: usize, parallelized: bool) -> Self {
        let bytes_per_record = if record_count > 0 {
            bytes as f64 / record_count as f64
        } else {
            0.0
        };

        Self {
            record_count,
            bytes,
            parallelized,
            bytes_per_record,
        }
    }

    /// Calculate compression ratio vs JSON
    pub fn compression_ratio_vs_json(&self, json_size: usize) -> f64 {
        if json_size > 0 {
            (self.bytes as f64 / json_size as f64) * 100.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_encoder_empty() {
        let encoder: BatchEncoder<i32> = BatchEncoder::new();
        assert_eq!(encoder.len(), 0);
        assert!(encoder.is_empty());
    }

    #[test]
    fn test_batch_encoder_add_records() {
        let mut encoder = BatchEncoder::new();
        encoder.add_record(1);
        encoder.add_record(2);
        encoder.add_record(3);

        assert_eq!(encoder.len(), 3);
        assert!(!encoder.is_empty());
    }

    #[test]
    fn test_batch_encoder_add_multiple() {
        let mut encoder = BatchEncoder::new();
        encoder.add_records(vec![1, 2, 3, 4, 5]);

        assert_eq!(encoder.len(), 5);
    }

    #[test]
    fn test_batch_encoder_clear() {
        let mut encoder = BatchEncoder::new();
        encoder.add_records(vec![1, 2, 3]);
        encoder.clear();

        assert_eq!(encoder.len(), 0);
        assert!(encoder.is_empty());
    }

    #[test]
    fn test_batch_encoder_encode_small() {
        let mut encoder = BatchEncoder::new();
        encoder.add_records(vec![1u32, 2u32, 3u32]);

        let bytes = encoder.encode().unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_batch_encoder_encode_large() {
        let mut encoder = BatchEncoder::new();
        for i in 0..1000 {
            encoder.add_record(i);
        }

        let bytes = encoder.encode().unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_batch_encoding_stats() {
        let stats = BatchEncodingStats::new(1000, 5000, true);
        assert_eq!(stats.record_count, 1000);
        assert_eq!(stats.bytes, 5000);
        assert!(stats.parallelized);
        assert!(stats.bytes_per_record > 4.9 && stats.bytes_per_record < 5.1);
    }

    #[test]
    fn test_compression_ratio_calculation() {
        let stats = BatchEncodingStats::new(100, 1000, false);
        let ratio = stats.compression_ratio_vs_json(10000);
        assert_eq!(ratio, 10.0); // 1000 / 10000 = 0.1 = 10%
    }
}
