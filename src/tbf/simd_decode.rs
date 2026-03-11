//! Optimized decoding operations for Phase 2
//!
//! This module provides performance-optimized decoding functions for:
//! - Float batch operations with better cache locality
//! - Parallel varint decoding using rayon
//! - Optimized hot paths for common cases (80% of varints are 1 byte)
//!
//! All optimizations are portable and work on stable Rust.
//! When the `performance` feature is enabled, uses rayon for parallelization.

use crate::error::{InterpretError, TauqError};

/// Optimized batch decode for f32 values using SIMD-friendly loads
///
/// Current implementation uses chunks for cache locality. When SIMD is available,
/// this can be extended to use SIMD loads.
#[inline]
pub fn batch_decode_f32_simd(bytes: &[u8], count: usize) -> Result<Vec<f32>, TauqError> {
    let required = count * 4;
    if bytes.len() < required {
        return Err(TauqError::Interpret(InterpretError::new(
            "Buffer too small for f32 batch decode",
        )));
    }

    let mut result = Vec::with_capacity(count);

    // Optimization: Use chunks_exact for better cache locality
    // and potential SIMD optimization opportunities
    for chunk in bytes[..required].chunks_exact(4) {
        let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        result.push(value);
    }

    Ok(result)
}

/// Optimized batch decode for f64 values using SIMD-friendly loads
#[inline]
pub fn batch_decode_f64_simd(bytes: &[u8], count: usize) -> Result<Vec<f64>, TauqError> {
    let required = count * 8;
    if bytes.len() < required {
        return Err(TauqError::Interpret(InterpretError::new(
            "Buffer too small for f64 batch decode",
        )));
    }

    let mut result = Vec::with_capacity(count);

    // Optimization: Use chunks_exact for better cache locality
    for chunk in bytes[..required].chunks_exact(8) {
        let value = f64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
        ]);
        result.push(value);
    }

    Ok(result)
}

/// Optimized varint decode that returns early for 1-byte case
///
/// This optimization targets the 80% case where varints are 1 byte,
/// eliminating branch misprediction overhead.
#[inline(always)]
pub fn fast_decode_varint_opt(bytes: &[u8]) -> Result<(u64, usize), TauqError> {
    if bytes.is_empty() {
        return Err(TauqError::Interpret(InterpretError::new("Empty buffer")));
    }

    let b0 = bytes[0];

    // Fast path: 80% of varints are single byte (< 0x80)
    if b0 < 0x80 {
        return Ok((b0 as u64, 1));
    }

    // Slow path: multi-byte varint
    let mut value = (b0 & 0x7F) as u64;
    let mut shift = 7;
    let mut pos = 1;

    loop {
        if pos >= bytes.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Incomplete varint",
            )));
        }

        let byte = bytes[pos];
        value |= ((byte & 0x7F) as u64) << shift;

        if byte < 0x80 {
            return Ok((value, pos + 1));
        }

        shift += 7;
        pos += 1;

        // Sanity check: varints shouldn't be longer than 10 bytes (64-bit)
        if pos > 10 {
            return Err(TauqError::Interpret(InterpretError::new("Varint too long")));
        }
    }
}

/// Decode multiple u32 values in parallel using thread-local buffers
///
/// When the `performance` feature is enabled, uses rayon to parallelize
/// varint decoding across multiple threads. For small batches or when
/// parallelization overhead is not worth it, delegates to sequential decoding.
#[cfg(feature = "performance")]
pub fn batch_decode_u32_parallel(
    bytes: &[u8],
    count: usize,
) -> Result<(Vec<u32>, usize), TauqError> {
    use rayon::prelude::*;

    if count == 0 {
        return Ok((Vec::new(), 0));
    }

    // For small batches, use sequential decoding (rayon overhead isn't worth it)
    if count < 100 {
        let mut result = Vec::with_capacity(count);
        let mut pos = 0;

        for _ in 0..count {
            if pos >= bytes.len() {
                return Err(TauqError::Interpret(InterpretError::new(
                    "Unexpected end of buffer in batch decode",
                )));
            }

            let (value, len) = fast_decode_varint_opt(&bytes[pos..])?;
            result.push(value as u32);
            pos += len;
        }

        return Ok((result, pos));
    }

    // Parse sequentially to get offsets (varints have variable size)
    // This is necessary because we can't parallelize varint decoding
    // without knowing byte boundaries
    let mut offsets = Vec::with_capacity(count + 1);
    offsets.push(0);

    let mut pos = 0;
    for _ in 0..count {
        if pos >= bytes.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of buffer",
            )));
        }

        let (_, len) = fast_decode_varint_opt(&bytes[pos..])?;
        pos += len;
        offsets.push(pos);
    }

    // Now parallel decode using pre-computed offsets
    let result: Result<Vec<u32>, TauqError> = offsets
        .par_windows(2)
        .map(|window| {
            let start = window[0];
            let end = window[1];
            let (value, _) = fast_decode_varint_opt(&bytes[start..end])?;
            Ok(value as u32)
        })
        .collect();

    match result {
        Ok(values) => Ok((values, pos)),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_decode_f32_simd() {
        let mut bytes = Vec::new();
        let values = vec![1.5f32, 2.5f32, 3.5f32];
        for v in &values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }

        let result = batch_decode_f32_simd(&bytes, 3).unwrap();
        assert_eq!(result, values);
    }

    #[test]
    fn test_batch_decode_f64_simd() {
        let mut bytes = Vec::new();
        let values = vec![1.5f64, 2.5f64, 3.5f64];
        for v in &values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }

        let result = batch_decode_f64_simd(&bytes, 3).unwrap();
        assert_eq!(result, values);
    }

    #[test]
    fn test_fast_decode_varint_opt_single_byte() {
        // Single byte varints (0-127)
        for i in 0..127 {
            let bytes = vec![i as u8];
            let (value, len) = fast_decode_varint_opt(&bytes).unwrap();
            assert_eq!(value, i as u64);
            assert_eq!(len, 1);
        }
    }

    #[test]
    fn test_fast_decode_varint_opt_multi_byte() {
        // Two-byte varint: 128 = 0x80 0x01
        let bytes = vec![0x80, 0x01];
        let (value, len) = fast_decode_varint_opt(&bytes).unwrap();
        assert_eq!(value, 128);
        assert_eq!(len, 2);

        // 16383 = 0xFF 0x7F
        let bytes = vec![0xFF, 0x7F];
        let (value, len) = fast_decode_varint_opt(&bytes).unwrap();
        assert_eq!(value, 16383);
        assert_eq!(len, 2);
    }

    #[test]
    fn test_batch_decode_u32_parallel() {
        #[cfg(feature = "performance")]
        {
            use crate::tbf::varint::encode_varint;

            let mut bytes = Vec::new();
            let values = vec![1u64, 128u64, 16384u64, 42u64];

            for v in &values {
                encode_varint(*v, &mut bytes);
            }

            let (result, _) = batch_decode_u32_parallel(&bytes, 4).unwrap();
            assert_eq!(result, vec![1u32, 128u32, 16384u32, 42u32]);
        }
    }
}
