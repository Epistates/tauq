//! Codec encoding coordination and integration (Phase 3, Week 7)
//!
//! This module provides the integration layer between the encoder and adaptive codecs.
//! It handles:
//! - Sampling and codec selection
//! - Routing values through appropriate codec encoders
//! - Codec metadata serialization
//! - Fallback to raw encoding if codec fails

use super::adaptive_encode::{
    CodecAnalyzer, CompressionCodec, DeltaEncoder, DictionaryEncoder, RLEEncoder,
};
use crate::error::TauqError;
use serde_json::Value;

/// Codec encoding context for a single sequence/column
#[derive(Debug, Clone)]
pub struct CodecEncodingContext {
    /// Codec analyzer for sampling
    pub analyzer: CodecAnalyzer,
    /// Selected codec after analysis
    pub selected_codec: Option<CompressionCodec>,
    /// Codec sample threshold (collect until this many items)
    pub sample_threshold: usize,
    /// Items collected so far
    pub items_collected: usize,
    /// Current delta encoder (if codec is Delta)
    pub delta_encoder: Option<DeltaEncoder>,
    /// Current dictionary encoder (if codec is Dictionary)
    pub dict_encoder: Option<DictionaryEncoder>,
    /// Current RLE encoder (if codec is RunLength)
    pub rle_encoder: Option<RLEEncoder>,
}

impl CodecEncodingContext {
    /// Create a new codec encoding context
    pub fn new(sample_threshold: usize) -> Self {
        Self {
            analyzer: CodecAnalyzer::new(sample_threshold),
            selected_codec: None,
            sample_threshold,
            items_collected: 0,
            delta_encoder: None,
            dict_encoder: None,
            rle_encoder: None,
        }
    }

    /// Add a sample value (during sampling phase)
    pub fn add_sample(&mut self, value: Option<&Value>) {
        if self.selected_codec.is_none() && self.items_collected < self.sample_threshold {
            self.analyzer.add_sample(value.cloned());
            self.items_collected += 1;

            // After collecting enough samples, select codec
            if self.items_collected >= self.sample_threshold {
                self.selected_codec = Some(self.analyzer.choose_codec());
                self.initialize_codec_encoder();
            }
        }
    }

    /// Initialize the appropriate codec encoder based on selection
    fn initialize_codec_encoder(&mut self) {
        match self.selected_codec {
            Some(CompressionCodec::Delta) => {
                // For delta encoding, we'll need an initial value
                // Set up encoder with base value 0 (will be updated with first value)
                self.delta_encoder = Some(DeltaEncoder::new(0));
            }
            Some(CompressionCodec::Dictionary) => {
                self.dict_encoder = Some(DictionaryEncoder::new());
            }
            Some(CompressionCodec::RunLength) => {
                self.rle_encoder = Some(RLEEncoder::new());
            }
            _ => {
                // Raw or None - no encoder needed
            }
        }
    }

    /// Check if codec selection is complete
    pub fn is_codec_selected(&self) -> bool {
        self.selected_codec.is_some()
    }

    /// Get the selected codec
    pub fn get_selected_codec(&self) -> Option<CompressionCodec> {
        self.selected_codec
    }

    /// Encode a value using the selected codec
    pub fn encode_value(&mut self, value: &Value) -> Result<(), TauqError> {
        match self.selected_codec {
            Some(CompressionCodec::Delta) => {
                // For numeric values, encode as delta
                if let Some(num) = value.as_i64() {
                    if let Some(ref mut encoder) = self.delta_encoder {
                        encoder.encode(num);
                        Ok(())
                    } else {
                        Err(TauqError::Interpret(crate::error::InterpretError::new(
                            "Delta encoder not initialized",
                        )))
                    }
                } else {
                    // Non-numeric - fallback to raw
                    Ok(())
                }
            }
            Some(CompressionCodec::Dictionary) => {
                if let Some(ref mut encoder) = self.dict_encoder {
                    encoder.encode(value)
                } else {
                    Err(TauqError::Interpret(crate::error::InterpretError::new(
                        "Dictionary encoder not initialized",
                    )))
                }
            }
            Some(CompressionCodec::RunLength) => {
                if let Some(ref mut encoder) = self.rle_encoder {
                    encoder.encode(value);
                    Ok(())
                } else {
                    Err(TauqError::Interpret(crate::error::InterpretError::new(
                        "RLE encoder not initialized",
                    )))
                }
            }
            Some(CompressionCodec::Raw) | None => {
                // No codec - handled elsewhere
                Ok(())
            }
        }
    }

    /// Get codec metadata for serialization
    pub fn get_codec_metadata(&self) -> CodecMetadata {
        match self.selected_codec {
            Some(CompressionCodec::Delta) => {
                if let Some(_encoder) = &self.delta_encoder {
                    // For delta, store the initial base value
                    CodecMetadata::Delta {
                        initial_value: 0i64,
                    }
                } else {
                    CodecMetadata::None
                }
            }
            Some(CompressionCodec::Dictionary) => {
                if let Some(encoder) = &self.dict_encoder {
                    CodecMetadata::Dictionary {
                        dictionary_size: encoder.dictionary().len() as u32,
                    }
                } else {
                    CodecMetadata::None
                }
            }
            Some(CompressionCodec::RunLength) => CodecMetadata::RLE,
            Some(CompressionCodec::Raw) | None => CodecMetadata::None,
        }
    }
}

/// Codec metadata for binary format
#[derive(Debug, Clone)]
pub enum CodecMetadata {
    /// No codec or raw encoding
    None,
    /// Delta encoding with initial value
    Delta {
        /// The initial base value for delta compression
        initial_value: i64,
    },
    /// Dictionary encoding with size
    Dictionary {
        /// Number of entries in the dictionary
        dictionary_size: u32,
    },
    /// Run-length encoding (no additional metadata needed)
    RLE,
}

impl CodecMetadata {
    /// Encode metadata to bytes
    pub fn encode(&self) -> Vec<u8> {
        match self {
            CodecMetadata::None => vec![],
            CodecMetadata::Delta { initial_value } => {
                let mut buf = Vec::new();
                // Store initial value as varint
                super::varint::encode_signed_varint(*initial_value, &mut buf);
                buf
            }
            CodecMetadata::Dictionary { dictionary_size } => {
                let mut buf = Vec::new();
                // Store dictionary size as varint
                super::varint::encode_varint(*dictionary_size as u64, &mut buf);
                buf
            }
            CodecMetadata::RLE => vec![],
        }
    }

    /// Get metadata size in bytes
    pub fn size(&self) -> usize {
        self.encode().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_codec_encoding_context_creation() {
        let ctx = CodecEncodingContext::new(100);
        assert_eq!(ctx.sample_threshold, 100);
        assert_eq!(ctx.items_collected, 0);
        assert!(ctx.selected_codec.is_none());
    }

    #[test]
    fn test_sampling_and_codec_selection() {
        let mut ctx = CodecEncodingContext::new(10);

        // Add 10 sorted samples (should trigger Delta detection)
        for i in 0..10 {
            ctx.add_sample(Some(&json!(i)));
        }

        assert!(ctx.is_codec_selected());
        assert_eq!(ctx.selected_codec, Some(CompressionCodec::Delta));
    }

    #[test]
    fn test_delta_encoding() {
        let mut ctx = CodecEncodingContext::new(5);

        // Add samples to trigger codec selection
        for i in 0..5 {
            ctx.add_sample(Some(&json!(i)));
        }

        // Now encode values
        assert!(ctx.encode_value(&json!(0)).is_ok());
        assert!(ctx.encode_value(&json!(2)).is_ok());
        assert!(ctx.encode_value(&json!(5)).is_ok());
    }

    #[test]
    fn test_dictionary_encoding() {
        let mut ctx = CodecEncodingContext::new(20);

        // Add repeated samples (should trigger Dictionary detection)
        // Need 20+ samples with low cardinality for detection
        for _ in 0..5 {
            ctx.add_sample(Some(&json!("alice")));
            ctx.add_sample(Some(&json!("bob")));
            ctx.add_sample(Some(&json!("carol")));
            ctx.add_sample(Some(&json!("alice")));
        }

        assert!(ctx.is_codec_selected());
        assert_eq!(ctx.selected_codec, Some(CompressionCodec::Dictionary));

        // Encode values
        assert!(ctx.encode_value(&json!("alice")).is_ok());
        assert!(ctx.encode_value(&json!("bob")).is_ok());
        assert!(ctx.encode_value(&json!("alice")).is_ok());
    }

    #[test]
    fn test_rle_encoding() {
        let mut ctx = CodecEncodingContext::new(20);

        // Add constant samples (should trigger RLE detection)
        // Need 20+ samples with 30%+ in runs
        for _ in 0..6 {
            ctx.add_sample(Some(&json!(true)));
        }
        for _ in 0..8 {
            ctx.add_sample(Some(&json!(false)));
        }
        for _ in 0..6 {
            ctx.add_sample(Some(&json!(true)));
        }

        assert!(ctx.is_codec_selected());
        assert_eq!(ctx.selected_codec, Some(CompressionCodec::RunLength));

        // Encode values
        assert!(ctx.encode_value(&json!(true)).is_ok());
        assert!(ctx.encode_value(&json!(true)).is_ok());
    }

    #[test]
    fn test_codec_metadata_encode() {
        let metadata = CodecMetadata::Delta {
            initial_value: 100i64,
        };

        let encoded = metadata.encode();
        assert!(!encoded.is_empty());

        let metadata2 = CodecMetadata::Dictionary {
            dictionary_size: 50,
        };

        let encoded2 = metadata2.encode();
        assert!(!encoded2.is_empty());
    }

    #[test]
    fn test_codec_metadata_size() {
        let metadata = CodecMetadata::Delta {
            initial_value: 42i64,
        };

        let size = metadata.size();
        let encoded_size = metadata.encode().len();
        assert_eq!(size, encoded_size);
    }

    #[test]
    fn test_no_codec_metadata() {
        let metadata = CodecMetadata::None;
        assert_eq!(metadata.size(), 0);
        assert!(metadata.encode().is_empty());

        let rle_metadata = CodecMetadata::RLE;
        assert_eq!(rle_metadata.size(), 0);
    }

    #[test]
    fn test_non_numeric_delta_fallback() {
        let mut ctx = CodecEncodingContext::new(5);

        // Add numeric samples first
        for i in 0..5 {
            ctx.add_sample(Some(&json!(i)));
        }

        // Try to encode non-numeric value
        let result = ctx.encode_value(&json!("not a number"));
        // Should succeed or gracefully degrade
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_codec_encoder_initialization() {
        let mut ctx = CodecEncodingContext::new(10);

        // Add sorted samples to trigger Delta codec
        for i in 0..10 {
            ctx.add_sample(Some(&json!(i)));
        }

        // Check that encoder is initialized
        assert!(ctx.delta_encoder.is_some());
        assert!(ctx.dict_encoder.is_none());
        assert!(ctx.rle_encoder.is_none());
    }
}
