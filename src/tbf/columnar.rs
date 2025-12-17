//! Columnar storage for TBF
//!
//! Stores data column-by-column rather than row-by-row for:
//! - Better compression (similar values are adjacent)
//! - Improved cache locality
//! - SIMD-friendly memory layout
//! - Delta/run-length encoding opportunities
//!
//! # Format
//!
//! ```text
//! Columnar TBF Structure:
//! ┌─────────────────────────────────────┐
//! │ Header (8 bytes)                    │
//! │   Magic: "TBC\x01" (4 bytes)        │  <- TBF Columnar
//! │   Version: u8                       │
//! │   Flags: u8                         │
//! │   Column count: u16                 │
//! ├─────────────────────────────────────┤
//! │ String Dictionary                   │
//! │   Count: varint                     │
//! │   Strings: [len:varint, utf8...]    │
//! ├─────────────────────────────────────┤
//! │ Row count: varint                   │
//! ├─────────────────────────────────────┤
//! │ Column Metadata (per column)        │
//! │   Name index: varint                │
//! │   Type: u8                          │
//! │   Encoding: u8 (raw/delta/rle)      │
//! ├─────────────────────────────────────┤
//! │ Column Data (per column)            │
//! │   [values...]                       │
//! └─────────────────────────────────────┘
//! ```

use super::dictionary::{BorrowedDictionary, StringDictionary};
use super::varint::{decode_signed_varint, decode_varint, encode_signed_varint, encode_varint};
use crate::error::{InterpretError, TauqError};

/// Magic bytes for columnar TBF: "TBC\x01"
pub const TBC_MAGIC: [u8; 4] = [0x54, 0x42, 0x43, 0x01];

/// Current columnar TBF version
pub const TBC_VERSION: u8 = 1;

/// Column encoding strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ColumnEncoding {
    /// Raw values (no special encoding)
    Raw = 0,
    /// Delta encoding (store differences)
    Delta = 1,
    /// Run-length encoding (for repeated values)
    Rle = 2,
    /// Dictionary encoding (for strings)
    Dictionary = 3,
}

impl ColumnEncoding {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(ColumnEncoding::Raw),
            1 => Some(ColumnEncoding::Delta),
            2 => Some(ColumnEncoding::Rle),
            3 => Some(ColumnEncoding::Dictionary),
            _ => None,
        }
    }
}

/// Column data type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ColumnType {
    Bool = 1,
    I8 = 2,
    I16 = 3,
    I32 = 4,
    I64 = 5,
    U8 = 6,
    U16 = 7,
    U32 = 8,
    U64 = 9,
    F32 = 10,
    F64 = 11,
    String = 12,
    Bytes = 13,
}

impl ColumnType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(ColumnType::Bool),
            2 => Some(ColumnType::I8),
            3 => Some(ColumnType::I16),
            4 => Some(ColumnType::I32),
            5 => Some(ColumnType::I64),
            6 => Some(ColumnType::U8),
            7 => Some(ColumnType::U16),
            8 => Some(ColumnType::U32),
            9 => Some(ColumnType::U64),
            10 => Some(ColumnType::F32),
            11 => Some(ColumnType::F64),
            12 => Some(ColumnType::String),
            13 => Some(ColumnType::Bytes),
            _ => None,
        }
    }
}

/// Column metadata
#[derive(Debug, Clone)]
pub struct ColumnMeta {
    /// Column name (dictionary index)
    pub name_idx: u32,
    /// Column type
    pub col_type: ColumnType,
    /// Encoding strategy
    pub encoding: ColumnEncoding,
}

/// Columnar encoder for homogeneous arrays of structs
pub struct ColumnarEncoder {
    /// String dictionary
    dict: StringDictionary,
    /// Column metadata
    columns: Vec<ColumnMeta>,
    /// Column data buffers
    column_data: Vec<Vec<u8>>,
    /// Row count
    row_count: usize,
    /// Whether columns have been initialized
    initialized: bool,
}

impl ColumnarEncoder {
    /// Create a new columnar encoder
    pub fn new() -> Self {
        Self {
            dict: StringDictionary::new(),
            columns: Vec::new(),
            column_data: Vec::new(),
            row_count: 0,
            initialized: false,
        }
    }

    /// Define a column
    pub fn add_column(&mut self, name: &str, col_type: ColumnType) {
        let name_idx = self.dict.intern(name);
        let encoding = match col_type {
            ColumnType::String => ColumnEncoding::Dictionary,
            ColumnType::I32 | ColumnType::I64 | ColumnType::U32 | ColumnType::U64 => {
                ColumnEncoding::Delta
            }
            _ => ColumnEncoding::Raw,
        };
        self.columns.push(ColumnMeta {
            name_idx,
            col_type,
            encoding,
        });
        self.column_data.push(Vec::new());
        self.initialized = true;
    }

    /// Add a bool value to a column
    #[inline]
    pub fn push_bool(&mut self, col_idx: usize, value: bool) {
        self.column_data[col_idx].push(if value { 1 } else { 0 });
    }

    /// Add a u32 value to a column (delta encoded)
    #[inline]
    pub fn push_u32(&mut self, col_idx: usize, value: u32) {
        encode_varint(value as u64, &mut self.column_data[col_idx]);
    }

    /// Add a u64 value to a column (delta encoded)
    #[inline]
    pub fn push_u64(&mut self, col_idx: usize, value: u64) {
        encode_varint(value, &mut self.column_data[col_idx]);
    }

    /// Add an i32 value to a column
    #[inline]
    pub fn push_i32(&mut self, col_idx: usize, value: i32) {
        encode_signed_varint(value as i64, &mut self.column_data[col_idx]);
    }

    /// Add an i64 value to a column
    #[inline]
    pub fn push_i64(&mut self, col_idx: usize, value: i64) {
        encode_signed_varint(value, &mut self.column_data[col_idx]);
    }

    /// Add an f32 value to a column
    #[inline]
    pub fn push_f32(&mut self, col_idx: usize, value: f32) {
        self.column_data[col_idx].extend_from_slice(&value.to_le_bytes());
    }

    /// Add an f64 value to a column
    #[inline]
    pub fn push_f64(&mut self, col_idx: usize, value: f64) {
        self.column_data[col_idx].extend_from_slice(&value.to_le_bytes());
    }

    /// Add a string value to a column (dictionary encoded)
    #[inline]
    pub fn push_string(&mut self, col_idx: usize, value: &str) {
        let idx = self.dict.intern(value);
        encode_varint(idx as u64, &mut self.column_data[col_idx]);
    }

    /// Increment row count
    #[inline]
    pub fn finish_row(&mut self) {
        self.row_count += 1;
    }

    /// Finalize and return encoded bytes
    pub fn finish(self) -> Vec<u8> {
        // Encode dictionary
        let mut dict_buf = Vec::new();
        self.dict.encode(&mut dict_buf);

        // Calculate total size
        let data_size: usize = self.column_data.iter().map(|c| c.len()).sum();
        let meta_size = self.columns.len() * 10; // Approximate
        let total_size = 8 + dict_buf.len() + 10 + meta_size + data_size;

        let mut result = Vec::with_capacity(total_size);

        // Header
        result.extend_from_slice(&TBC_MAGIC);
        result.push(TBC_VERSION);
        result.push(0); // Flags
        result.extend_from_slice(&(self.columns.len() as u16).to_le_bytes());

        // Dictionary
        result.extend_from_slice(&dict_buf);

        // Row count
        encode_varint(self.row_count as u64, &mut result);

        // Column metadata
        for col in &self.columns {
            encode_varint(col.name_idx as u64, &mut result);
            result.push(col.col_type as u8);
            result.push(col.encoding as u8);
        }

        // Column data
        for col_data in &self.column_data {
            encode_varint(col_data.len() as u64, &mut result);
            result.extend_from_slice(col_data);
        }

        result
    }
}

impl Default for ColumnarEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Columnar decoder
pub struct ColumnarDecoder<'a> {
    /// Raw data
    data: &'a [u8],
    /// String dictionary
    dict: BorrowedDictionary<'a>,
    /// Column metadata
    columns: Vec<ColumnMeta>,
    /// Column data slices
    column_data: Vec<&'a [u8]>,
    /// Row count
    row_count: usize,
}

impl<'a> ColumnarDecoder<'a> {
    /// Create a new columnar decoder
    pub fn new(data: &'a [u8]) -> Result<Self, TauqError> {
        // Verify header
        if data.len() < 8 {
            return Err(TauqError::Interpret(InterpretError::new(
                "Buffer too small for columnar TBF header",
            )));
        }

        if data[0..4] != TBC_MAGIC {
            return Err(TauqError::Interpret(InterpretError::new(
                "Invalid columnar TBF magic bytes",
            )));
        }

        if data[4] != TBC_VERSION {
            return Err(TauqError::Interpret(InterpretError::new(format!(
                "Unsupported columnar TBF version: {}",
                data[4]
            ))));
        }

        let column_count = u16::from_le_bytes([data[6], data[7]]) as usize;
        let mut pos = 8;

        // Decode dictionary
        let (dict, dict_len) = BorrowedDictionary::decode(&data[pos..])?;
        pos += dict_len;

        // Decode row count
        let (row_count, len) = decode_varint(&data[pos..])?;
        pos += len;
        let row_count = row_count as usize;

        // Decode column metadata
        let mut columns = Vec::with_capacity(column_count);
        for _ in 0..column_count {
            let (name_idx, len) = decode_varint(&data[pos..])?;
            pos += len;

            let col_type = ColumnType::from_u8(data[pos])
                .ok_or_else(|| TauqError::Interpret(InterpretError::new("Invalid column type")))?;
            pos += 1;

            let encoding = ColumnEncoding::from_u8(data[pos]).ok_or_else(|| {
                TauqError::Interpret(InterpretError::new("Invalid column encoding"))
            })?;
            pos += 1;

            columns.push(ColumnMeta {
                name_idx: name_idx as u32,
                col_type,
                encoding,
            });
        }

        // Decode column data slices
        let mut column_data = Vec::with_capacity(column_count);
        for _ in 0..column_count {
            let (col_len, len) = decode_varint(&data[pos..])?;
            pos += len;

            let col_len = col_len as usize;
            if pos + col_len > data.len() {
                return Err(TauqError::Interpret(InterpretError::new(
                    "Column data extends past buffer",
                )));
            }

            column_data.push(&data[pos..pos + col_len]);
            pos += col_len;
        }

        Ok(Self {
            data,
            dict,
            columns,
            column_data,
            row_count,
        })
    }

    /// Get row count
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Get column count
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Get column name
    pub fn column_name(&self, col_idx: usize) -> Option<&'a str> {
        self.columns
            .get(col_idx)
            .and_then(|c| self.dict.get(c.name_idx))
    }

    /// Get column type
    pub fn column_type(&self, col_idx: usize) -> Option<ColumnType> {
        self.columns.get(col_idx).map(|c| c.col_type)
    }

    /// Create a column reader
    pub fn column_reader(&'a self, col_idx: usize) -> Option<ColumnReader<'a>> {
        if col_idx >= self.columns.len() {
            return None;
        }

        Some(ColumnReader {
            data: self.column_data[col_idx],
            dict: &self.dict,
            col_type: self.columns[col_idx].col_type,
            pos: 0,
        })
    }
}

/// Column reader for iterating over column values
pub struct ColumnReader<'a> {
    data: &'a [u8],
    dict: &'a BorrowedDictionary<'a>,
    col_type: ColumnType,
    pos: usize,
}

impl<'a> ColumnReader<'a> {
    /// Read next bool value
    pub fn next_bool(&mut self) -> Option<bool> {
        if self.pos >= self.data.len() {
            return None;
        }
        let value = self.data[self.pos] != 0;
        self.pos += 1;
        Some(value)
    }

    /// Read next u32 value
    pub fn next_u32(&mut self) -> Option<u32> {
        if self.pos >= self.data.len() {
            return None;
        }
        let (value, len) = decode_varint(&self.data[self.pos..]).ok()?;
        self.pos += len;
        Some(value as u32)
    }

    /// Read next u64 value
    pub fn next_u64(&mut self) -> Option<u64> {
        if self.pos >= self.data.len() {
            return None;
        }
        let (value, len) = decode_varint(&self.data[self.pos..]).ok()?;
        self.pos += len;
        Some(value)
    }

    /// Read next i32 value
    pub fn next_i32(&mut self) -> Option<i32> {
        if self.pos >= self.data.len() {
            return None;
        }
        let (value, len) = decode_signed_varint(&self.data[self.pos..]).ok()?;
        self.pos += len;
        Some(value as i32)
    }

    /// Read next i64 value
    pub fn next_i64(&mut self) -> Option<i64> {
        if self.pos >= self.data.len() {
            return None;
        }
        let (value, len) = decode_signed_varint(&self.data[self.pos..]).ok()?;
        self.pos += len;
        Some(value)
    }

    /// Read next f32 value
    pub fn next_f32(&mut self) -> Option<f32> {
        if self.pos + 4 > self.data.len() {
            return None;
        }
        let bytes: [u8; 4] = self.data[self.pos..self.pos + 4].try_into().ok()?;
        self.pos += 4;
        Some(f32::from_le_bytes(bytes))
    }

    /// Read next f64 value
    pub fn next_f64(&mut self) -> Option<f64> {
        if self.pos + 8 > self.data.len() {
            return None;
        }
        let bytes: [u8; 8] = self.data[self.pos..self.pos + 8].try_into().ok()?;
        self.pos += 8;
        Some(f64::from_le_bytes(bytes))
    }

    /// Read next string value (dictionary index -> string)
    pub fn next_string(&mut self) -> Option<&'a str> {
        if self.pos >= self.data.len() {
            return None;
        }
        let (idx, len) = decode_varint(&self.data[self.pos..]).ok()?;
        self.pos += len;
        self.dict.get(idx as u32)
    }

    /// Reset position to beginning
    pub fn reset(&mut self) {
        self.pos = 0;
    }
}

// =============================================================================
// High-level API for encoding/decoding slices
// =============================================================================

/// Trait for types that can be columnar-encoded
pub trait ColumnarEncode {
    /// Define columns for this type
    fn define_columns(encoder: &mut ColumnarEncoder);

    /// Encode this value's fields to the columns
    fn encode_to_columns(&self, encoder: &mut ColumnarEncoder);

    /// Encode a slice of values in columnar format
    fn columnar_encode_slice(items: &[Self]) -> Vec<u8>
    where
        Self: Sized,
    {
        let mut encoder = ColumnarEncoder::new();

        if !items.is_empty() {
            Self::define_columns(&mut encoder);

            for item in items {
                item.encode_to_columns(&mut encoder);
                encoder.finish_row();
            }
        }

        encoder.finish()
    }
}

/// Trait for types that can be columnar-decoded
pub trait ColumnarDecode: Sized {
    /// Decode this value's fields from column readers
    fn decode_from_columns(readers: &mut [ColumnReader<'_>]) -> Option<Self>;

    /// Decode a slice of values from columnar format
    fn columnar_decode_slice(data: &[u8]) -> Result<Vec<Self>, TauqError> {
        let decoder = ColumnarDecoder::new(data)?;
        let row_count = decoder.row_count();

        let mut readers: Vec<_> = (0..decoder.column_count())
            .filter_map(|i| decoder.column_reader(i))
            .collect();

        let mut items = Vec::with_capacity(row_count);
        for _ in 0..row_count {
            let item = Self::decode_from_columns(&mut readers).ok_or_else(|| {
                TauqError::Interpret(InterpretError::new("Failed to decode row from columns"))
            })?;
            items.push(item);
        }

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestEmployee {
        id: u32,
        name: String,
        age: u32,
        salary: u32,
        active: bool,
    }

    impl ColumnarEncode for TestEmployee {
        fn define_columns(encoder: &mut ColumnarEncoder) {
            encoder.add_column("id", ColumnType::U32);
            encoder.add_column("name", ColumnType::String);
            encoder.add_column("age", ColumnType::U32);
            encoder.add_column("salary", ColumnType::U32);
            encoder.add_column("active", ColumnType::Bool);
        }

        fn encode_to_columns(&self, encoder: &mut ColumnarEncoder) {
            encoder.push_u32(0, self.id);
            encoder.push_string(1, &self.name);
            encoder.push_u32(2, self.age);
            encoder.push_u32(3, self.salary);
            encoder.push_bool(4, self.active);
        }
    }

    impl ColumnarDecode for TestEmployee {
        fn decode_from_columns(readers: &mut [ColumnReader<'_>]) -> Option<Self> {
            Some(TestEmployee {
                id: readers[0].next_u32()?,
                name: readers[1].next_string()?.to_string(),
                age: readers[2].next_u32()?,
                salary: readers[3].next_u32()?,
                active: readers[4].next_bool()?,
            })
        }
    }

    #[test]
    fn test_columnar_roundtrip() {
        let employees = vec![
            TestEmployee {
                id: 1,
                name: "Alice".into(),
                age: 30,
                salary: 75000,
                active: true,
            },
            TestEmployee {
                id: 2,
                name: "Bob".into(),
                age: 25,
                salary: 65000,
                active: true,
            },
            TestEmployee {
                id: 3,
                name: "Carol".into(),
                age: 35,
                salary: 85000,
                active: false,
            },
        ];

        let bytes = TestEmployee::columnar_encode_slice(&employees);
        let decoded = TestEmployee::columnar_decode_slice(&bytes).unwrap();

        assert_eq!(employees, decoded);
    }

    #[test]
    fn test_columnar_size_comparison() {
        // Generate test data
        let employees: Vec<TestEmployee> = (0..1000)
            .map(|i| TestEmployee {
                id: i,
                name: format!("Employee{}", i % 100), // Repeated names for dictionary benefit
                age: 25 + (i % 40),
                salary: 50000 + (i * 100),
                active: i % 3 != 0,
            })
            .collect();

        let columnar_bytes = TestEmployee::columnar_encode_slice(&employees);
        let json_bytes = serde_json::to_string(&serde_json::json!(
            employees.iter().map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "name": e.name,
                    "age": e.age,
                    "salary": e.salary,
                    "active": e.active
                })
            }).collect::<Vec<_>>()
        ))
        .unwrap();

        println!("Columnar: {} bytes", columnar_bytes.len());
        println!("JSON: {} bytes", json_bytes.len());
        println!(
            "Columnar is {:.1}% of JSON",
            (columnar_bytes.len() as f64 / json_bytes.len() as f64) * 100.0
        );

        // Columnar should be much smaller
        assert!(columnar_bytes.len() < json_bytes.len() / 3);
    }
}
