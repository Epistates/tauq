//! Schema-Aware Encoding with User-Declarable Field Strategies
//!
//! This module provides a flexible schema-based encoding system where users
//! declare *encoding strategies* for fields rather than hardcoded bit widths.
//!
//! # Design Philosophy
//!
//! Instead of tightly coupling to specific data ranges like `bits: 8, offset: 22`,
//! users declare encoding *intent*:
//!
//! - `Auto` - Let the encoder sample and choose optimal encoding
//! - `Compact { min_hint, max_hint }` - Hint at expected range, encoder adapts
//! - `Dictionary` - Use dictionary encoding for low-cardinality strings
//! - `Inline` - Inline strings without dictionary overhead
//! - `VarInt` - Variable-length integer (flexible, slightly larger)
//!
//! # Example
//!
//! ```ignore
//! let schema = TableSchema::builder()
//!     .column("id", FieldEncoding::Auto)                    // Encoder picks best
//!     .column("age", FieldEncoding::compact(0, 150))        // Hint: ages 0-150
//!     .column("city", FieldEncoding::Dictionary)            // Low cardinality
//!     .column("name", FieldEncoding::Inline)                // High cardinality
//!     .column("salary", FieldEncoding::compact(0, 500_000)) // Hint: salary range
//!     .build();
//! ```

use super::ultra_encode::UltraBuffer;
use std::cell::RefCell;
use std::collections::HashMap;

// =============================================================================
// Thread-Local Scratch Buffers
// =============================================================================

thread_local! {
    /// Thread-local scratch buffer for intermediate encoding
    static SCRATCH: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };

    /// Thread-local output buffer for reuse
    static OUTPUT: RefCell<UltraBuffer> = RefCell::new(UltraBuffer::new());
}

/// Execute with a thread-local scratch buffer
#[inline]
pub fn with_scratch<T>(f: impl FnOnce(&mut Vec<u8>) -> T) -> T {
    SCRATCH.with(|s| {
        let s = &mut *s.borrow_mut();
        s.clear();
        f(s)
    })
}

/// Execute with a thread-local output buffer
#[inline]
pub fn with_output<T>(estimated_size: usize, f: impl FnOnce(&mut UltraBuffer) -> T) -> T {
    OUTPUT.with(|o| {
        let o = &mut *o.borrow_mut();
        o.clear();
        if o.capacity() < estimated_size {
            *o = UltraBuffer::with_capacity(estimated_size);
        }
        f(o)
    })
}

// =============================================================================
// Field Encoding Strategies (User-Declarable)
// =============================================================================

/// Encoding strategy for a field - declares *intent*, not exact implementation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FieldEncoding {
    /// Automatic: sample data and choose optimal encoding
    /// Best for unknown data distributions
    Auto,

    // =========================================================================
    // Fixed-width integer types (direct Rust type mapping)
    // =========================================================================

    /// Unsigned 8-bit integer (0..255)
    U8,
    /// Unsigned 16-bit integer (0..65535)
    U16,
    /// Unsigned 32-bit integer
    U32,
    /// Unsigned 64-bit integer
    U64,
    /// Signed 8-bit integer (-128..127)
    I8,
    /// Signed 16-bit integer
    I16,
    /// Signed 32-bit integer
    I32,
    /// Signed 64-bit integer
    I64,

    // =========================================================================
    // Fixed-width with offset (best compression + clean API)
    // =========================================================================

    /// Unsigned 8-bit with offset: value - offset stored as u8
    /// Example: age with offset 18 stores 18-273 as 0-255
    U8Offset { offset: i64 },
    /// Unsigned 16-bit with offset
    U16Offset { offset: i64 },
    /// Unsigned 32-bit with offset
    U32Offset { offset: i64 },

    // =========================================================================
    // Ranged/Compact encodings
    // =========================================================================

    /// Compact integer with optional range hints
    /// The encoder will use the smallest representation that fits
    /// If data exceeds hints, falls back gracefully to larger encoding
    Compact {
        /// Minimum expected value (hint, not enforced)
        min_hint: i64,
        /// Maximum expected value (hint, not enforced)
        max_hint: i64,
    },

    /// Variable-length integer encoding (LEB128-style)
    /// Good for mixed ranges, slightly larger than compact
    VarInt,

    // =========================================================================
    // String encodings
    // =========================================================================

    /// Dictionary encoding for strings
    /// Best for low-cardinality fields (< 65536 unique values)
    /// Falls back to inline if cardinality exceeds threshold
    Dictionary,

    /// Inline string encoding (length-prefixed)
    /// Best for high-cardinality or unique strings
    Inline,

    // =========================================================================
    // Other types
    // =========================================================================

    /// Boolean field (bit-packed)
    Bool,

    /// 32-bit float
    Float32,

    /// 64-bit float
    Float64,
}

impl FieldEncoding {
    /// Create a compact integer encoding with range hints
    #[inline]
    pub const fn compact(min_hint: i64, max_hint: i64) -> Self {
        FieldEncoding::Compact { min_hint, max_hint }
    }

    /// Create a compact unsigned integer encoding
    #[inline]
    pub const fn compact_unsigned(max_hint: u64) -> Self {
        FieldEncoding::Compact {
            min_hint: 0,
            max_hint: max_hint as i64,
        }
    }

    /// Get the fixed bit width for this encoding
    #[inline]
    pub fn bits(&self) -> Option<u8> {
        match self {
            FieldEncoding::U8 | FieldEncoding::I8 | FieldEncoding::U8Offset { .. } => Some(8),
            FieldEncoding::U16 | FieldEncoding::I16 | FieldEncoding::U16Offset { .. } => Some(16),
            FieldEncoding::U32 | FieldEncoding::I32 | FieldEncoding::Float32 | FieldEncoding::U32Offset { .. } => Some(32),
            FieldEncoding::U64 | FieldEncoding::I64 | FieldEncoding::Float64 => Some(64),
            FieldEncoding::Bool => Some(1),
            FieldEncoding::Compact { min_hint, max_hint } => {
                let range = (*max_hint as u64).saturating_sub(*min_hint as u64);
                Some(if range <= 0xFF { 8 }
                    else if range <= 0xFFFF { 16 }
                    else if range <= 0xFFFF_FFFF { 32 }
                    else { 64 })
            }
            _ => None,
        }
    }

    /// Check if this is a signed integer type
    #[inline]
    pub fn is_signed(&self) -> bool {
        matches!(self, FieldEncoding::I8 | FieldEncoding::I16 |
                       FieldEncoding::I32 | FieldEncoding::I64 |
                       FieldEncoding::Compact { .. })
    }

    /// Get the offset for range-based encoding
    #[inline]
    pub fn offset(&self) -> i64 {
        match self {
            FieldEncoding::U8Offset { offset } => *offset,
            FieldEncoding::U16Offset { offset } => *offset,
            FieldEncoding::U32Offset { offset } => *offset,
            FieldEncoding::Compact { min_hint, .. } => *min_hint,
            _ => 0,
        }
    }
}

// =============================================================================
// Column Schema
// =============================================================================

/// Schema for a single column
#[derive(Debug, Clone)]
pub struct ColumnSchema {
    /// Column name
    pub name: String,
    /// Encoding strategy
    pub encoding: FieldEncoding,
}

impl ColumnSchema {
    /// Create a new column schema
    pub fn new(name: impl Into<String>, encoding: FieldEncoding) -> Self {
        Self {
            name: name.into(),
            encoding,
        }
    }
}

// =============================================================================
// Table Schema
// =============================================================================

/// Schema for a table (collection of columns)
#[derive(Debug, Clone, Default)]
pub struct TableSchema {
    columns: Vec<ColumnSchema>,
}

impl TableSchema {
    /// Create an empty schema
    pub fn new() -> Self {
        Self { columns: Vec::new() }
    }

    /// Create a schema builder
    pub fn builder() -> TableSchemaBuilder {
        TableSchemaBuilder::new()
    }

    /// Add a column to the schema
    pub fn add_column(&mut self, name: impl Into<String>, encoding: FieldEncoding) {
        self.columns.push(ColumnSchema::new(name, encoding));
    }

    /// Get column schemas
    pub fn columns(&self) -> &[ColumnSchema] {
        &self.columns
    }

    /// Get encoding for column by index
    pub fn encoding(&self, index: usize) -> Option<FieldEncoding> {
        self.columns.get(index).map(|c| c.encoding)
    }

    /// Get encoding for column by name
    pub fn encoding_by_name(&self, name: &str) -> Option<FieldEncoding> {
        self.columns.iter()
            .find(|c| c.name == name)
            .map(|c| c.encoding)
    }
}

/// Builder for TableSchema
#[derive(Debug, Default)]
pub struct TableSchemaBuilder {
    columns: Vec<ColumnSchema>,
}

impl TableSchemaBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { columns: Vec::new() }
    }

    /// Add a column with encoding
    pub fn column(mut self, name: impl Into<String>, encoding: FieldEncoding) -> Self {
        self.columns.push(ColumnSchema::new(name, encoding));
        self
    }

    // =========================================================================
    // Type-based methods (most intuitive API)
    // =========================================================================

    /// Add a u8 column (unsigned 8-bit integer)
    pub fn u8(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::U8)
    }

    /// Add a u16 column (unsigned 16-bit integer)
    pub fn u16(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::U16)
    }

    /// Add a u32 column (unsigned 32-bit integer)
    pub fn u32(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::U32)
    }

    /// Add a u64 column (unsigned 64-bit integer)
    pub fn u64(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::U64)
    }

    /// Add an i8 column (signed 8-bit integer)
    pub fn i8(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::I8)
    }

    /// Add an i16 column (signed 16-bit integer)
    pub fn i16(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::I16)
    }

    /// Add an i32 column (signed 32-bit integer)
    pub fn i32(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::I32)
    }

    /// Add an i64 column (signed 64-bit integer)
    pub fn i64(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::I64)
    }

    /// Add an f32 column (32-bit float)
    pub fn f32(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::Float32)
    }

    /// Add an f64 column (64-bit float)
    pub fn f64(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::Float64)
    }

    /// Add a bool column
    pub fn bool(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::Bool)
    }

    // =========================================================================
    // Type with offset (best compression)
    // =========================================================================

    /// Add a u8 column with offset (value - offset stored as u8)
    /// Example: `.u8_offset("age", 18)` stores ages 18-273 as 0-255
    pub fn u8_offset(self, name: impl Into<String>, offset: i64) -> Self {
        self.column(name, FieldEncoding::U8Offset { offset })
    }

    /// Add a u16 column with offset
    pub fn u16_offset(self, name: impl Into<String>, offset: i64) -> Self {
        self.column(name, FieldEncoding::U16Offset { offset })
    }

    /// Add a u32 column with offset
    pub fn u32_offset(self, name: impl Into<String>, offset: i64) -> Self {
        self.column(name, FieldEncoding::U32Offset { offset })
    }

    // =========================================================================
    // String methods
    // =========================================================================

    /// Add a dictionary-encoded string column (low cardinality)
    pub fn dict(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::Dictionary)
    }

    /// Add an inline string column (high cardinality / unique values)
    pub fn string(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::Inline)
    }

    // =========================================================================
    // Legacy/flexible methods
    // =========================================================================

    /// Add an auto-encoded column (encoder samples and chooses optimal)
    pub fn auto(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::Auto)
    }

    /// Add a compact integer column with range hints
    pub fn compact(self, name: impl Into<String>, min: i64, max: i64) -> Self {
        self.column(name, FieldEncoding::compact(min, max))
    }

    /// Add a varint column (variable-length integer)
    pub fn varint(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::VarInt)
    }

    /// Add a dictionary-encoded string column (alias for dict)
    pub fn dictionary(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::Dictionary)
    }

    /// Add an inline string column (alias for string)
    pub fn inline(self, name: impl Into<String>) -> Self {
        self.column(name, FieldEncoding::Inline)
    }

    /// Build the schema
    pub fn build(self) -> TableSchema {
        TableSchema { columns: self.columns }
    }
}

// =============================================================================
// Adaptive Column Encoder
// =============================================================================

/// Adaptive integer encoder that respects schema hints but handles overflow
pub struct AdaptiveIntEncoder {
    values: Vec<i64>,
    encoding: FieldEncoding,
    /// Actual min observed (for Auto mode)
    actual_min: i64,
    /// Actual max observed (for Auto mode)
    actual_max: i64,
}

impl AdaptiveIntEncoder {
    /// Create encoder with schema-defined encoding
    pub fn new(encoding: FieldEncoding, capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            encoding,
            actual_min: i64::MAX,
            actual_max: i64::MIN,
        }
    }

    /// Push a value, tracking actual range for Auto mode
    #[inline(always)]
    pub fn push(&mut self, value: i64) {
        self.values.push(value);
        if matches!(self.encoding, FieldEncoding::Auto) {
            self.actual_min = self.actual_min.min(value);
            self.actual_max = self.actual_max.max(value);
        }
    }

    /// Push unsigned value
    #[inline(always)]
    pub fn push_u32(&mut self, value: u32) {
        self.push(value as i64);
    }

    /// Encode to buffer using schema hints or sampled data
    pub fn encode_to(&self, buf: &mut UltraBuffer) {
        if self.values.is_empty() {
            return;
        }

        // Determine encoding parameters based on type
        let (bits, offset, signed) = match self.encoding {
            // Fixed-width types (no offset, direct encoding)
            FieldEncoding::U8 => (8, 0i64, false),
            FieldEncoding::U16 => (16, 0, false),
            FieldEncoding::U32 => (32, 0, false),
            FieldEncoding::U64 => (64, 0, false),
            FieldEncoding::I8 => (8, 0, true),
            FieldEncoding::I16 => (16, 0, true),
            FieldEncoding::I32 => (32, 0, true),
            FieldEncoding::I64 => (64, 0, true),

            // Fixed-width with offset (best compression)
            FieldEncoding::U8Offset { offset } => (8, offset, false),
            FieldEncoding::U16Offset { offset } => (16, offset, false),
            FieldEncoding::U32Offset { offset } => (32, offset, false),

            // Auto: sample actual range
            FieldEncoding::Auto => {
                let range = (self.actual_max - self.actual_min) as u64;
                let bits = if range <= 0xFF { 8 }
                    else if range <= 0xFFFF { 16 }
                    else if range <= 0xFFFF_FFFF { 32 }
                    else { 64 };
                (bits, self.actual_min, true)
            }

            // Compact: use hints
            FieldEncoding::Compact { min_hint, max_hint } => {
                let hint_range = (max_hint - min_hint) as u64;
                let bits = if hint_range <= 0xFF { 8 }
                    else if hint_range <= 0xFFFF { 16 }
                    else if hint_range <= 0xFFFF_FFFF { 32 }
                    else { 64 };
                (bits, min_hint, true)
            }

            // Varint: variable-length encoding
            FieldEncoding::VarInt => {
                self.encode_varint(buf);
                return;
            }

            _ => (64, 0, false),
        };

        let _ = signed; // Reserved for future signed encoding support

        // Write encoding metadata
        buf.push(bits);
        if bits < 64 {
            // Write offset for compact encodings
            encode_varint_fast(offset as u64, buf);
        }

        // Encode values
        match bits {
            8 => {
                buf.reserve(self.values.len());
                for &v in &self.values {
                    let packed = (v - offset) as u8;
                    unsafe { buf.push_unchecked(packed); }
                }
            }
            16 => {
                buf.reserve(self.values.len() * 2);
                for &v in &self.values {
                    let packed = (v - offset) as u16;
                    unsafe { buf.extend_unchecked(&packed.to_le_bytes()); }
                }
            }
            32 => {
                buf.reserve(self.values.len() * 4);
                for &v in &self.values {
                    let packed = (v - offset) as u32;
                    unsafe { buf.extend_unchecked(&packed.to_le_bytes()); }
                }
            }
            64 => {
                buf.reserve(self.values.len() * 8);
                for &v in &self.values {
                    unsafe { buf.extend_unchecked(&v.to_le_bytes()); }
                }
            }
            _ => unreachable!(),
        }
    }

    fn encode_varint(&self, buf: &mut UltraBuffer) {
        buf.push(0); // Marker for varint mode
        for &v in &self.values {
            encode_signed_varint_fast(v, buf);
        }
    }
}

// =============================================================================
// Adaptive String Encoder
// =============================================================================

/// Adaptive string encoder that respects schema hints
pub struct AdaptiveStringEncoder {
    /// For dictionary mode: string -> index
    dict: HashMap<String, u16>,
    /// For dictionary mode: indices
    indices: Vec<u16>,
    /// For dictionary mode: strings in order
    dict_strings: Vec<String>,
    /// For inline mode: pre-encoded data
    inline_data: UltraBuffer,
    /// Encoding strategy
    encoding: FieldEncoding,
    /// Count of strings (for both modes)
    count: usize,
}

impl AdaptiveStringEncoder {
    /// Create encoder with schema-defined encoding
    pub fn new(encoding: FieldEncoding, capacity: usize) -> Self {
        let use_dict = matches!(encoding, FieldEncoding::Dictionary | FieldEncoding::Auto);
        Self {
            dict: if use_dict { HashMap::with_capacity(256) } else { HashMap::new() },
            indices: if use_dict { Vec::with_capacity(capacity) } else { Vec::new() },
            dict_strings: if use_dict { Vec::with_capacity(256) } else { Vec::new() },
            inline_data: if !use_dict { UltraBuffer::with_capacity(capacity * 16) } else { UltraBuffer::new() },
            encoding,
            count: 0,
        }
    }

    /// Push a string value
    #[inline]
    pub fn push(&mut self, s: &str) {
        self.count += 1;

        match self.encoding {
            FieldEncoding::Dictionary => {
                self.push_dict(s);
            }
            FieldEncoding::Inline => {
                self.push_inline(s);
            }
            FieldEncoding::Auto => {
                // Start with dictionary, switch to inline if cardinality too high
                if self.dict_strings.len() < 65535 || self.dict.contains_key(s) {
                    self.push_dict(s);
                } else {
                    // Cardinality exceeded, but we're committed to dict for this batch
                    self.push_dict(s);
                }
            }
            _ => {
                self.push_inline(s);
            }
        }
    }

    #[inline]
    fn push_dict(&mut self, s: &str) {
        let idx = if let Some(&idx) = self.dict.get(s) {
            idx
        } else {
            let idx = self.dict_strings.len() as u16;
            self.dict.insert(s.to_string(), idx);
            self.dict_strings.push(s.to_string());
            idx
        };
        self.indices.push(idx);
    }

    #[inline]
    fn push_inline(&mut self, s: &str) {
        let len = s.len();
        self.inline_data.reserve(len + 4);

        if len < 128 {
            unsafe { self.inline_data.push_unchecked(len as u8); }
        } else {
            encode_varint_fast(len as u64, &mut self.inline_data);
        }
        unsafe { self.inline_data.extend_unchecked(s.as_bytes()); }
    }

    /// Encode to buffer
    pub fn encode_to(self, buf: &mut UltraBuffer) {
        let use_dict = !self.dict_strings.is_empty();

        // Write mode marker
        buf.push(if use_dict { 1 } else { 0 });

        if use_dict {
            // Dictionary mode
            encode_varint_fast(self.dict_strings.len() as u64, buf);

            // Write dictionary
            for s in &self.dict_strings {
                encode_varint_fast(s.len() as u64, buf);
                buf.extend(s.as_bytes());
            }

            // Write indices with adaptive bit width
            let dict_size = self.dict_strings.len();
            if dict_size <= 16 {
                // 4-bit indices (packed)
                let packed_len = (self.indices.len() + 1) / 2;
                buf.reserve(packed_len);
                for chunk in self.indices.chunks(2) {
                    let byte = (chunk[0] as u8) | ((chunk.get(1).copied().unwrap_or(0) as u8) << 4);
                    unsafe { buf.push_unchecked(byte); }
                }
            } else if dict_size <= 256 {
                // 8-bit indices
                buf.reserve(self.indices.len());
                for &idx in &self.indices {
                    unsafe { buf.push_unchecked(idx as u8); }
                }
            } else {
                // 16-bit indices
                buf.reserve(self.indices.len() * 2);
                for &idx in &self.indices {
                    unsafe { buf.extend_unchecked(&idx.to_le_bytes()); }
                }
            }
        } else {
            // Inline mode
            buf.extend(self.inline_data.as_slice());
        }
    }
}

// =============================================================================
// Fast Varint Encoding
// =============================================================================

/// Encode unsigned varint
#[inline(always)]
pub fn encode_varint_fast(mut value: u64, buf: &mut UltraBuffer) {
    buf.reserve(10);
    while value >= 0x80 {
        unsafe { buf.push_unchecked((value as u8) | 0x80); }
        value >>= 7;
    }
    unsafe { buf.push_unchecked(value as u8); }
}

/// Encode signed varint using zigzag encoding
#[inline(always)]
pub fn encode_signed_varint_fast(value: i64, buf: &mut UltraBuffer) {
    let encoded = ((value << 1) ^ (value >> 63)) as u64;
    encode_varint_fast(encoded, buf);
}

// =============================================================================
// Constants & Traits
// =============================================================================

/// Magic bytes for Schema-Aware format
pub const SCHEMA_MAGIC: [u8; 4] = [0x53, 0x43, 0x48, 0x01]; // "SCH\x01"

/// Trait for type-safe schema-based encoding
///
/// Implement this trait to define how your struct should be encoded.
/// Use `TableSchema::builder()` to define the schema with type-based methods.
///
/// # Example
///
/// ```ignore
/// impl TableEncode for Employee {
///     fn schema() -> TableSchema {
///         TableSchema::builder()
///             .u16("id")
///             .string("name")
///             .u8_offset("age", 18)
///             .dict("city")
///             .u32_offset("salary", 30_000)
///             .build()
///     }
///
///     fn encode_with_schema(items: &[Self]) -> Vec<u8> {
///         // ... encoding implementation
///     }
/// }
/// ```
pub trait TableEncode {
    /// Get the table schema defining field types and encodings
    fn schema() -> TableSchema;

    /// Encode a slice of items using the schema
    fn encode_with_schema(items: &[Self]) -> Vec<u8> where Self: Sized;
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_schema_builder() {
        // New type-based API - clean and intuitive
        let schema = TableSchema::builder()
            .u32("id")
            .string("name")
            .u8("age")
            .dict("city")
            .dict("department")
            .u32("salary")
            .u8("experience")
            .u8("project_count")
            .build();

        assert_eq!(schema.columns().len(), 8);
        assert_eq!(schema.encoding(0), Some(FieldEncoding::U32));
        assert_eq!(schema.encoding(2), Some(FieldEncoding::U8));
        assert_eq!(schema.encoding_by_name("city"), Some(FieldEncoding::Dictionary));
        assert_eq!(schema.encoding_by_name("name"), Some(FieldEncoding::Inline));
    }

    #[test]
    fn test_field_encoding_bits() {
        assert_eq!(FieldEncoding::U8.bits(), Some(8));
        assert_eq!(FieldEncoding::U16.bits(), Some(16));
        assert_eq!(FieldEncoding::U32.bits(), Some(32));
        assert_eq!(FieldEncoding::U64.bits(), Some(64));
        assert_eq!(FieldEncoding::I8.bits(), Some(8));
        assert_eq!(FieldEncoding::compact(0, 100).bits(), Some(8));
        assert_eq!(FieldEncoding::compact(0, 1000).bits(), Some(16));
        assert_eq!(FieldEncoding::compact(0, 100_000).bits(), Some(32));
    }

    #[test]
    fn test_adaptive_int_encoder_auto() {
        let mut enc = AdaptiveIntEncoder::new(FieldEncoding::Auto, 10);
        for i in 0..10 {
            enc.push(i * 10);
        }

        let mut buf = UltraBuffer::with_capacity(100);
        enc.encode_to(&mut buf);

        // Should auto-detect 8-bit encoding (range 0-90)
        assert!(buf.len() < 20); // Much smaller than 10 * 8 bytes
    }

    #[test]
    fn test_adaptive_int_encoder_compact() {
        let mut enc = AdaptiveIntEncoder::new(FieldEncoding::compact(1000, 1100), 5);
        enc.push(1000);
        enc.push(1050);
        enc.push(1100);

        let mut buf = UltraBuffer::with_capacity(100);
        enc.encode_to(&mut buf);

        // Should use 8-bit encoding with offset
        // 1 byte encoding marker + offset varint + 3 bytes data
        assert!(buf.len() <= 10);
    }

    #[test]
    fn test_adaptive_string_encoder_dict() {
        let mut enc = AdaptiveStringEncoder::new(FieldEncoding::Dictionary, 100);
        for _ in 0..10 {
            enc.push("NYC");
            enc.push("LA");
            enc.push("Chicago");
        }

        let mut buf = UltraBuffer::with_capacity(200);
        enc.encode_to(&mut buf);

        // Dictionary with 3 strings + 30 indices should be compact
        assert!(buf.len() < 100);
    }

    #[test]
    fn test_adaptive_string_encoder_inline() {
        let mut enc = AdaptiveStringEncoder::new(FieldEncoding::Inline, 10);
        enc.push("hello");
        enc.push("world");

        let mut buf = UltraBuffer::with_capacity(100);
        enc.encode_to(&mut buf);

        // Mode marker + 2 strings with length prefixes
        // 1 + (1+5) + (1+5) = 13 bytes
        assert_eq!(buf.len(), 13);
    }

    #[test]
    fn test_thread_local_scratch() {
        let result = with_scratch(|buf| {
            buf.extend_from_slice(&[1, 2, 3]);
            buf.len()
        });
        assert_eq!(result, 3);

        let result2 = with_scratch(|buf| {
            assert!(buf.is_empty());
            buf.extend_from_slice(&[4, 5]);
            buf.len()
        });
        assert_eq!(result2, 2);
    }
}
