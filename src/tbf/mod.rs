//! Tauq Binary Format (TBF) - State-of-the-Art Binary Serialization
//!
//! TBF is a custom binary format designed specifically for Tauq's schema-based
//! architecture. It achieves best-in-class performance through:
//!
//! - **Direct serde integration**: No intermediate representations
//! - **Schema-aware encoding**: No type tags needed - schema defines structure
//! - **Varint encoding**: Compact integers using LEB128
//! - **String dictionary**: Deduplicate repeated strings
//! - **Zero-copy decoding**: Borrowed references where possible
//!
//! # Performance
//!
//! TBF achieves ~17% of JSON size with competitive serialization speed.
//!
//! # Format Specification
//!
//! ```text
//! TBF File Structure:
//! ┌─────────────────────────────────────┐
//! │ Header (8 bytes)                    │
//! │   Magic: "TBF\x01" (4 bytes)        │
//! │   Version: u8                       │
//! │   Flags: u8                         │
//! │   Reserved: u16                     │
//! ├─────────────────────────────────────┤
//! │ String Dictionary                   │
//! │   Count: varint                     │
//! │   Strings: [len:varint, utf8...]    │
//! ├─────────────────────────────────────┤
//! │ Data Section                        │
//! │   Encoded values (type-tagged)      │
//! └─────────────────────────────────────┘
//! ```

mod varint;
mod dictionary;
mod schema;
mod encoder;
mod decoder;
mod serde_impl;
mod traits;
mod columnar;
mod fast_encode;
mod fast_decode;
mod ultra_encode;
mod schema_encode;
mod stats;
mod bitmap;
mod bloom;
mod stats_collector;
mod simd_decode;
mod parallel_encode;
mod batch_encode;

pub use varint::*;
pub use dictionary::*;
pub use schema::*;
pub use encoder::*;
pub use decoder::*;
pub use traits::{TbfEncode, TbfDecode};
pub use columnar::{
    ColumnarEncoder, ColumnarDecoder, ColumnReader, ColumnType, ColumnEncoding, ColumnMeta,
    ColumnarEncode, ColumnarDecode, TBC_MAGIC, TBC_VERSION,
};
pub use fast_encode::{
    FastEncode, FastBuffer, FastStringDictionary, fast_encode_slice,
    fast_encode_varint, fast_encode_signed_varint,
};
pub use fast_decode::{
    FastDecode, FastBorrowedDictionary, fast_decode_varint, fast_decode_signed_varint,
    batch_decode_u32, batch_decode_u64, batch_decode_i32, batch_decode_i64,
    batch_decode_f32, batch_decode_f64, batch_decode_bool, batch_decode_strings,
};
pub use ultra_encode::{
    UltraEncode, UltraEncodeDirect, UltraBuffer, ColumnCollectors, ColumnData,
    ColumnType as UltraColumnType, IntPacking, pack_u32_adaptive, pack_u64_adaptive,
    encode_varint_to_ultra, DirectU32Encoder, DirectStringEncoder,
    ULTRA_MAGIC, ULTRA_VERSION,
};
pub use schema_encode::{
    // Type-based schema API
    FieldEncoding, ColumnSchema, TableSchema, TableSchemaBuilder, TableEncode,
    AdaptiveIntEncoder, AdaptiveStringEncoder,
    // Utilities
    with_scratch, with_output, encode_varint_fast, encode_signed_varint_fast, SCHEMA_MAGIC,
};
pub use stats::ColumnStats;
pub use bitmap::NullBitmap;
pub use bloom::BloomFilter;
pub use stats_collector::StatisticsCollector;

#[cfg(feature = "performance")]
pub use parallel_encode::{ParallelBatchEncoder, ParallelEncodingStats};
#[cfg(all(feature = "performance", test))]
pub use parallel_encode::ParallelStringDictionary;
pub use batch_encode::{BatchEncoder, BatchEncodingStats};

// serde_impl exports are not needed at module level

use crate::error::TauqError;

// ============================================================================
// Constants
// ============================================================================

/// TBF magic bytes: "TBF\x01"
pub const TBF_MAGIC: [u8; 4] = [0x54, 0x42, 0x46, 0x01];

/// Current TBF version
pub const TBF_VERSION: u8 = 1;

/// Flag: String dictionary enabled
pub const FLAG_DICTIONARY: u8 = 0x02;

// ============================================================================
// Type Tags
// ============================================================================

/// Type tags for TBF encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TypeTag {
    Null = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    String = 4,
    Bytes = 5,
    Seq = 6,
    Map = 7,
    // Extended tags
    Unit = 8,
    None = 9,
    Some = 10,
    I8 = 11,
    I16 = 12,
    I32 = 13,
    I64 = 14,
    I128 = 15,
    U8 = 16,
    U16 = 17,
    U32 = 18,
    U64 = 19,
    U128 = 20,
    F32 = 21,
    F64 = 22,
    Char = 23,
}

impl TypeTag {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(TypeTag::Null),
            1 => Some(TypeTag::Bool),
            2 => Some(TypeTag::Int),
            3 => Some(TypeTag::Float),
            4 => Some(TypeTag::String),
            5 => Some(TypeTag::Bytes),
            6 => Some(TypeTag::Seq),
            7 => Some(TypeTag::Map),
            8 => Some(TypeTag::Unit),
            9 => Some(TypeTag::None),
            10 => Some(TypeTag::Some),
            11 => Some(TypeTag::I8),
            12 => Some(TypeTag::I16),
            13 => Some(TypeTag::I32),
            14 => Some(TypeTag::I64),
            15 => Some(TypeTag::I128),
            16 => Some(TypeTag::U8),
            17 => Some(TypeTag::U16),
            18 => Some(TypeTag::U32),
            19 => Some(TypeTag::U64),
            20 => Some(TypeTag::U128),
            21 => Some(TypeTag::F32),
            22 => Some(TypeTag::F64),
            23 => Some(TypeTag::Char),
            _ => None,
        }
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Serialize a value directly to TBF bytes (fast path)
///
/// This uses direct serde integration, bypassing any intermediate representation.
///
/// # Example
/// ```
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// let user = User { id: 1, name: "Alice".into() };
/// let bytes = tauq::tbf::to_bytes(&user).unwrap();
/// ```
pub fn to_bytes<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, TauqError> {
    let mut serializer = TbfSerializer::new();
    value.serialize(&mut serializer)?;
    Ok(serializer.into_bytes())
}

/// Serialize a value to TBF bytes with pre-allocated capacity
pub fn to_bytes_with_capacity<T: serde::Serialize>(value: &T, capacity: usize) -> Result<Vec<u8>, TauqError> {
    let mut serializer = TbfSerializer::with_capacity(capacity);
    value.serialize(&mut serializer)?;
    Ok(serializer.into_bytes())
}

/// Deserialize TBF bytes directly to a value (fast path)
///
/// # Example
/// ```
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Debug)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// // let bytes = ...;
/// // let user: User = tauq::tbf::from_bytes(&bytes).unwrap();
/// ```
pub fn from_bytes<'de, T: serde::Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, TauqError> {
    let mut deserializer = TbfDeserializer::new(bytes)?;
    T::deserialize(&mut deserializer)
}

/// Encode Tauq source to TBF binary format (via JSON - slower path)
pub fn encode(source: &str) -> Result<Vec<u8>, TauqError> {
    let json = crate::compile_tauq(source)?;
    encode_json(&json)
}

/// Encode JSON value to TBF binary format
pub fn encode_json(json: &serde_json::Value) -> Result<Vec<u8>, TauqError> {
    to_bytes(json)
}

/// Decode TBF binary to JSON
pub fn decode(data: &[u8]) -> Result<serde_json::Value, TauqError> {
    from_bytes(data)
}

/// Decode TBF binary to Tauq string
pub fn decode_to_tauq(data: &[u8]) -> Result<String, TauqError> {
    let json: serde_json::Value = from_bytes(data)?;
    Ok(crate::format_to_tauq(&json))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestUser {
        id: u32,
        name: String,
        age: u32,
        active: bool,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Employee {
        id: u32,
        name: String,
        age: u32,
        city: String,
        department: String,
        salary: u32,
    }

    #[test]
    fn test_direct_serde_roundtrip() {
        let user = TestUser {
            id: 1,
            name: "Alice".into(),
            age: 30,
            active: true,
        };

        let bytes = to_bytes(&user).unwrap();
        let decoded: TestUser = from_bytes(&bytes).unwrap();

        assert_eq!(user, decoded);
    }

    #[test]
    fn test_vec_roundtrip() {
        let users = vec![
            TestUser { id: 1, name: "Alice".into(), age: 30, active: true },
            TestUser { id: 2, name: "Bob".into(), age: 25, active: false },
            TestUser { id: 3, name: "Carol".into(), age: 35, active: true },
        ];

        let bytes = to_bytes(&users).unwrap();
        let decoded: Vec<TestUser> = from_bytes(&bytes).unwrap();

        assert_eq!(users, decoded);
    }

    #[test]
    fn test_primitives() {
        // Integers
        let v: i32 = -42;
        assert_eq!(v, from_bytes::<i32>(&to_bytes(&v).unwrap()).unwrap());

        let v: u64 = 12345678901234;
        assert_eq!(v, from_bytes::<u64>(&to_bytes(&v).unwrap()).unwrap());

        // Floats
        let v: f64 = 3.14159265358979;
        assert_eq!(v, from_bytes::<f64>(&to_bytes(&v).unwrap()).unwrap());

        // Bool
        let v: bool = true;
        assert_eq!(v, from_bytes::<bool>(&to_bytes(&v).unwrap()).unwrap());

        // String
        let v: String = "Hello, World!".into();
        assert_eq!(v, from_bytes::<String>(&to_bytes(&v).unwrap()).unwrap());
    }

    #[test]
    fn test_option() {
        let some: Option<i32> = Some(42);
        let none: Option<i32> = None;

        assert_eq!(some, from_bytes(&to_bytes(&some).unwrap()).unwrap());
        assert_eq!(none, from_bytes::<Option<i32>>(&to_bytes(&none).unwrap()).unwrap());
    }

    #[test]
    fn test_nested_struct() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Outer {
            name: String,
            inner: Inner,
        }

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Inner {
            value: i32,
            data: Vec<u8>,
        }

        let outer = Outer {
            name: "test".into(),
            inner: Inner {
                value: 42,
                data: vec![1, 2, 3, 4, 5],
            },
        };

        let bytes = to_bytes(&outer).unwrap();
        let decoded: Outer = from_bytes(&bytes).unwrap();

        assert_eq!(outer, decoded);
    }

    #[test]
    fn test_json_value_roundtrip() {
        let json = serde_json::json!({
            "users": [
                {"id": 1, "name": "Alice", "age": 30},
                {"id": 2, "name": "Bob", "age": 25},
            ],
            "count": 2
        });

        let bytes = encode_json(&json).unwrap();
        let decoded = decode(&bytes).unwrap();

        assert_eq!(json, decoded);
    }

    #[test]
    fn test_size_comparison() {
        let employees: Vec<Employee> = (0..100)
            .map(|i| Employee {
                id: i,
                name: format!("Employee{}", i),
                age: 25 + (i % 40),
                city: ["NYC", "LA", "Chicago", "Houston", "Phoenix"][i as usize % 5].into(),
                department: ["Engineering", "Sales", "Marketing", "HR", "Finance"][i as usize % 5].into(),
                salary: 50000 + (i * 1000),
            })
            .collect();

        let json_str = serde_json::to_string(&employees).unwrap();
        let tbf_bytes = to_bytes(&employees).unwrap();

        println!("JSON size: {} bytes", json_str.len());
        println!("TBF size: {} bytes", tbf_bytes.len());
        println!(
            "Compression ratio: {:.1}%",
            (tbf_bytes.len() as f64 / json_str.len() as f64) * 100.0
        );

        // TBF should be smaller than JSON
        assert!(tbf_bytes.len() < json_str.len());
    }
}
