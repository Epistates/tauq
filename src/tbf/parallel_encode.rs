//! Parallel encoding for large datasets (Phase 2, Week 5)
//!
//! This module provides parallel encoding capabilities using rayon for:
//! - Multi-threaded string dictionary construction
//! - Parallel columnar encoding
//! - Thread-safe encoding context
//!
//! Use when encoding large batches (1000+ items) where parallelization
//! overhead is justified by speedup.

use crate::error::TauqError;

/// Builder for parallel batch encoding
///
/// Collects structured data and encodes it using multiple threads
/// for better throughput on large datasets.
#[derive(Debug, Clone)]
pub struct ParallelBatchEncoder {
    /// Batch size for parallel processing
    batch_size: usize,
    /// Minimum items to parallelize (overhead not worth it below this)
    min_parallel: usize,
}

impl ParallelBatchEncoder {
    /// Create a new parallel batch encoder
    pub fn new() -> Self {
        Self {
            batch_size: 1000,
            min_parallel: 100,
        }
    }

    /// Set the batch size for parallel processing
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Set minimum item count before parallelization kicks in
    pub fn with_min_parallel(mut self, count: usize) -> Self {
        self.min_parallel = count;
        self
    }

    /// Check if parallelization should be used for this item count
    pub fn should_parallelize(&self, count: usize) -> bool {
        #[cfg(feature = "performance")]
        {
            count >= self.min_parallel
        }
        #[cfg(not(feature = "performance"))]
        {
            false
        }
    }

    /// Calculate optimal thread count for parallel work
    ///
    /// Returns number of threads to use based on available CPUs
    /// and batch size
    pub fn optimal_threads(&self, total_items: usize) -> usize {
        #[cfg(feature = "performance")]
        {
            use rayon::current_num_threads;
            let num_threads = current_num_threads();
            let items_per_thread = self.batch_size / num_threads;

            if items_per_thread > 0 {
                (total_items / items_per_thread).min(num_threads)
            } else {
                1
            }
        }
        #[cfg(not(feature = "performance"))]
        {
            1
        }
    }
}

impl Default for ParallelBatchEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Parallel string dictionary intern
///
/// Each thread maintains its own dictionary, then all are merged
/// with index remapping. This avoids lock contention compared to
/// a single shared dictionary.
#[cfg(feature = "performance")]
pub struct ParallelStringDictionary {
    /// Per-thread dictionaries
    thread_dicts: Vec<std::collections::HashMap<String, u32>>,
    /// Global dictionary after merge
    global_dict: std::collections::HashMap<String, u32>,
    /// Mapping from old indices to new indices
    index_mapping: Vec<Vec<u32>>,
}

#[cfg(feature = "performance")]
impl ParallelStringDictionary {
    /// Create a new parallel string dictionary
    pub fn new(num_threads: usize) -> Self {
        Self {
            thread_dicts: vec![std::collections::HashMap::new(); num_threads],
            global_dict: std::collections::HashMap::new(),
            index_mapping: Vec::new(),
        }
    }

    /// Intern a string in a thread-local dictionary
    pub fn intern_in_thread(&mut self, thread_id: usize, s: &str) -> u32 {
        let dict = &mut self.thread_dicts[thread_id];
        let index = dict.len() as u32;
        *dict.entry(s.to_string()).or_insert(index)
    }

    /// Merge all thread-local dictionaries into a global one
    pub fn merge(&mut self) -> Result<(), TauqError> {
        let mut next_index = 0u32;

        // First pass: collect all unique strings
        for dict in &self.thread_dicts {
            for s in dict.keys() {
                if !self.global_dict.contains_key(s) {
                    self.global_dict.insert(s.clone(), next_index);
                    next_index += 1;
                }
            }
        }

        // Second pass: build index mapping for each thread
        for dict in &self.thread_dicts {
            let mut mapping = vec![0; dict.len()];
            for (s, old_idx) in dict {
                let new_idx = self.global_dict[s];
                mapping[*old_idx as usize] = new_idx;
            }
            self.index_mapping.push(mapping);
        }

        Ok(())
    }

    /// Get the merged global dictionary
    pub fn global_dict(&self) -> &std::collections::HashMap<String, u32> {
        &self.global_dict
    }

    /// Get index mapping for a specific thread
    pub fn get_mapping(&self, thread_id: usize) -> Option<&[u32]> {
        self.index_mapping.get(thread_id).map(|v| v.as_slice())
    }
}

/// Parallel encoding statistics
///
/// Tracks performance metrics during parallel encoding
#[derive(Debug, Clone)]
pub struct ParallelEncodingStats {
    /// Total items processed
    pub total_items: u64,
    /// Number of threads used
    pub threads_used: usize,
    /// Items per thread (average)
    pub items_per_thread: u64,
    /// Whether parallelization was used
    pub parallelized: bool,
}

impl ParallelEncodingStats {
    /// Create new encoding statistics
    pub fn new(total_items: usize, threads_used: usize, parallelized: bool) -> Self {
        let items_per_thread = if threads_used > 0 {
            (total_items as u64) / (threads_used as u64)
        } else {
            0
        };

        Self {
            total_items: total_items as u64,
            threads_used,
            items_per_thread,
            parallelized,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_batch_encoder_creation() {
        let encoder = ParallelBatchEncoder::new();
        assert_eq!(encoder.batch_size, 1000);
        assert_eq!(encoder.min_parallel, 100);
    }

    #[test]
    fn test_should_parallelize() {
        let encoder = ParallelBatchEncoder::new();

        // Below threshold
        assert!(!encoder.should_parallelize(50));

        // At threshold
        #[cfg(feature = "performance")]
        assert!(encoder.should_parallelize(100));

        // Above threshold
        #[cfg(feature = "performance")]
        assert!(encoder.should_parallelize(1000));
    }

    #[test]
    fn test_optimal_threads() {
        let encoder = ParallelBatchEncoder::new();
        let threads = encoder.optimal_threads(10000);

        #[cfg(feature = "performance")]
        {
            assert!(threads > 0);
            assert!(threads <= rayon::current_num_threads());
        }

        #[cfg(not(feature = "performance"))]
        {
            assert_eq!(threads, 1);
        }
    }

    #[test]
    #[cfg(feature = "performance")]
    fn test_parallel_string_dictionary_merge() {
        let mut dict = ParallelStringDictionary::new(2);

        // Thread 0 interns strings
        let idx1 = dict.intern_in_thread(0, "alice");
        let idx2 = dict.intern_in_thread(0, "bob");

        // Thread 1 interns same strings
        let idx3 = dict.intern_in_thread(1, "alice");
        let idx4 = dict.intern_in_thread(1, "charlie");

        // After merge, all unique strings should be in global dict
        dict.merge().unwrap();

        let global = dict.global_dict();
        assert_eq!(global.len(), 3); // alice, bob, charlie

        // Index mapping should be consistent
        let mapping0 = dict.get_mapping(0).unwrap();
        let mapping1 = dict.get_mapping(1).unwrap();

        // Thread 0 mapped indices should point to global dict
        assert_eq!(mapping0[idx1 as usize], global["alice"]);
        assert_eq!(mapping0[idx2 as usize], global["bob"]);

        // Thread 1 mapped indices should point to global dict
        assert_eq!(mapping1[idx3 as usize], global["alice"]);
        assert_eq!(mapping1[idx4 as usize], global["charlie"]);
    }

    #[test]
    fn test_parallel_encoding_stats() {
        let stats = ParallelEncodingStats::new(10000, 4, true);
        assert_eq!(stats.total_items, 10000);
        assert_eq!(stats.threads_used, 4);
        assert_eq!(stats.items_per_thread, 2500);
        assert!(stats.parallelized);
    }
}
