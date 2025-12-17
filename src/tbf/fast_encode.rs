//! High-performance TBF encoding
//!
//! Optimized for maximum throughput with:
//! - ahash for fast, battle-tested hashing (same as Rust's HashMap)
//! - Batch varint encoding
//! - Pre-allocated buffers
//! - Minimized allocations

use ahash::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};

// =============================================================================
// Fast String Dictionary
// =============================================================================

/// Ultra-fast string dictionary using open addressing
/// - Uses ahash (same as Rust's HashMap) for battle-tested hashing
/// - Direct array lookup with open addressing
/// - Minimal allocations
pub struct FastStringDictionary {
    /// Stored strings
    strings: Vec<String>,
    /// Open-addressed hash table: (hash, index) pairs
    /// Size is always power of 2 for fast modulo
    slots: Vec<(u64, u32)>,
    /// Mask for fast modulo (slots.len() - 1)
    mask: usize,
    /// Hash builder (ahash)
    hash_builder: RandomState,
}

impl FastStringDictionary {
    /// Create with expected capacity
    #[inline]
    pub fn with_capacity(string_count: usize) -> Self {
        // Round up to power of 2, with 2x load factor headroom
        let slot_count = (string_count * 2).next_power_of_two().max(64);
        Self {
            strings: Vec::with_capacity(string_count),
            slots: vec![(0, u32::MAX); slot_count],
            mask: slot_count - 1,
            hash_builder: RandomState::new(),
        }
    }

    /// Hash a string using ahash
    #[inline(always)]
    fn hash_str(&self, s: &str) -> u64 {
        let mut hasher = self.hash_builder.build_hasher();
        s.hash(&mut hasher);
        hasher.finish()
    }

    /// Intern a string - returns index (ultra-fast path)
    #[inline]
    pub fn intern(&mut self, s: &str) -> u32 {
        // Resize if > 70% full to prevent infinite loops
        if self.strings.len() * 10 >= self.slots.len() * 7 {
            self.resize();
        }

        let hash = self.hash_str(s);
        let mut slot_idx = (hash as usize) & self.mask;

        // Linear probe with open addressing
        loop {
            let (slot_hash, slot_value) = self.slots[slot_idx];

            // Empty slot - insert new string
            if slot_value == u32::MAX {
                let idx = self.strings.len() as u32;
                self.strings.push(s.to_string());
                self.slots[slot_idx] = (hash, idx);
                return idx;
            }

            // Hash match - verify string
            if slot_hash == hash {
                // SAFETY: slot_value is a valid index we inserted
                if self.strings[slot_value as usize] == s {
                    return slot_value;
                }
            }

            // Collision - linear probe
            slot_idx = (slot_idx + 1) & self.mask;
        }
    }

    /// Resize the hash table when it gets too full
    fn resize(&mut self) {
        let new_size = self.slots.len() * 2;
        let new_mask = new_size - 1;
        let mut new_slots = vec![(0u64, u32::MAX); new_size];

        // Rehash all existing strings
        for (i, s) in self.strings.iter().enumerate() {
            let hash = self.hash_str(s);
            let mut slot_idx = (hash as usize) & new_mask;

            // Find empty slot
            while new_slots[slot_idx].1 != u32::MAX {
                slot_idx = (slot_idx + 1) & new_mask;
            }
            new_slots[slot_idx] = (hash, i as u32);
        }

        self.slots = new_slots;
        self.mask = new_mask;
    }

    /// Get string by index
    #[inline(always)]
    pub fn get(&self, idx: u32) -> Option<&str> {
        self.strings.get(idx as usize).map(|s| s.as_str())
    }

    /// Encode dictionary to buffer
    pub fn encode(&self, buf: &mut Vec<u8>) {
        fast_encode_varint(self.strings.len() as u64, buf);
        for s in &self.strings {
            let bytes = s.as_bytes();
            fast_encode_varint(bytes.len() as u64, buf);
            buf.extend_from_slice(bytes);
        }
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

// =============================================================================
// Fast Varint Encoding
// =============================================================================

/// Encode varint directly into buffer with minimal overhead
/// Uses stack buffer to avoid repeated bounds checks
#[inline(always)]
pub fn fast_encode_varint(value: u64, buf: &mut Vec<u8>) {
    // Fast path for common small values (0-127)
    if value < 128 {
        buf.push(value as u8);
        return;
    }

    // Fast path for 2-byte values (128-16383)
    if value < 16384 {
        buf.extend_from_slice(&[
            (value as u8) | 0x80,
            (value >> 7) as u8,
        ]);
        return;
    }

    // General case: use stack buffer
    let mut temp = [0u8; 10]; // Max varint size
    let mut i = 0;
    let mut v = value;

    while v >= 0x80 {
        temp[i] = (v as u8) | 0x80;
        v >>= 7;
        i += 1;
    }
    temp[i] = v as u8;

    buf.extend_from_slice(&temp[..=i]);
}

/// Encode signed varint with zigzag encoding
#[inline(always)]
pub fn fast_encode_signed_varint(value: i64, buf: &mut Vec<u8>) {
    let encoded = ((value << 1) ^ (value >> 63)) as u64;
    fast_encode_varint(encoded, buf);
}

// =============================================================================
// Fast Buffer Writer
// =============================================================================

/// Pre-allocated buffer for fast serialization
pub struct FastBuffer {
    data: Vec<u8>,
}

impl FastBuffer {
    /// Create with capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Get inner buffer
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Take inner buffer
    #[inline]
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    /// Push single byte
    #[inline(always)]
    pub fn push(&mut self, byte: u8) {
        self.data.push(byte);
    }

    /// Extend from slice
    #[inline(always)]
    pub fn extend(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    /// Write u32 as varint
    #[inline(always)]
    pub fn write_u32(&mut self, value: u32) {
        fast_encode_varint(value as u64, &mut self.data);
    }

    /// Write u64 as varint
    #[inline(always)]
    pub fn write_u64(&mut self, value: u64) {
        fast_encode_varint(value, &mut self.data);
    }

    /// Write i32 as signed varint
    #[inline(always)]
    pub fn write_i32(&mut self, value: i32) {
        fast_encode_signed_varint(value as i64, &mut self.data);
    }

    /// Write i64 as signed varint
    #[inline(always)]
    pub fn write_i64(&mut self, value: i64) {
        fast_encode_signed_varint(value, &mut self.data);
    }

    /// Write f32
    #[inline(always)]
    pub fn write_f32(&mut self, value: f32) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    /// Write f64
    #[inline(always)]
    pub fn write_f64(&mut self, value: f64) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    /// Write bool
    #[inline(always)]
    pub fn write_bool(&mut self, value: bool) {
        self.data.push(if value { 1 } else { 0 });
    }

    /// Write string via dictionary (returns index)
    #[inline(always)]
    pub fn write_string(&mut self, s: &str, dict: &mut FastStringDictionary) {
        let idx = dict.intern(s);
        fast_encode_varint(idx as u64, &mut self.data);
    }

    /// Current length
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clear buffer for reuse
    #[inline(always)]
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

// =============================================================================
// Fast Encode Trait
// =============================================================================

/// Trait for fast encoding without type tags
pub trait FastEncode {
    /// Encode to buffer with dictionary
    fn fast_encode_to(&self, buf: &mut FastBuffer, dict: &mut FastStringDictionary);

    /// Estimate encoded size (for pre-allocation)
    fn estimated_size(&self) -> usize {
        64 // Default estimate
    }
}

/// Encode a slice with optimizations
pub fn fast_encode_slice<T: FastEncode>(items: &[T]) -> Vec<u8> {
    use super::{TBF_MAGIC, TBF_VERSION, FLAG_DICTIONARY};

    if items.is_empty() {
        let mut result = Vec::with_capacity(16);
        result.extend_from_slice(&TBF_MAGIC);
        result.push(TBF_VERSION);
        result.push(FLAG_DICTIONARY);
        result.extend_from_slice(&[0u8; 2]);
        fast_encode_varint(0, &mut result); // Empty dictionary
        fast_encode_varint(0, &mut result); // Zero items
        return result;
    }

    // Estimate sizes for pre-allocation
    let item_size = items.first().map(|i| i.estimated_size()).unwrap_or(64);
    let total_data_size = items.len() * item_size + 16;
    // Dictionary estimate: assume 3 strings per item, ~50% are unique
    // This is more generous to avoid expensive resizes
    let dict_size = (items.len() * 3 / 2).max(64);

    // Create pre-allocated structures
    let mut dict = FastStringDictionary::with_capacity(dict_size);
    let mut buf = FastBuffer::with_capacity(total_data_size);

    // Write item count
    fast_encode_varint(items.len() as u64, &mut buf.data);

    // Encode all items
    for item in items {
        item.fast_encode_to(&mut buf, &mut dict);
    }

    // Encode dictionary
    let mut dict_buf = Vec::with_capacity(dict.len() * 16 + 16);
    dict.encode(&mut dict_buf);

    // Assemble final result in one allocation
    let total_size = 8 + dict_buf.len() + buf.len();
    let mut result = Vec::with_capacity(total_size);

    // Write header + dictionary + data in sequence
    result.extend_from_slice(&TBF_MAGIC);
    result.push(TBF_VERSION);
    result.push(FLAG_DICTIONARY);
    result.extend_from_slice(&[0u8; 2]);
    result.extend_from_slice(&dict_buf);
    result.extend_from_slice(buf.as_slice());

    result
}

// =============================================================================
// Primitive Implementations
// =============================================================================

impl FastEncode for bool {
    #[inline(always)]
    fn fast_encode_to(&self, buf: &mut FastBuffer, _dict: &mut FastStringDictionary) {
        buf.write_bool(*self);
    }
    fn estimated_size(&self) -> usize { 1 }
}

impl FastEncode for u32 {
    #[inline(always)]
    fn fast_encode_to(&self, buf: &mut FastBuffer, _dict: &mut FastStringDictionary) {
        buf.write_u32(*self);
    }
    fn estimated_size(&self) -> usize { 5 }
}

impl FastEncode for u64 {
    #[inline(always)]
    fn fast_encode_to(&self, buf: &mut FastBuffer, _dict: &mut FastStringDictionary) {
        buf.write_u64(*self);
    }
    fn estimated_size(&self) -> usize { 10 }
}

impl FastEncode for i32 {
    #[inline(always)]
    fn fast_encode_to(&self, buf: &mut FastBuffer, _dict: &mut FastStringDictionary) {
        buf.write_i32(*self);
    }
    fn estimated_size(&self) -> usize { 5 }
}

impl FastEncode for i64 {
    #[inline(always)]
    fn fast_encode_to(&self, buf: &mut FastBuffer, _dict: &mut FastStringDictionary) {
        buf.write_i64(*self);
    }
    fn estimated_size(&self) -> usize { 10 }
}

impl FastEncode for f32 {
    #[inline(always)]
    fn fast_encode_to(&self, buf: &mut FastBuffer, _dict: &mut FastStringDictionary) {
        buf.write_f32(*self);
    }
    fn estimated_size(&self) -> usize { 4 }
}

impl FastEncode for f64 {
    #[inline(always)]
    fn fast_encode_to(&self, buf: &mut FastBuffer, _dict: &mut FastStringDictionary) {
        buf.write_f64(*self);
    }
    fn estimated_size(&self) -> usize { 8 }
}

impl FastEncode for String {
    #[inline(always)]
    fn fast_encode_to(&self, buf: &mut FastBuffer, dict: &mut FastStringDictionary) {
        buf.write_string(self, dict);
    }
    fn estimated_size(&self) -> usize { 2 }
}

impl FastEncode for &str {
    #[inline(always)]
    fn fast_encode_to(&self, buf: &mut FastBuffer, dict: &mut FastStringDictionary) {
        buf.write_string(self, dict);
    }
    fn estimated_size(&self) -> usize { 2 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ahash() {
        // Verify ahash produces different hashes for different strings
        let dict = FastStringDictionary::with_capacity(10);
        let h1 = dict.hash_str("hello");
        let h2 = dict.hash_str("world");
        let h3 = dict.hash_str("hello");

        assert_ne!(h1, h2);
        assert_eq!(h1, h3);
    }

    #[test]
    fn test_fast_varint() {
        let mut buf = Vec::new();

        fast_encode_varint(0, &mut buf);
        assert_eq!(buf, vec![0]);
        buf.clear();

        fast_encode_varint(127, &mut buf);
        assert_eq!(buf, vec![127]);
        buf.clear();

        fast_encode_varint(128, &mut buf);
        assert_eq!(buf, vec![0x80, 0x01]);
        buf.clear();

        fast_encode_varint(16383, &mut buf);
        assert_eq!(buf, vec![0xFF, 0x7F]);
        buf.clear();

        fast_encode_varint(16384, &mut buf);
        assert_eq!(buf, vec![0x80, 0x80, 0x01]);
    }

    #[test]
    fn test_fast_dictionary() {
        let mut dict = FastStringDictionary::with_capacity(10);

        assert_eq!(dict.intern("hello"), 0);
        assert_eq!(dict.intern("world"), 1);
        assert_eq!(dict.intern("hello"), 0); // Should return same index
        assert_eq!(dict.intern("foo"), 2);

        assert_eq!(dict.get(0), Some("hello"));
        assert_eq!(dict.get(1), Some("world"));
        assert_eq!(dict.get(2), Some("foo"));
    }

    #[test]
    fn test_fast_buffer() {
        let mut dict = FastStringDictionary::with_capacity(10);
        let mut buf = FastBuffer::with_capacity(100);

        buf.write_u32(12345);
        buf.write_string("test", &mut dict);
        buf.write_bool(true);

        assert!(!buf.is_empty());
        assert!(buf.len() > 0);
    }
}
