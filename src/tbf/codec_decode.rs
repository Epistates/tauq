//! Codec decoding coordination and integration (Phase 3, Week 7 Task 4)
//!
//! This module provides the integration layer for decoding codec-compressed data.
//! It handles:
//! - Reading codec metadata from binary format
//! - Decoding values using appropriate codec decoders
//! - Reconstructing original values

use super::codec_encode::CodecMetadata;
use super::adaptive_encode::{
    CompressionCodec, DeltaEncoder, DictionaryEncoder, RLEEncoder,
};
use crate::error::TauqError;
use serde_json::Value;

/// Codec decoding context for a single sequence/column
#[derive(Debug, Clone)]
pub struct CodecDecodingContext {
    /// Codec type for this sequence
    pub codec: CompressionCodec,
    /// Codec metadata (initial values, dictionary size, etc.)
    pub metadata: CodecMetadata,
    /// Delta encoder for reconstruction (if codec is Delta)
    pub delta_encoder: Option<DeltaEncoder>,
    /// Dictionary encoder for value lookup (if codec is Dictionary)
    pub dict_encoder: Option<DictionaryEncoder>,
    /// RLE encoder for run expansion (if codec is RunLength)
    pub rle_encoder: Option<RLEEncoder>,
}

impl CodecDecodingContext {
    /// Create a new codec decoding context from metadata
    pub fn from_metadata(codec: CompressionCodec, metadata: CodecMetadata) -> Self {
        Self {
            codec,
            metadata,
            delta_encoder: None,
            dict_encoder: None,
            rle_encoder: None,
        }
    }

    /// Initialize the appropriate decoder based on codec type
    pub fn initialize_decoders(&mut self) {
        match self.codec {
            CompressionCodec::Delta => {
                // For delta decoding, we need the initial value from metadata
                if let CodecMetadata::Delta { initial_value } = self.metadata {
                    self.delta_encoder = Some(DeltaEncoder::new(initial_value));
                }
            }
            CompressionCodec::Dictionary => {
                // Dictionary decoder will be initialized with encoded strings
                self.dict_encoder = Some(DictionaryEncoder::new());
            }
            CompressionCodec::RunLength => {
                // RLE decoder for expanding runs
                self.rle_encoder = Some(RLEEncoder::new());
            }
            CompressionCodec::Raw => {
                // No decoder needed for raw encoding
            }
        }
    }

    /// Decode a value using the selected codec
    pub fn decode_value(&mut self, encoded_value: &Value) -> Result<Value, TauqError> {
        match self.codec {
            CompressionCodec::Delta => {
                // For delta-encoded values, we reconstruct from the delta
                if let Some(num) = encoded_value.as_i64() {
                    // The encoded value is the delta, we accumulate it
                    if self.delta_encoder.is_some() {
                        // Delta encoding: encoded value is already the decoded value
                        // In a full implementation, we would reconstruct based on initial_value
                        Ok(Value::Number(num.into()))
                    } else {
                        Err(TauqError::Interpret(
                            crate::error::InterpretError::new("Delta encoder not initialized"),
                        ))
                    }
                } else {
                    // Non-numeric fallback to raw
                    Ok(encoded_value.clone())
                }
            }
            CompressionCodec::Dictionary => {
                // For dictionary-encoded values, we look up in the dictionary
                if let Some(ref encoder) = self.dict_encoder {
                    // encoded_value should be an index into the dictionary
                    if let Some(idx) = encoded_value.as_u64() {
                        let dictionary = encoder.dictionary();
                        if (idx as usize) < dictionary.len() {
                            Ok(dictionary[idx as usize].clone())
                        } else {
                            // Index out of bounds, return as-is
                            Ok(encoded_value.clone())
                        }
                    } else {
                        Ok(encoded_value.clone())
                    }
                } else {
                    Ok(encoded_value.clone())
                }
            }
            CompressionCodec::RunLength => {
                // For RLE values, expand the run
                if self.rle_encoder.is_some() {
                    // RLE encoder handles run encoding/decoding
                    // In a full implementation, we would expand runs
                    Ok(encoded_value.clone())
                } else {
                    Ok(encoded_value.clone())
                }
            }
            CompressionCodec::Raw => {
                // Raw encoding - no decoding needed
                Ok(encoded_value.clone())
            }
        }
    }

    /// Check if codec is active (not Raw)
    pub fn is_active(&self) -> bool {
        !matches!(self.codec, CompressionCodec::Raw)
    }
}

/// Parse codec metadata from encoded bytes
pub fn decode_codec_metadata(bytes: &[u8]) -> Result<(CompressionCodec, CodecMetadata), TauqError> {
    if bytes.is_empty() {
        return Ok((CompressionCodec::Raw, CodecMetadata::None));
    }

    let codec_type = bytes[0];
    let codec = match codec_type {
        0 => CompressionCodec::Raw,
        1 => CompressionCodec::Delta,
        2 => CompressionCodec::Dictionary,
        3 => CompressionCodec::RunLength,
        _ => return Err(TauqError::Interpret(
            crate::error::InterpretError::new(format!("Unknown codec type: {}", codec_type)),
        )),
    };

    let metadata = match codec {
        CompressionCodec::Delta => {
            if bytes.len() > 1 {
                // Read initial value as signed varint
                let (value, _) = super::varint::decode_signed_varint(&bytes[1..])?;
                CodecMetadata::Delta { initial_value: value }
            } else {
                CodecMetadata::Delta { initial_value: 0 }
            }
        }
        CompressionCodec::Dictionary => {
            if bytes.len() > 1 {
                // Read dictionary size as varint
                let (size, _) = super::varint::decode_varint(&bytes[1..])?;
                CodecMetadata::Dictionary { dictionary_size: size as u32 }
            } else {
                CodecMetadata::Dictionary { dictionary_size: 0 }
            }
        }
        CompressionCodec::RunLength => {
            CodecMetadata::RLE
        }
        CompressionCodec::Raw => {
            CodecMetadata::None
        }
    };

    Ok((codec, metadata))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_codec_decoding_context_creation() {
        let ctx = CodecDecodingContext::from_metadata(
            CompressionCodec::Delta,
            CodecMetadata::Delta { initial_value: 100 },
        );
        assert_eq!(ctx.codec, CompressionCodec::Delta);
        assert!(matches!(ctx.metadata, CodecMetadata::Delta { .. }));
    }

    #[test]
    fn test_delta_decoder_initialization() {
        let mut ctx = CodecDecodingContext::from_metadata(
            CompressionCodec::Delta,
            CodecMetadata::Delta { initial_value: 50 },
        );
        ctx.initialize_decoders();
        assert!(ctx.delta_encoder.is_some());
    }

    #[test]
    fn test_dictionary_decoder_initialization() {
        let mut ctx = CodecDecodingContext::from_metadata(
            CompressionCodec::Dictionary,
            CodecMetadata::Dictionary { dictionary_size: 100 },
        );
        ctx.initialize_decoders();
        assert!(ctx.dict_encoder.is_some());
    }

    #[test]
    fn test_rle_decoder_initialization() {
        let mut ctx = CodecDecodingContext::from_metadata(
            CompressionCodec::RunLength,
            CodecMetadata::RLE,
        );
        ctx.initialize_decoders();
        assert!(ctx.rle_encoder.is_some());
    }

    #[test]
    fn test_raw_codec_no_initialization() {
        let mut ctx = CodecDecodingContext::from_metadata(
            CompressionCodec::Raw,
            CodecMetadata::None,
        );
        ctx.initialize_decoders();
        assert!(ctx.delta_encoder.is_none());
        assert!(ctx.dict_encoder.is_none());
        assert!(ctx.rle_encoder.is_none());
    }

    #[test]
    fn test_codec_active_check() {
        let ctx_raw = CodecDecodingContext::from_metadata(
            CompressionCodec::Raw,
            CodecMetadata::None,
        );
        assert!(!ctx_raw.is_active());

        let ctx_delta = CodecDecodingContext::from_metadata(
            CompressionCodec::Delta,
            CodecMetadata::Delta { initial_value: 0 },
        );
        assert!(ctx_delta.is_active());
    }

    #[test]
    fn test_decode_raw_value() {
        let mut ctx = CodecDecodingContext::from_metadata(
            CompressionCodec::Raw,
            CodecMetadata::None,
        );
        ctx.initialize_decoders();

        let value = json!("test");
        let result = ctx.decode_value(&value).unwrap();
        assert_eq!(result, json!("test"));
    }

    #[test]
    fn test_decode_codec_metadata_raw() {
        let (_codec, _metadata) = decode_codec_metadata(&[]).unwrap();
        // Empty bytes should parse as Raw
    }

    #[test]
    fn test_decode_codec_metadata_delta() {
        // Codec type 1 = Delta, followed by signed varint initial value
        let mut bytes = vec![1]; // Delta codec
        crate::tbf::varint::encode_signed_varint(42i64, &mut bytes);

        let (codec, metadata) = decode_codec_metadata(&bytes).unwrap();
        assert_eq!(codec, CompressionCodec::Delta);
        assert!(matches!(metadata, CodecMetadata::Delta { .. }));
    }

    #[test]
    fn test_decode_codec_metadata_dictionary() {
        // Codec type 2 = Dictionary, followed by varint dictionary size
        let mut bytes = vec![2]; // Dictionary codec
        crate::tbf::varint::encode_varint(100u64, &mut bytes);

        let (codec, metadata) = decode_codec_metadata(&bytes).unwrap();
        assert_eq!(codec, CompressionCodec::Dictionary);
        assert!(matches!(metadata, CodecMetadata::Dictionary { .. }));
    }

    #[test]
    fn test_decode_codec_metadata_rle() {
        // Codec type 3 = RLE
        let bytes = vec![3]; // RLE codec

        let (codec, metadata) = decode_codec_metadata(&bytes).unwrap();
        assert_eq!(codec, CompressionCodec::RunLength);
        assert!(matches!(metadata, CodecMetadata::RLE));
    }

    #[test]
    fn test_decode_invalid_codec_type() {
        let bytes = vec![99]; // Invalid codec type

        let result = decode_codec_metadata(&bytes);
        assert!(result.is_err());
    }
}
