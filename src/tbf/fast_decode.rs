//! High-performance TBF decoding
//!
//! Optimized for maximum deserialization throughput with:
//! - Fast varint decoding with special cases for 1-2 byte values
//! - Batch decode operations for arrays
//! - Upfront validation enabling unsafe access in hot paths
//! - SIMD-friendly memory access patterns

use crate::error::{InterpretError, TauqError};

// =============================================================================
// Fast Varint Decoding
// =============================================================================

/// Fast varint decode with special cases for common small values
///
/// ~80% of varints are 1-2 bytes, so we optimize heavily for those cases.
/// Returns (value, bytes_consumed).
#[inline(always)]
pub fn fast_decode_varint(bytes: &[u8]) -> Result<(u64, usize), TauqError> {
    if bytes.is_empty() {
        return Err(TauqError::Interpret(InterpretError::new(
            "Empty buffer for varint",
        )));
    }

    let b0 = bytes[0];

    // Fast path: 1-byte value (0-127) - ~60% of cases
    if b0 < 0x80 {
        return Ok((b0 as u64, 1));
    }

    if bytes.len() < 2 {
        return Err(TauqError::Interpret(InterpretError::new(
            "Truncated varint",
        )));
    }

    let b1 = bytes[1];

    // Fast path: 2-byte value (128-16383) - ~30% of cases
    if b1 < 0x80 {
        let value = ((b0 & 0x7F) as u64) | ((b1 as u64) << 7);
        return Ok((value, 2));
    }

    // Slow path: 3+ bytes - ~10% of cases
    decode_varint_slow(bytes)
}

/// Slow path for varints >= 3 bytes
#[cold]
#[inline(never)]
fn decode_varint_slow(bytes: &[u8]) -> Result<(u64, usize), TauqError> {
    let mut result: u64 = 0;
    let mut shift = 0;

    for (pos, &byte) in bytes.iter().enumerate() {
        result |= ((byte & 0x7F) as u64) << shift;

        if byte & 0x80 == 0 {
            return Ok((result, pos + 1));
        }

        shift += 7;
        if shift >= 64 {
            return Err(TauqError::Interpret(InterpretError::new(
                "Varint overflow",
            )));
        }
    }

    Err(TauqError::Interpret(InterpretError::new(
        "Truncated varint",
    )))
}

/// Fast signed varint decode with zigzag decoding
#[inline(always)]
pub fn fast_decode_signed_varint(bytes: &[u8]) -> Result<(i64, usize), TauqError> {
    let (encoded, len) = fast_decode_varint(bytes)?;
    // Zigzag decode
    let value = ((encoded >> 1) as i64) ^ (-((encoded & 1) as i64));
    Ok((value, len))
}

// =============================================================================
// Unsafe Fast Decode (after validation)
// =============================================================================

/// Ultra-fast varint decode - ONLY call after validating buffer length
///
/// # Safety
/// Caller must ensure bytes.len() >= 10 (max varint size) OR
/// that the varint is complete within the slice.
#[inline(always)]
pub unsafe fn fast_decode_varint_unchecked(bytes: &[u8]) -> (u64, usize) {
    // SAFETY: Caller guarantees bytes has sufficient length
    let b0 = unsafe { *bytes.get_unchecked(0) };

    if b0 < 0x80 {
        return (b0 as u64, 1);
    }

    // SAFETY: Caller guarantees bytes has sufficient length
    let b1 = unsafe { *bytes.get_unchecked(1) };

    if b1 < 0x80 {
        let value = ((b0 & 0x7F) as u64) | ((b1 as u64) << 7);
        return (value, 2);
    }

    // 3+ bytes - rare, use safe slow path
    // This is safe because we only reach here if bytes has at least 2 elements
    decode_varint_slow(bytes).unwrap_or((0, 0))
}

// =============================================================================
// Batch Decode Operations
// =============================================================================

/// Batch decode u32 varints from a buffer
///
/// Pre-allocates output vector and decodes all values in a tight loop.
/// Returns the decoded values and bytes consumed.
#[inline]
pub fn batch_decode_u32(bytes: &[u8], count: usize) -> Result<(Vec<u32>, usize), TauqError> {
    let mut result = Vec::with_capacity(count);
    let mut pos = 0;

    for _ in 0..count {
        if pos >= bytes.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of buffer in batch decode",
            )));
        }

        let (value, len) = fast_decode_varint(&bytes[pos..])?;
        result.push(value as u32);
        pos += len;
    }

    Ok((result, pos))
}

/// Batch decode u64 varints from a buffer
#[inline]
pub fn batch_decode_u64(bytes: &[u8], count: usize) -> Result<(Vec<u64>, usize), TauqError> {
    let mut result = Vec::with_capacity(count);
    let mut pos = 0;

    for _ in 0..count {
        if pos >= bytes.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of buffer in batch decode",
            )));
        }

        let (value, len) = fast_decode_varint(&bytes[pos..])?;
        result.push(value);
        pos += len;
    }

    Ok((result, pos))
}

/// Batch decode i32 varints (zigzag encoded)
#[inline]
pub fn batch_decode_i32(bytes: &[u8], count: usize) -> Result<(Vec<i32>, usize), TauqError> {
    let mut result = Vec::with_capacity(count);
    let mut pos = 0;

    for _ in 0..count {
        if pos >= bytes.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of buffer in batch decode",
            )));
        }

        let (value, len) = fast_decode_signed_varint(&bytes[pos..])?;
        result.push(value as i32);
        pos += len;
    }

    Ok((result, pos))
}

/// Batch decode i64 varints (zigzag encoded)
#[inline]
pub fn batch_decode_i64(bytes: &[u8], count: usize) -> Result<(Vec<i64>, usize), TauqError> {
    let mut result = Vec::with_capacity(count);
    let mut pos = 0;

    for _ in 0..count {
        if pos >= bytes.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of buffer in batch decode",
            )));
        }

        let (value, len) = fast_decode_signed_varint(&bytes[pos..])?;
        result.push(value);
        pos += len;
    }

    Ok((result, pos))
}

/// Batch decode f32 values (fixed 4 bytes each)
#[inline]
pub fn batch_decode_f32(bytes: &[u8], count: usize) -> Result<Vec<f32>, TauqError> {
    let required = count * 4;
    if bytes.len() < required {
        return Err(TauqError::Interpret(InterpretError::new(
            "Buffer too small for f32 batch decode",
        )));
    }

    let mut result = Vec::with_capacity(count);

    // SIMD-friendly: process 4 bytes at a time
    for i in 0..count {
        let offset = i * 4;
        let value = f32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        result.push(value);
    }

    Ok(result)
}

/// Batch decode f64 values (fixed 8 bytes each)
#[inline]
pub fn batch_decode_f64(bytes: &[u8], count: usize) -> Result<Vec<f64>, TauqError> {
    let required = count * 8;
    if bytes.len() < required {
        return Err(TauqError::Interpret(InterpretError::new(
            "Buffer too small for f64 batch decode",
        )));
    }

    let mut result = Vec::with_capacity(count);

    for i in 0..count {
        let offset = i * 8;
        let value = f64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        result.push(value);
    }

    Ok(result)
}

/// Batch decode bool values (1 byte each)
#[inline]
pub fn batch_decode_bool(bytes: &[u8], count: usize) -> Result<Vec<bool>, TauqError> {
    if bytes.len() < count {
        return Err(TauqError::Interpret(InterpretError::new(
            "Buffer too small for bool batch decode",
        )));
    }

    Ok(bytes[..count].iter().map(|&b| b != 0).collect())
}

// =============================================================================
// Fast String Dictionary
// =============================================================================

/// Fast borrowed string dictionary for decoding
///
/// Pre-resolves all string offsets during construction for O(1) access.
pub struct FastBorrowedDictionary<'a> {
    /// Pre-resolved string slices
    strings: Vec<&'a str>,
}

impl<'a> FastBorrowedDictionary<'a> {
    /// Decode dictionary from bytes, pre-resolving all string offsets
    pub fn decode(bytes: &'a [u8]) -> Result<(Self, usize), TauqError> {
        let (count, mut pos) = fast_decode_varint(bytes)?;
        let count = count as usize;

        let mut strings = Vec::with_capacity(count);

        for _ in 0..count {
            let (len, len_bytes) = fast_decode_varint(&bytes[pos..])?;
            pos += len_bytes;

            let len = len as usize;
            if pos + len > bytes.len() {
                return Err(TauqError::Interpret(InterpretError::new(
                    "String extends past buffer",
                )));
            }

            let s = std::str::from_utf8(&bytes[pos..pos + len]).map_err(|_| {
                TauqError::Interpret(InterpretError::new("Invalid UTF-8 in dictionary"))
            })?;

            strings.push(s);
            pos += len;
        }

        Ok((Self { strings }, pos))
    }

    /// Get string by index - O(1)
    #[inline(always)]
    pub fn get(&self, idx: u32) -> Option<&'a str> {
        self.strings.get(idx as usize).copied()
    }

    /// Get string by index without bounds check
    ///
    /// # Safety
    /// idx must be < self.len()
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, idx: u32) -> &'a str {
        // SAFETY: Caller guarantees idx < self.len()
        unsafe { self.strings.get_unchecked(idx as usize) }
    }

    /// Number of strings
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

/// Batch decode string indices and resolve to string slices
#[inline]
pub fn batch_decode_strings<'a>(
    bytes: &[u8],
    count: usize,
    dict: &FastBorrowedDictionary<'a>,
) -> Result<(Vec<&'a str>, usize), TauqError> {
    let mut result = Vec::with_capacity(count);
    let mut pos = 0;

    for _ in 0..count {
        let (idx, len) = fast_decode_varint(&bytes[pos..])?;
        pos += len;

        let s = dict.get(idx as u32).ok_or_else(|| {
            TauqError::Interpret(InterpretError::new(format!(
                "Invalid string index: {}",
                idx
            )))
        })?;
        result.push(s);
    }

    Ok((result, pos))
}

// =============================================================================
// FastDecode Trait
// =============================================================================

/// Trait for fast schema-aware decoding without type tags
///
/// Similar to `FastEncode`, this provides a fast path for decoding
/// when the schema is known at compile time.
pub trait FastDecode: Sized {
    /// Decode a single value from bytes
    fn fast_decode_from(bytes: &[u8], dict: &FastBorrowedDictionary) -> Result<(Self, usize), TauqError>;

    /// Decode a slice of values (batch optimized)
    fn fast_decode_slice(bytes: &[u8]) -> Result<Vec<Self>, TauqError> {
        use super::{TBF_MAGIC, TBF_VERSION};

        // Verify header
        if bytes.len() < 8 {
            return Err(TauqError::Interpret(InterpretError::new(
                "Buffer too small for TBF header",
            )));
        }

        if bytes[0..4] != TBF_MAGIC {
            return Err(TauqError::Interpret(InterpretError::new(
                "Invalid TBF magic",
            )));
        }

        if bytes[4] > TBF_VERSION {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unsupported TBF version",
            )));
        }

        let mut pos = 8;

        // Decode dictionary
        let (dict, dict_len) = FastBorrowedDictionary::decode(&bytes[pos..])?;
        pos += dict_len;

        // Decode item count
        let (count, len) = fast_decode_varint(&bytes[pos..])?;
        pos += len;
        let count = count as usize;

        // Decode items
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            let (item, len) = Self::fast_decode_from(&bytes[pos..], &dict)?;
            result.push(item);
            pos += len;
        }

        Ok(result)
    }
}

// =============================================================================
// Primitive Implementations
// =============================================================================

impl FastDecode for u32 {
    #[inline(always)]
    fn fast_decode_from(bytes: &[u8], _dict: &FastBorrowedDictionary) -> Result<(Self, usize), TauqError> {
        let (value, len) = fast_decode_varint(bytes)?;
        Ok((value as u32, len))
    }
}

impl FastDecode for u64 {
    #[inline(always)]
    fn fast_decode_from(bytes: &[u8], _dict: &FastBorrowedDictionary) -> Result<(Self, usize), TauqError> {
        fast_decode_varint(bytes)
    }
}

impl FastDecode for i32 {
    #[inline(always)]
    fn fast_decode_from(bytes: &[u8], _dict: &FastBorrowedDictionary) -> Result<(Self, usize), TauqError> {
        let (value, len) = fast_decode_signed_varint(bytes)?;
        Ok((value as i32, len))
    }
}

impl FastDecode for i64 {
    #[inline(always)]
    fn fast_decode_from(bytes: &[u8], _dict: &FastBorrowedDictionary) -> Result<(Self, usize), TauqError> {
        fast_decode_signed_varint(bytes)
    }
}

impl FastDecode for f32 {
    #[inline(always)]
    fn fast_decode_from(bytes: &[u8], _dict: &FastBorrowedDictionary) -> Result<(Self, usize), TauqError> {
        if bytes.len() < 4 {
            return Err(TauqError::Interpret(InterpretError::new(
                "Buffer too small for f32",
            )));
        }
        let value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        Ok((value, 4))
    }
}

impl FastDecode for f64 {
    #[inline(always)]
    fn fast_decode_from(bytes: &[u8], _dict: &FastBorrowedDictionary) -> Result<(Self, usize), TauqError> {
        if bytes.len() < 8 {
            return Err(TauqError::Interpret(InterpretError::new(
                "Buffer too small for f64",
            )));
        }
        let value = f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        Ok((value, 8))
    }
}

impl FastDecode for bool {
    #[inline(always)]
    fn fast_decode_from(bytes: &[u8], _dict: &FastBorrowedDictionary) -> Result<(Self, usize), TauqError> {
        if bytes.is_empty() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Buffer too small for bool",
            )));
        }
        Ok((bytes[0] != 0, 1))
    }
}

impl FastDecode for String {
    #[inline(always)]
    fn fast_decode_from(bytes: &[u8], dict: &FastBorrowedDictionary) -> Result<(Self, usize), TauqError> {
        let (idx, len) = fast_decode_varint(bytes)?;
        let s = dict.get(idx as u32).ok_or_else(|| {
            TauqError::Interpret(InterpretError::new(format!(
                "Invalid string index: {}",
                idx
            )))
        })?;
        Ok((s.to_string(), len))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_decode_varint() {
        // 1-byte values
        assert_eq!(fast_decode_varint(&[0]).unwrap(), (0, 1));
        assert_eq!(fast_decode_varint(&[1]).unwrap(), (1, 1));
        assert_eq!(fast_decode_varint(&[127]).unwrap(), (127, 1));

        // 2-byte values
        assert_eq!(fast_decode_varint(&[0x80, 0x01]).unwrap(), (128, 2));
        assert_eq!(fast_decode_varint(&[0xFF, 0x7F]).unwrap(), (16383, 2));

        // 3-byte values
        assert_eq!(fast_decode_varint(&[0x80, 0x80, 0x01]).unwrap(), (16384, 3));
    }

    #[test]
    fn test_fast_decode_signed_varint() {
        // Positive values
        assert_eq!(fast_decode_signed_varint(&[0]).unwrap(), (0, 1));
        assert_eq!(fast_decode_signed_varint(&[2]).unwrap(), (1, 1));
        assert_eq!(fast_decode_signed_varint(&[4]).unwrap(), (2, 1));

        // Negative values (zigzag encoded)
        assert_eq!(fast_decode_signed_varint(&[1]).unwrap(), (-1, 1));
        assert_eq!(fast_decode_signed_varint(&[3]).unwrap(), (-2, 1));
    }

    #[test]
    fn test_batch_decode_u32() {
        // Create test data: [1, 128, 16384]
        let data = vec![1, 0x80, 0x01, 0x80, 0x80, 0x01];
        let (values, consumed) = batch_decode_u32(&data, 3).unwrap();

        assert_eq!(values, vec![1, 128, 16384]);
        assert_eq!(consumed, 6);
    }

    #[test]
    fn test_batch_decode_f32() {
        let pi = std::f32::consts::PI;
        let e = std::f32::consts::E;

        let mut data = Vec::new();
        data.extend_from_slice(&pi.to_le_bytes());
        data.extend_from_slice(&e.to_le_bytes());

        let values = batch_decode_f32(&data, 2).unwrap();

        assert_eq!(values[0], pi);
        assert_eq!(values[1], e);
    }

    #[test]
    fn test_fast_borrowed_dictionary() {
        // Create dictionary bytes: count=2, "hello", "world"
        let mut data = Vec::new();
        data.push(2); // count
        data.push(5); // "hello" length
        data.extend_from_slice(b"hello");
        data.push(5); // "world" length
        data.extend_from_slice(b"world");

        let (dict, consumed) = FastBorrowedDictionary::decode(&data).unwrap();

        assert_eq!(dict.len(), 2);
        assert_eq!(dict.get(0), Some("hello"));
        assert_eq!(dict.get(1), Some("world"));
        assert_eq!(dict.get(2), None);
        // count(1) + len(1) + "hello"(5) + len(1) + "world"(5) = 13
        assert_eq!(consumed, 13);
    }
}
