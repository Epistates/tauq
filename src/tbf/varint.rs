//! Varint encoding using LEB128

use crate::error::{InterpretError, TauqError};

/// Encode a u64 as a variable-length integer (LEB128)
///
/// Small values use fewer bytes:
/// - 0-127: 1 byte
/// - 128-16383: 2 bytes
/// - etc.
#[inline(always)]
pub fn encode_varint(mut value: u64, buf: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80; // Set continuation bit
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Encode a u64 to a fixed-size buffer, returning bytes written
#[inline(always)]
pub fn encode_varint_to_slice(mut value: u64, buf: &mut [u8]) -> usize {
    let mut pos = 0;
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf[pos] = byte;
        pos += 1;
        if value == 0 {
            break;
        }
    }
    pos
}

/// Decode a variable-length integer from bytes
///
/// Returns (value, bytes_read)
#[inline(always)]
pub fn decode_varint(bytes: &[u8]) -> Result<(u64, usize), TauqError> {
    let mut result: u64 = 0;
    let mut shift = 0;
    let mut pos = 0;

    loop {
        if pos >= bytes.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of varint".to_string(),
            )));
        }

        let byte = bytes[pos];
        pos += 1;

        result |= ((byte & 0x7F) as u64) << shift;

        if byte & 0x80 == 0 {
            break;
        }

        shift += 7;
        if shift >= 64 {
            return Err(TauqError::Interpret(InterpretError::new(
                "Varint overflow".to_string(),
            )));
        }
    }

    Ok((result, pos))
}

/// Encode a signed integer using zigzag encoding + varint
#[inline(always)]
pub fn encode_signed_varint(value: i64, buf: &mut Vec<u8>) {
    // Zigzag encoding: map negative numbers to positive
    let encoded = ((value << 1) ^ (value >> 63)) as u64;
    encode_varint(encoded, buf);
}

/// Decode a zigzag-encoded signed varint
#[inline(always)]
pub fn decode_signed_varint(bytes: &[u8]) -> Result<(i64, usize), TauqError> {
    let (encoded, len) = decode_varint(bytes)?;
    // Zigzag decode
    let value = ((encoded >> 1) as i64) ^ (-((encoded & 1) as i64));
    Ok((value, len))
}

/// Encode i128 using zigzag + varint (up to 19 bytes)
#[inline]
pub fn encode_i128_varint(value: i128, buf: &mut Vec<u8>) {
    let encoded = ((value << 1) ^ (value >> 127)) as u128;
    encode_u128_varint(encoded, buf);
}

/// Encode u128 using varint (up to 19 bytes)
#[inline]
pub fn encode_u128_varint(mut value: u128, buf: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        buf.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Decode u128 varint
#[inline]
pub fn decode_u128_varint(bytes: &[u8]) -> Result<(u128, usize), TauqError> {
    let mut result: u128 = 0;
    let mut shift = 0;
    let mut pos = 0;

    loop {
        if pos >= bytes.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of varint".to_string(),
            )));
        }

        let byte = bytes[pos];
        pos += 1;

        result |= ((byte & 0x7F) as u128) << shift;

        if byte & 0x80 == 0 {
            break;
        }

        shift += 7;
        if shift >= 128 {
            return Err(TauqError::Interpret(InterpretError::new(
                "Varint overflow".to_string(),
            )));
        }
    }

    Ok((result, pos))
}

/// Decode i128 zigzag varint
#[inline]
pub fn decode_i128_varint(bytes: &[u8]) -> Result<(i128, usize), TauqError> {
    let (encoded, len) = decode_u128_varint(bytes)?;
    let value = ((encoded >> 1) as i128) ^ (-((encoded & 1) as i128));
    Ok((value, len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_roundtrip() {
        for value in [
            0u64,
            1,
            127,
            128,
            255,
            256,
            16383,
            16384,
            u64::MAX / 2,
            u64::MAX,
        ] {
            let mut buf = Vec::new();
            encode_varint(value, &mut buf);
            let (decoded, _) = decode_varint(&buf).unwrap();
            assert_eq!(value, decoded, "Failed for {}", value);
        }
    }

    #[test]
    fn test_signed_varint_roundtrip() {
        for value in [
            0i64,
            1,
            -1,
            127,
            -128,
            i64::MIN / 2,
            i64::MAX / 2,
            i64::MIN,
            i64::MAX,
        ] {
            let mut buf = Vec::new();
            encode_signed_varint(value, &mut buf);
            let (decoded, _) = decode_signed_varint(&buf).unwrap();
            assert_eq!(value, decoded, "Failed for {}", value);
        }
    }

    #[test]
    fn test_varint_sizes() {
        let mut buf = Vec::new();

        // 1-byte values (0-127)
        encode_varint(0, &mut buf);
        assert_eq!(buf.len(), 1);
        buf.clear();

        encode_varint(127, &mut buf);
        assert_eq!(buf.len(), 1);
        buf.clear();

        // 2-byte values (128-16383)
        encode_varint(128, &mut buf);
        assert_eq!(buf.len(), 2);
        buf.clear();

        encode_varint(16383, &mut buf);
        assert_eq!(buf.len(), 2);
        buf.clear();
    }
}
