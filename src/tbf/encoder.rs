//! TBF Encoder with schema-aware encoding
//!
//! The encoder supports two modes:
//! 1. **Self-describing mode**: Every value has a type tag (flexible but larger)
//! 2. **Schema mode**: For homogeneous sequences, emit schema once then values without tags

use super::dictionary::StringDictionary;
use super::schema::{Schema, SchemaField, SchemaType, SchemaRegistry};
use super::stats_collector::StatisticsCollector;
use super::varint::*;
use super::{TBF_MAGIC, TBF_VERSION, FLAG_DICTIONARY, TypeTag};

/// Encoding mode flags
pub const MODE_SELF_DESCRIBING: u8 = 0x00;
pub const MODE_SCHEMA: u8 = 0x01;

/// Special marker for schema-encoded sequence
pub const MARKER_SCHEMA_SEQ: u8 = 0xFE;

/// TBF Serializer - encodes values directly to TBF binary format
///
/// Supports both self-describing and schema-based encoding for optimal size.
pub struct TbfSerializer {
    /// Output buffer for data
    pub(crate) buf: Vec<u8>,
    /// String dictionary for deduplication
    pub(crate) dict: StringDictionary,
    /// Schema registry
    pub(crate) schemas: SchemaRegistry,
    /// Schema buffer (written after dictionary)
    pub(crate) schema_buf: Vec<u8>,
    /// Current encoding context
    pub(crate) context: EncodingContext,
    /// Nesting depth
    pub(crate) depth: usize,
    /// Optional statistics collector (Phase 2)
    pub(crate) stats: Option<StatisticsCollector>,
}

/// Tracks the current encoding context for schema detection
#[derive(Debug, Clone, Default)]
pub struct EncodingContext {
    /// Are we inside a sequence?
    pub in_sequence: bool,
    /// Number of elements seen in current sequence
    pub seq_element_count: usize,
    /// Detected schema for current sequence (if homogeneous)
    pub seq_schema: Option<DetectedSchema>,
    /// Current struct field names being collected
    pub current_fields: Vec<(String, SchemaType)>,
    /// Schema index for current sequence (if schema mode)
    pub seq_schema_idx: Option<u32>,
    /// Are we in schema mode for current sequence?
    pub schema_mode: bool,
}

/// A schema detected during serialization
#[derive(Debug, Clone)]
pub struct DetectedSchema {
    /// Field names in order
    pub fields: Vec<String>,
    /// Field types in order
    pub types: Vec<SchemaType>,
}

impl TbfSerializer {
    /// Create a new serializer
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(1024),
            dict: StringDictionary::new(),
            schemas: SchemaRegistry::new(),
            schema_buf: Vec::new(),
            context: EncodingContext::default(),
            depth: 0,
            stats: None,
        }
    }

    /// Create a serializer with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
            dict: StringDictionary::with_capacity(capacity / 32),
            schemas: SchemaRegistry::new(),
            schema_buf: Vec::new(),
            context: EncodingContext::default(),
            depth: 0,
            stats: None,
        }
    }

    /// Create a serializer with statistics collection enabled
    pub fn with_statistics() -> Self {
        Self {
            buf: Vec::with_capacity(1024),
            dict: StringDictionary::new(),
            schemas: SchemaRegistry::new(),
            schema_buf: Vec::new(),
            context: EncodingContext::default(),
            depth: 0,
            stats: Some(StatisticsCollector::new()),
        }
    }

    /// Create a serializer with pre-allocated capacity and statistics collection
    pub fn with_capacity_and_statistics(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
            dict: StringDictionary::with_capacity(capacity / 32),
            schemas: SchemaRegistry::new(),
            schema_buf: Vec::new(),
            context: EncodingContext::default(),
            depth: 0,
            stats: Some(StatisticsCollector::new()),
        }
    }

    /// Finalize and get the encoded bytes
    pub fn into_bytes(mut self) -> Vec<u8> {
        // Encode dictionary
        let mut dict_buf = Vec::new();
        self.dict.encode(&mut dict_buf);

        // Encode schemas
        self.schemas.encode(&mut self.schema_buf, &mut self.dict);
        // Re-encode dictionary since schema encoding may have added strings
        dict_buf.clear();
        self.dict.encode(&mut dict_buf);

        // Determine mode
        let mode = if self.schemas.is_empty() {
            MODE_SELF_DESCRIBING
        } else {
            MODE_SCHEMA
        };

        let mut result = Vec::with_capacity(8 + dict_buf.len() + self.schema_buf.len() + self.buf.len());

        // Write header
        result.extend_from_slice(&TBF_MAGIC);
        result.push(TBF_VERSION);
        result.push(FLAG_DICTIONARY | (mode << 4));
        result.extend_from_slice(&[0u8; 2]); // Reserved

        // Write dictionary
        result.extend_from_slice(&dict_buf);

        // Write schemas (if any)
        if mode == MODE_SCHEMA {
            result.extend_from_slice(&self.schema_buf);
        }

        // Write data
        result.extend_from_slice(&self.buf);

        // Write statistics footer (Phase 2)
        if let Some(stats) = self.stats {
            if let Ok(stats_bytes) = stats.encode_all() {
                // Store offset to footer for random access
                let footer_offset = result.len() as u64;
                result.extend_from_slice(&stats_bytes);
                // Append footer offset (8 bytes, little-endian)
                result.extend_from_slice(&footer_offset.to_le_bytes());
            }
        }

        result
    }

    /// Get reference to output buffer
    pub fn output(&self) -> &[u8] {
        &self.buf
    }

    /// Write a type tag
    #[inline(always)]
    pub(crate) fn write_tag(&mut self, tag: TypeTag) {
        self.buf.push(tag as u8);
    }

    /// Write a varint
    #[inline(always)]
    pub(crate) fn write_varint(&mut self, value: u64) {
        encode_varint(value, &mut self.buf);
    }

    /// Write a signed varint
    #[inline(always)]
    pub(crate) fn write_signed_varint(&mut self, value: i64) {
        encode_signed_varint(value, &mut self.buf);
    }

    /// Intern a string and write its index
    #[inline]
    pub(crate) fn write_string(&mut self, s: &str) {
        let idx = self.dict.intern(s);
        encode_varint(idx as u64, &mut self.buf);
    }

    /// Write a value without type tag (schema mode)
    #[inline]
    pub(crate) fn write_typed_value_bool(&mut self, v: bool) {
        self.buf.push(if v { 1 } else { 0 });
    }

    #[inline]
    pub(crate) fn write_typed_value_int(&mut self, v: i64) {
        encode_signed_varint(v, &mut self.buf);
    }

    #[inline]
    pub(crate) fn write_typed_value_uint(&mut self, v: u64) {
        encode_varint(v, &mut self.buf);
    }

    #[inline]
    pub(crate) fn write_typed_value_f32(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    #[inline]
    pub(crate) fn write_typed_value_f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    #[inline]
    pub(crate) fn write_typed_value_string(&mut self, s: &str) {
        let idx = self.dict.intern(s);
        encode_varint(idx as u64, &mut self.buf);
    }

    /// Enter a nested structure
    #[inline]
    pub(crate) fn enter(&mut self) {
        self.depth += 1;
    }

    /// Leave a nested structure
    #[inline]
    pub(crate) fn leave(&mut self) {
        self.depth -= 1;
    }

    /// Check if at root level
    #[inline]
    pub(crate) fn is_root(&self) -> bool {
        self.depth == 0
    }

    /// Begin a sequence - returns a helper for schema detection
    pub(crate) fn begin_sequence(&mut self, len: Option<usize>) -> SequenceEncoder<'_> {
        SequenceEncoder::new(self, len)
    }

    /// Check if currently in schema mode for structs
    pub(crate) fn in_schema_mode(&self) -> bool {
        self.context.schema_mode && self.context.seq_schema.is_some()
    }

    /// Get expected field type for current position (schema mode)
    pub(crate) fn get_expected_field_type(&self, field_idx: usize) -> Option<SchemaType> {
        self.context.seq_schema.as_ref()
            .and_then(|s| s.types.get(field_idx).copied())
    }
}

impl Default for TbfSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper for encoding sequences with schema detection
pub struct SequenceEncoder<'a> {
    serializer: &'a mut TbfSerializer,
    len: Option<usize>,
    element_count: usize,
    schema_detected: bool,
    first_element_buf: Vec<u8>,
    first_element_fields: Vec<(String, SchemaType)>,
}

impl<'a> SequenceEncoder<'a> {
    fn new(serializer: &'a mut TbfSerializer, len: Option<usize>) -> Self {
        Self {
            serializer,
            len,
            element_count: 0,
            schema_detected: false,
            first_element_buf: Vec::new(),
            first_element_fields: Vec::new(),
        }
    }

    /// Called before each element
    pub fn before_element(&mut self) {
        self.element_count += 1;
    }

    /// Called when a struct starts within the sequence
    pub fn struct_started(&mut self, field_count: usize) {
        if self.element_count == 1 {
            // First element - start collecting field info
            self.serializer.context.current_fields = Vec::with_capacity(field_count);
        }
    }

    /// Called when a struct field is serialized
    pub fn field_serialized(&mut self, name: &str, typ: SchemaType) {
        if self.element_count == 1 {
            self.serializer.context.current_fields.push((name.to_string(), typ));
        }
    }

    /// Called when first struct in sequence ends
    pub fn first_struct_ended(&mut self) {
        if self.element_count == 1 && !self.serializer.context.current_fields.is_empty() {
            // Create schema from collected fields
            let fields: Vec<String> = self.serializer.context.current_fields.iter()
                .map(|(n, _)| n.clone())
                .collect();
            let types: Vec<SchemaType> = self.serializer.context.current_fields.iter()
                .map(|(_, t)| *t)
                .collect();

            let detected = DetectedSchema { fields, types };
            self.serializer.context.seq_schema = Some(detected);
            self.serializer.context.schema_mode = true;
            self.schema_detected = true;
        }
    }

    /// Finalize the sequence encoder
    pub fn finish(self) {
        // Reset context
        self.serializer.context.in_sequence = false;
        self.serializer.context.seq_element_count = 0;
        self.serializer.context.seq_schema = None;
        self.serializer.context.schema_mode = false;
        self.serializer.context.current_fields.clear();
    }
}

/// Schema-aware struct serializer
pub struct SchemaStructSerializer<'a> {
    serializer: &'a mut TbfSerializer,
    field_idx: usize,
    schema_mode: bool,
    expected_types: Option<Vec<SchemaType>>,
}

impl<'a> SchemaStructSerializer<'a> {
    pub fn new(serializer: &'a mut TbfSerializer, schema_mode: bool) -> Self {
        let expected_types = if schema_mode {
            serializer.context.seq_schema.as_ref().map(|s| s.types.clone())
        } else {
            None
        };

        Self {
            serializer,
            field_idx: 0,
            schema_mode,
            expected_types,
        }
    }

    /// Get expected type for current field
    pub fn expected_type(&self) -> Option<SchemaType> {
        self.expected_types.as_ref()
            .and_then(|types| types.get(self.field_idx).copied())
    }

    /// Move to next field
    pub fn next_field(&mut self) {
        self.field_idx += 1;
    }

    /// Check if in schema mode
    pub fn is_schema_mode(&self) -> bool {
        self.schema_mode
    }
}
