//! Adaptive compression codecs for columnar encoding (Phase 2, Week 6)
//!
//! This module provides automatic codec selection based on data patterns:
//! - Delta encoding for sorted integers
//! - Dictionary encoding for repeated values
//! - RLE (run-length encoding) for constant regions
//! - Raw encoding as fallback
//!
//! Codec selection is automatic via sampling the first 100 values.

use crate::error::TauqError;
use serde_json::Value;

/// Codec for encoding values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionCodec {
    /// No compression - raw values
    Raw = 0,
    /// Delta encoding for sorted integers
    Delta = 1,
    /// Dictionary encoding for repeated values
    Dictionary = 2,
    /// Run-length encoding for constant regions
    RunLength = 3,
}

impl CompressionCodec {
    /// Convert byte to CompressionCodec variant
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(CompressionCodec::Raw),
            1 => Some(CompressionCodec::Delta),
            2 => Some(CompressionCodec::Dictionary),
            3 => Some(CompressionCodec::RunLength),
            _ => None,
        }
    }
}

/// Analyzes data patterns to select best codec
#[derive(Debug, Clone)]
pub struct CodecAnalyzer {
    /// Sample of first N values
    samples: Vec<Option<Value>>,
    /// Maximum samples to analyze
    sample_size: usize,
}

impl CodecAnalyzer {
    /// Create a new codec analyzer
    pub fn new(sample_size: usize) -> Self {
        Self {
            samples: Vec::with_capacity(sample_size),
            sample_size,
        }
    }

    /// Add a sample value
    pub fn add_sample(&mut self, value: Option<Value>) {
        if self.samples.len() < self.sample_size {
            self.samples.push(value);
        }
    }

    /// Analyze samples and choose best codec
    pub fn choose_codec(&self) -> CompressionCodec {
        if self.samples.is_empty() {
            return CompressionCodec::Raw;
        }

        // Filter out nulls for analysis
        let non_null_samples: Vec<&Value> = self.samples
            .iter()
            .filter_map(|v| v.as_ref())
            .collect();

        if non_null_samples.is_empty() {
            return CompressionCodec::Raw;
        }

        // Check for RLE (constant values)
        if self.check_rle(&non_null_samples) {
            return CompressionCodec::RunLength;
        }

        // Check for delta encoding (sorted integers)
        if self.check_delta(&non_null_samples) {
            return CompressionCodec::Delta;
        }

        // Check for dictionary encoding (repeated values)
        if self.check_dictionary(&non_null_samples) {
            return CompressionCodec::Dictionary;
        }

        CompressionCodec::Raw
    }

    /// Check if values are constant (RLE candidates)
    fn check_rle(&self, values: &[&Value]) -> bool {
        if values.len() < 10 {
            return false; // Need sufficient data
        }

        // Count consecutive equal values and measure run lengths
        let mut total_run_length = 0;
        let mut current_run = 1;

        for i in 1..values.len() {
            if values[i] == values[i - 1] {
                current_run += 1;
            } else {
                if current_run >= 3 {
                    total_run_length += current_run;
                }
                current_run = 1;
            }
        }

        if current_run >= 3 {
            total_run_length += current_run;
        }

        // RLE is beneficial if > 30% of data is in runs of 3+ values
        total_run_length as f64 / values.len() as f64 > 0.3
    }

    /// Check if values are sorted or nearly sorted (delta candidates)
    fn check_delta(&self, values: &[&Value]) -> bool {
        if values.len() < 10 {
            return false;
        }

        // Only works for numbers
        let numeric_values: Vec<f64> = values
            .iter()
            .filter_map(|v| {
                if let Value::Number(n) = v {
                    n.as_f64()
                } else {
                    None
                }
            })
            .collect();

        if numeric_values.len() < 10 {
            return false;
        }

        // Check if sorted (ascending or descending)
        let is_ascending = numeric_values.windows(2).all(|w| w[0] <= w[1]);
        let is_descending = numeric_values.windows(2).all(|w| w[0] >= w[1]);

        is_ascending || is_descending
    }

    /// Check if values have high cardinality and repetition (dictionary candidates)
    fn check_dictionary(&self, values: &[&Value]) -> bool {
        if values.len() < 20 {
            return false;
        }

        // Count unique values and their frequencies
        let mut unique_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        // First pass: collect unique values to check cardinality
        for val in values {
            *unique_counts.entry(val.to_string()).or_insert(0) += 1;
        }

        let cardinality = unique_counts.len();
        let max_cardinality = (values.len() / 4).max(10); // 25% max cardinality

        // Dictionary is beneficial if:
        // 1. Low cardinality (< 25% unique values)
        // 2. Good repetition (some values appear multiple times)
        if cardinality > max_cardinality {
            return false;
        }

        // Check for repetition
        unique_counts.values().any(|&count| count > 1)
    }

    /// Get analysis of current samples (for testing/debugging)
    pub fn analyze(&self) -> CodecAnalysis {
        CodecAnalysis {
            sample_count: self.samples.len(),
            null_count: self.samples.iter().filter(|v| v.is_none()).count(),
            unique_values: self.count_unique_values(),
        }
    }

    fn count_unique_values(&self) -> usize {
        let mut unique = std::collections::HashSet::new();
        // First pass: collect unique values to check cardinality
        for val in self.samples.iter().flatten() {
            unique.insert(val.to_string());
        }
        unique.len()
    }
}

/// Analysis results
#[derive(Debug, Clone)]
pub struct CodecAnalysis {
    /// Total number of samples collected
    pub sample_count: usize,
    /// Number of null/None values seen
    pub null_count: usize,
    /// Number of unique values seen
    pub unique_values: usize,
}

impl Default for CodecAnalyzer {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Delta-encoded value buffer
#[derive(Debug, Clone)]
pub struct DeltaEncoder {
    /// Minimum value (base)
    base: i64,
    /// Deltas from base
    deltas: Vec<i64>,
}

impl DeltaEncoder {
    /// Create a new delta encoder
    pub fn new(base: i64) -> Self {
        Self {
            base,
            deltas: Vec::new(),
        }
    }

    /// Encode a value as delta
    pub fn encode(&mut self, value: i64) {
        let delta = value - self.base;
        self.deltas.push(delta);
        self.base = value;
    }

    /// Get encoded deltas
    pub fn deltas(&self) -> &[i64] {
        &self.deltas
    }

    /// Reconstruct original values
    pub fn decode(&self, initial: i64) -> Vec<i64> {
        let mut result = vec![initial];
        let mut current = initial;

        for &delta in &self.deltas {
            current += delta;
            result.push(current);
        }

        result
    }
}

/// Dictionary encoder for repeated values
#[derive(Debug, Clone)]
pub struct DictionaryEncoder {
    /// Unique values in order
    dictionary: Vec<Value>,
    /// Indices into dictionary
    indices: Vec<u32>,
}

impl DictionaryEncoder {
    /// Create a new dictionary encoder
    pub fn new() -> Self {
        Self {
            dictionary: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Encode a value using dictionary
    pub fn encode(&mut self, value: &Value) -> Result<(), TauqError> {
        // Find or insert value
        let idx = if let Some(pos) = self.dictionary.iter().position(|v| v == value) {
            pos as u32
        } else {
            let new_idx = self.dictionary.len() as u32;
            self.dictionary.push(value.clone());
            new_idx
        };

        self.indices.push(idx);
        Ok(())
    }

    /// Get dictionary
    pub fn dictionary(&self) -> &[Value] {
        &self.dictionary
    }

    /// Get indices
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Reconstruct original values
    pub fn decode(&self) -> Vec<Value> {
        self.indices
            .iter()
            .map(|&idx| self.dictionary[idx as usize].clone())
            .collect()
    }
}

impl Default for DictionaryEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Run-length encoded value
#[derive(Debug, Clone, PartialEq)]
pub struct RunLengthValue {
    /// The value
    pub value: Value,
    /// How many times it appears
    pub count: u32,
}

/// RLE encoder
#[derive(Debug, Clone)]
pub struct RLEEncoder {
    /// Run-length encoded values
    runs: Vec<RunLengthValue>,
}

impl RLEEncoder {
    /// Create a new RLE encoder
    pub fn new() -> Self {
        Self { runs: Vec::new() }
    }

    /// Encode a value with RLE
    pub fn encode(&mut self, value: &Value) {
        // Check if same as last run
        if let Some(last) = self.runs.last_mut()
            && last.value == *value
        {
            last.count += 1;
            return;
        }

        self.runs.push(RunLengthValue {
            value: value.clone(),
            count: 1,
        });
    }

    /// Get runs
    pub fn runs(&self) -> &[RunLengthValue] {
        &self.runs
    }

    /// Reconstruct original values
    pub fn decode(&self) -> Vec<Value> {
        let mut result = Vec::new();
        for run in &self.runs {
            for _ in 0..run.count {
                result.push(run.value.clone());
            }
        }
        result
    }
}

impl Default for RLEEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_codec_analyzer_rle_detection() {
        let mut analyzer = CodecAnalyzer::new(100);

        // Add constant values (good for RLE)
        for _ in 0..20 {
            analyzer.add_sample(Some(json!(true)));
        }

        let codec = analyzer.choose_codec();
        assert_eq!(codec, CompressionCodec::RunLength);
    }

    #[test]
    fn test_codec_analyzer_delta_detection() {
        let mut analyzer = CodecAnalyzer::new(100);

        // Add sorted values (good for delta)
        for i in 0..50 {
            analyzer.add_sample(Some(json!(i * 10)));
        }

        let codec = analyzer.choose_codec();
        assert_eq!(codec, CompressionCodec::Delta);
    }

    #[test]
    fn test_codec_analyzer_dictionary_detection() {
        let mut analyzer = CodecAnalyzer::new(100);

        // Add repeated values (good for dictionary)
        let values = vec!["alice", "bob", "alice", "carol", "bob", "alice"];
        for _ in 0..5 {
            for v in &values {
                analyzer.add_sample(Some(json!(v)));
            }
        }

        let codec = analyzer.choose_codec();
        assert_eq!(codec, CompressionCodec::Dictionary);
    }

    #[test]
    fn test_delta_encoder() {
        let mut encoder = DeltaEncoder::new(100);
        encoder.encode(102);
        encoder.encode(105);
        encoder.encode(107);

        assert_eq!(encoder.deltas(), &[2, 3, 2]);

        let reconstructed = encoder.decode(100);
        assert_eq!(reconstructed, vec![100, 102, 105, 107]);
    }

    #[test]
    fn test_dictionary_encoder() {
        let mut encoder = DictionaryEncoder::new();

        encoder.encode(&json!("alice")).unwrap();
        encoder.encode(&json!("bob")).unwrap();
        encoder.encode(&json!("alice")).unwrap();
        encoder.encode(&json!("carol")).unwrap();

        assert_eq!(encoder.dictionary().len(), 3);
        assert_eq!(encoder.indices(), &[0, 1, 0, 2]);

        let reconstructed = encoder.decode();
        assert_eq!(reconstructed, vec![
            json!("alice"),
            json!("bob"),
            json!("alice"),
            json!("carol"),
        ]);
    }

    #[test]
    fn test_rle_encoder() {
        let mut encoder = RLEEncoder::new();

        encoder.encode(&json!(true));
        encoder.encode(&json!(true));
        encoder.encode(&json!(true));
        encoder.encode(&json!(false));
        encoder.encode(&json!(false));
        encoder.encode(&json!(true));

        assert_eq!(encoder.runs().len(), 3);
        assert_eq!(encoder.runs()[0].count, 3);
        assert_eq!(encoder.runs()[1].count, 2);
        assert_eq!(encoder.runs()[2].count, 1);

        let reconstructed = encoder.decode();
        assert_eq!(reconstructed, vec![
            json!(true), json!(true), json!(true),
            json!(false), json!(false),
            json!(true),
        ]);
    }

    #[test]
    fn test_codec_analysis() {
        let mut analyzer = CodecAnalyzer::new(50);

        for i in 0..30 {
            analyzer.add_sample(Some(json!(i)));
        }

        let analysis = analyzer.analyze();
        assert_eq!(analysis.sample_count, 30);
        assert_eq!(analysis.null_count, 0);
        assert_eq!(analysis.unique_values, 30);
    }

    #[test]
    fn test_codec_analysis_with_nulls() {
        let mut analyzer = CodecAnalyzer::new(50);

        for i in 0..30 {
            if i % 5 == 0 {
                analyzer.add_sample(None);
            } else {
                analyzer.add_sample(Some(json!(i)));
            }
        }

        let analysis = analyzer.analyze();
        assert_eq!(analysis.null_count, 6); // 30 / 5 = 6 nulls
    }

    #[test]
    fn test_raw_codec_default() {
        let analyzer = CodecAnalyzer::new(100);
        let codec = analyzer.choose_codec();
        assert_eq!(codec, CompressionCodec::Raw); // Empty analyzer defaults to Raw
    }

    #[test]
    fn test_compression_codec_from_u8() {
        assert_eq!(CompressionCodec::from_u8(0), Some(CompressionCodec::Raw));
        assert_eq!(CompressionCodec::from_u8(1), Some(CompressionCodec::Delta));
        assert_eq!(CompressionCodec::from_u8(2), Some(CompressionCodec::Dictionary));
        assert_eq!(CompressionCodec::from_u8(3), Some(CompressionCodec::RunLength));
        assert_eq!(CompressionCodec::from_u8(99), None);
    }

    #[test]
    fn test_delta_encoder_empty() {
        let encoder = DeltaEncoder::new(100);
        assert!(encoder.deltas().is_empty());

        let reconstructed = encoder.decode(100);
        assert_eq!(reconstructed, vec![100]);
    }

    #[test]
    fn test_dictionary_encoder_single_value() {
        let mut encoder = DictionaryEncoder::new();

        encoder.encode(&json!(42)).unwrap();
        encoder.encode(&json!(42)).unwrap();
        encoder.encode(&json!(42)).unwrap();

        assert_eq!(encoder.dictionary().len(), 1);
        assert_eq!(encoder.indices(), &[0, 0, 0]);
    }
}
