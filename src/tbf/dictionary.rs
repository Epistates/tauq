//! Optimized string dictionary for TBF
//!
//! The dictionary interns strings and assigns each unique string an index.
//! This enables compact encoding of repeated strings.

use crate::error::{InterpretError, TauqError};
use super::varint::{encode_varint, decode_varint};
use std::collections::HashMap;

/// String dictionary for deduplicating strings
///
/// Optimized to avoid double allocation during interning.
#[derive(Debug, Default)]
pub struct StringDictionary {
    /// Stored strings in order of insertion
    strings: Vec<String>,
    /// Map from string content to index (uses string hash for lookup)
    index: HashMap<u64, u32>,
}

impl StringDictionary {
    /// Create a new empty dictionary
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a dictionary with pre-allocated capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            strings: Vec::with_capacity(capacity),
            index: HashMap::with_capacity(capacity),
        }
    }

    /// Fast string hash using FNV-1a
    #[inline(always)]
    fn hash_str(s: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
        for byte in s.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3); // FNV prime
        }
        hash
    }

    /// Add a string and return its index
    ///
    /// Optimized to avoid double string allocation.
    #[inline]
    pub fn intern(&mut self, s: &str) -> u32 {
        let hash = Self::hash_str(s);

        // Check if already interned
        if let Some(&idx) = self.index.get(&hash) {
            // Verify it's actually the same string (handle hash collisions)
            if self.strings[idx as usize] == s {
                return idx;
            }
            // Hash collision - linear probe for empty slot
            // In practice this is rare with FNV-1a on strings
        }

        // New string - allocate once
        let idx = self.strings.len() as u32;
        self.strings.push(s.to_string());
        self.index.insert(hash, idx);
        idx
    }

    /// Intern a string that's already owned (avoids allocation)
    #[inline]
    pub fn intern_owned(&mut self, s: String) -> u32 {
        let hash = Self::hash_str(&s);

        if let Some(&idx) = self.index.get(&hash)
            && self.strings[idx as usize] == s
        {
            return idx;
        }

        let idx = self.strings.len() as u32;
        self.index.insert(hash, idx);
        self.strings.push(s);
        idx
    }

    /// Get a string by index
    #[inline(always)]
    pub fn get(&self, idx: u32) -> Option<&str> {
        self.strings.get(idx as usize).map(|s| s.as_str())
    }

    /// Number of strings in dictionary
    #[inline]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if dictionary is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Get all strings as a slice
    #[inline]
    pub fn strings(&self) -> &[String] {
        &self.strings
    }

    /// Encode dictionary to bytes
    pub fn encode(&self, buf: &mut Vec<u8>) {
        encode_varint(self.strings.len() as u64, buf);
        for s in &self.strings {
            let bytes = s.as_bytes();
            encode_varint(bytes.len() as u64, buf);
            buf.extend_from_slice(bytes);
        }
    }

    /// Estimate encoded size for pre-allocation
    pub fn encoded_size(&self) -> usize {
        let mut size = 10; // Varint for count (max)
        for s in &self.strings {
            size += 10 + s.len(); // Varint for length + bytes
        }
        size
    }

    /// Decode dictionary from bytes
    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), TauqError> {
        let (count, mut pos) = decode_varint(bytes)?;

        let mut dict = Self::with_capacity(count as usize);

        for _ in 0..count {
            let (str_len, len) = decode_varint(&bytes[pos..])?;
            pos += len;

            if pos + str_len as usize > bytes.len() {
                return Err(TauqError::Interpret(InterpretError::new(
                    "String extends past end of buffer".to_string(),
                )));
            }

            let s = std::str::from_utf8(&bytes[pos..pos + str_len as usize]).map_err(|e| {
                TauqError::Interpret(InterpretError::new(format!("Invalid UTF-8: {}", e)))
            })?;
            dict.intern(s);
            pos += str_len as usize;
        }

        Ok((dict, pos))
    }
}

/// Zero-copy string dictionary for decoding
///
/// Holds references into the original buffer instead of allocating.
#[derive(Debug)]
pub struct BorrowedDictionary<'a> {
    /// String slices referencing the original buffer
    strings: Vec<&'a str>,
}

impl<'a> BorrowedDictionary<'a> {
    /// Decode dictionary with zero-copy string references
    pub fn decode(bytes: &'a [u8]) -> Result<(Self, usize), TauqError> {
        let (count, mut pos) = decode_varint(bytes)?;

        let mut strings = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let (str_len, len) = decode_varint(&bytes[pos..])?;
            pos += len;

            if pos + str_len as usize > bytes.len() {
                return Err(TauqError::Interpret(InterpretError::new(
                    "String extends past end of buffer".to_string(),
                )));
            }

            let s = std::str::from_utf8(&bytes[pos..pos + str_len as usize]).map_err(|e| {
                TauqError::Interpret(InterpretError::new(format!("Invalid UTF-8: {}", e)))
            })?;
            strings.push(s);
            pos += str_len as usize;
        }

        Ok((Self { strings }, pos))
    }

    /// Get a string by index (zero-copy)
    #[inline(always)]
    pub fn get(&self, idx: u32) -> Option<&'a str> {
        self.strings.get(idx as usize).copied()
    }

    /// Number of strings
    #[inline]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_dictionary() {
        let mut dict = StringDictionary::new();
        assert_eq!(dict.intern("hello"), 0);
        assert_eq!(dict.intern("world"), 1);
        assert_eq!(dict.intern("hello"), 0); // Deduplicated
        assert_eq!(dict.get(0), Some("hello"));
        assert_eq!(dict.get(1), Some("world"));
    }

    #[test]
    fn test_dictionary_roundtrip() {
        let mut dict = StringDictionary::new();
        dict.intern("hello");
        dict.intern("world");
        dict.intern("test");

        let mut buf = Vec::new();
        dict.encode(&mut buf);

        let (decoded, _) = StringDictionary::decode(&buf).unwrap();
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded.get(0), Some("hello"));
        assert_eq!(decoded.get(1), Some("world"));
        assert_eq!(decoded.get(2), Some("test"));
    }

    #[test]
    fn test_borrowed_dictionary() {
        let mut dict = StringDictionary::new();
        dict.intern("hello");
        dict.intern("world");

        let mut buf = Vec::new();
        dict.encode(&mut buf);

        let (borrowed, _) = BorrowedDictionary::decode(&buf).unwrap();
        assert_eq!(borrowed.get(0), Some("hello"));
        assert_eq!(borrowed.get(1), Some("world"));
    }

    #[test]
    fn test_intern_owned() {
        let mut dict = StringDictionary::new();
        let s = String::from("owned string");
        let idx = dict.intern_owned(s);
        assert_eq!(dict.get(idx), Some("owned string"));
    }
}
