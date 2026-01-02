//! Ultra-high-performance TBF encoding
//!
//! Implements bitcode-style optimizations:
//! - UltraBuffer: Pointer-based unchecked push (3 ops vs Vec's ~10)
//! - Adaptive bit-packing: Sample data, choose optimal fixed-width
//! - Columnar layout: Group same-type fields for cache efficiency
//! - Batch operations: No per-value branches in hot path
//!
//! This provides bitcode-competitive performance while maintaining
//! TBF's excellent compression ratio.

use std::marker::PhantomData;

// =============================================================================
// UltraBuffer - Pointer-based buffer with unchecked operations
// =============================================================================

/// High-performance buffer using direct pointer manipulation.
///
/// Like bitcode's FastVec, this avoids the overhead of Vec::push:
/// - No capacity check per push (reserve upfront)
/// - No length calculation (track end pointer)
/// - Direct pointer write + increment
///
/// # Safety
/// Caller must ensure sufficient capacity before unchecked operations.
pub struct UltraBuffer {
    start: *mut u8,
    end: *mut u8,
    capacity: *mut u8,
    _marker: PhantomData<Vec<u8>>,
}

impl Default for UltraBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for UltraBuffer {
    fn drop(&mut self) {
        // Convert back to Vec for proper deallocation
        unsafe {
            let _ = Vec::from_raw_parts(
                self.start,
                self.len(),
                self.capacity(),
            );
        }
    }
}

// Safety: Same bounds as Vec<u8>
unsafe impl Send for UltraBuffer {}
unsafe impl Sync for UltraBuffer {}

impl UltraBuffer {
    /// Create empty buffer
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create buffer with pre-allocated capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let mut vec: Vec<u8> = Vec::with_capacity(capacity);
        let start = vec.as_mut_ptr();
        let end = start; // Empty, so end == start
        let cap = unsafe { start.add(vec.capacity()) };
        std::mem::forget(vec);

        Self {
            start,
            end,
            capacity: cap,
            _marker: PhantomData,
        }
    }

    /// Current length
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.end as usize - self.start as usize
    }

    /// Check if empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.end == self.start
    }

    /// Current capacity
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity as usize - self.start as usize
    }

    /// Remaining capacity
    #[inline(always)]
    pub fn remaining(&self) -> usize {
        self.capacity as usize - self.end as usize
    }

    /// Get slice of written data
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.start, self.len()) }
    }

    /// Convert to Vec (consumes buffer)
    #[inline]
    pub fn into_vec(self) -> Vec<u8> {
        let vec = unsafe {
            Vec::from_raw_parts(self.start, self.len(), self.capacity())
        };
        std::mem::forget(self); // Don't run Drop
        vec
    }

    /// Clear buffer (keeps capacity)
    #[inline(always)]
    pub fn clear(&mut self) {
        self.end = self.start;
    }

    /// Reserve additional capacity
    pub fn reserve(&mut self, additional: usize) {
        if additional > self.remaining() {
            self.reserve_slow(additional);
        }
    }

    #[cold]
    #[inline(never)]
    fn reserve_slow(&mut self, additional: usize) {
        // Convert to Vec, reserve, convert back
        let len = self.len();
        let cap = self.capacity();
        let mut vec = unsafe {
            Vec::from_raw_parts(self.start, len, cap)
        };
        vec.reserve(additional);

        self.start = vec.as_mut_ptr();
        self.end = unsafe { self.start.add(len) };
        self.capacity = unsafe { self.start.add(vec.capacity()) };
        std::mem::forget(vec);
    }

    /// Push single byte - UNCHECKED
    ///
    /// # Safety
    /// Caller must ensure remaining() >= 1
    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, byte: u8) {
        debug_assert!(self.end < self.capacity);
        unsafe {
            std::ptr::write(self.end, byte);
            self.end = self.end.add(1);
        }
    }

    /// Push slice - UNCHECKED
    ///
    /// # Safety
    /// Caller must ensure remaining() >= bytes.len()
    #[inline(always)]
    pub unsafe fn extend_unchecked(&mut self, bytes: &[u8]) {
        debug_assert!(self.remaining() >= bytes.len());
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.end, bytes.len());
            self.end = self.end.add(bytes.len());
        }
    }

    /// Push single byte - checked (reserves if needed)
    #[inline(always)]
    pub fn push(&mut self, byte: u8) {
        self.reserve(1);
        unsafe { self.push_unchecked(byte); }
    }

    /// Extend from slice - checked (reserves if needed)
    #[inline(always)]
    pub fn extend(&mut self, bytes: &[u8]) {
        self.reserve(bytes.len());
        unsafe { self.extend_unchecked(bytes); }
    }

    /// Get end pointer for direct writes
    #[inline(always)]
    pub fn end_ptr(&mut self) -> *mut u8 {
        self.end
    }

    /// Advance end pointer after direct writes
    ///
    /// # Safety
    /// Caller must ensure bytes written and new_end <= capacity
    #[inline(always)]
    pub unsafe fn set_end(&mut self, new_end: *mut u8) {
        debug_assert!(new_end >= self.start && new_end <= self.capacity);
        self.end = new_end;
    }
}

// =============================================================================
// Adaptive Bit Packing - Sample and pack integers
// =============================================================================

/// Packing sizes for integers (descending order)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum IntPacking {
    /// 64-bit integer packing (0)
    Bits64 = 0,
    /// 32-bit integer packing (1)
    Bits32 = 1,
    /// 16-bit integer packing (2)
    Bits16 = 2,
    /// 8-bit integer packing (3)
    Bits8 = 3,
}

impl IntPacking {
    /// Determine optimal packing from max value
    #[inline]
    pub fn from_max_u64(max: u64) -> Self {
        if max <= u8::MAX as u64 {
            IntPacking::Bits8
        } else if max <= u16::MAX as u64 {
            IntPacking::Bits16
        } else if max <= u32::MAX as u64 {
            IntPacking::Bits32
        } else {
            IntPacking::Bits64
        }
    }

    /// Determine optimal packing from max value (u32)
    #[inline]
    pub fn from_max_u32(max: u32) -> Self {
        if max <= u8::MAX as u32 {
            IntPacking::Bits8
        } else if max <= u16::MAX as u32 {
            IntPacking::Bits16
        } else {
            IntPacking::Bits32
        }
    }

    /// Bytes per value for this packing
    #[inline(always)]
    pub fn bytes_per_value(&self) -> usize {
        match self {
            IntPacking::Bits8 => 1,
            IntPacking::Bits16 => 2,
            IntPacking::Bits32 => 4,
            IntPacking::Bits64 => 8,
        }
    }
}

/// Sample min/max from first N elements (like bitcode's approach)
#[inline]
pub fn sample_minmax_u32(values: &[u32], sample_size: usize) -> (u32, u32) {
    if values.is_empty() {
        return (0, 0);
    }

    let sample = &values[..values.len().min(sample_size)];
    let mut min = sample[0];
    let mut max = sample[0];

    for &v in &sample[1..] {
        min = min.min(v);
        max = max.max(v);
    }

    // If sample doesn't cover full range, scan remainder
    if sample.len() < values.len() {
        for &v in &values[sample.len()..] {
            min = min.min(v);
            max = max.max(v);
        }
    }

    (min, max)
}

/// Sample min/max from first N elements (u64)
#[inline]
pub fn sample_minmax_u64(values: &[u64], sample_size: usize) -> (u64, u64) {
    if values.is_empty() {
        return (0, 0);
    }

    let sample = &values[..values.len().min(sample_size)];
    let mut min = sample[0];
    let mut max = sample[0];

    for &v in &sample[1..] {
        min = min.min(v);
        max = max.max(v);
    }

    if sample.len() < values.len() {
        for &v in &values[sample.len()..] {
            min = min.min(v);
            max = max.max(v);
        }
    }

    (min, max)
}

/// Pack u32 array with adaptive bit width
///
/// Returns (packing_used, offset_applied)
pub fn pack_u32_adaptive(values: &[u32], buf: &mut UltraBuffer) -> (IntPacking, bool) {
    if values.is_empty() {
        return (IntPacking::Bits8, false);
    }

    let (min, max) = sample_minmax_u32(values, 16);

    // Try offset packing (subtract min) if it helps
    let range = max.wrapping_sub(min);
    let basic_packing = IntPacking::from_max_u32(max);
    let offset_packing = IntPacking::from_max_u32(range);

    // Only use offset if it improves packing and array is large enough
    let use_offset = offset_packing > basic_packing && values.len() > 5;
    let packing = if use_offset { offset_packing } else { basic_packing };

    // Reserve space: 1 byte header + optional 4 byte offset + packed data
    let data_size = values.len() * packing.bytes_per_value();
    let header_size = 1 + if use_offset { 4 } else { 0 };
    buf.reserve(header_size + data_size);

    // Write header: packing (2 bits) + offset_flag (1 bit)
    let header = (packing as u8) << 1 | (use_offset as u8);
    unsafe { buf.push_unchecked(header); }

    // Write offset if used
    if use_offset {
        unsafe { buf.extend_unchecked(&min.to_le_bytes()); }
    }

    // Pack values
    match packing {
        IntPacking::Bits8 => {
            buf.reserve(values.len());
            if use_offset {
                for &v in values {
                    unsafe { buf.push_unchecked((v.wrapping_sub(min)) as u8); }
                }
            } else {
                for &v in values {
                    unsafe { buf.push_unchecked(v as u8); }
                }
            }
        }
        IntPacking::Bits16 => {
            buf.reserve(values.len() * 2);
            if use_offset {
                for &v in values {
                    unsafe { buf.extend_unchecked(&(v.wrapping_sub(min) as u16).to_le_bytes()); }
                }
            } else {
                for &v in values {
                    unsafe { buf.extend_unchecked(&(v as u16).to_le_bytes()); }
                }
            }
        }
        IntPacking::Bits32 => {
            buf.reserve(values.len() * 4);
            if use_offset {
                for &v in values {
                    unsafe { buf.extend_unchecked(&v.wrapping_sub(min).to_le_bytes()); }
                }
            } else {
                // Direct copy for native endian (most systems are little-endian)
                #[cfg(target_endian = "little")]
                unsafe {
                    let bytes = std::slice::from_raw_parts(
                        values.as_ptr() as *const u8,
                        values.len() * 4
                    );
                    buf.extend_unchecked(bytes);
                }
                #[cfg(target_endian = "big")]
                for &v in values {
                    unsafe { buf.extend_unchecked(&v.to_le_bytes()); }
                }
            }
        }
        IntPacking::Bits64 => unreachable!("u32 can't need 64-bit packing"),
    }

    (packing, use_offset)
}

/// Pack u64 array with adaptive bit width
pub fn pack_u64_adaptive(values: &[u64], buf: &mut UltraBuffer) -> (IntPacking, bool) {
    if values.is_empty() {
        return (IntPacking::Bits8, false);
    }

    let (min, max) = sample_minmax_u64(values, 16);
    let range = max.wrapping_sub(min);
    let basic_packing = IntPacking::from_max_u64(max);
    let offset_packing = IntPacking::from_max_u64(range);

    let use_offset = offset_packing > basic_packing && values.len() > 5;
    let packing = if use_offset { offset_packing } else { basic_packing };

    let data_size = values.len() * packing.bytes_per_value();
    let header_size = 1 + if use_offset { 8 } else { 0 };
    buf.reserve(header_size + data_size);

    let header = (packing as u8) << 1 | (use_offset as u8);
    unsafe { buf.push_unchecked(header); }

    if use_offset {
        unsafe { buf.extend_unchecked(&min.to_le_bytes()); }
    }

    match packing {
        IntPacking::Bits8 => {
            buf.reserve(values.len());
            if use_offset {
                for &v in values {
                    unsafe { buf.push_unchecked((v.wrapping_sub(min)) as u8); }
                }
            } else {
                for &v in values {
                    unsafe { buf.push_unchecked(v as u8); }
                }
            }
        }
        IntPacking::Bits16 => {
            buf.reserve(values.len() * 2);
            if use_offset {
                for &v in values {
                    unsafe { buf.extend_unchecked(&(v.wrapping_sub(min) as u16).to_le_bytes()); }
                }
            } else {
                for &v in values {
                    unsafe { buf.extend_unchecked(&(v as u16).to_le_bytes()); }
                }
            }
        }
        IntPacking::Bits32 => {
            buf.reserve(values.len() * 4);
            if use_offset {
                for &v in values {
                    unsafe { buf.extend_unchecked(&(v.wrapping_sub(min) as u32).to_le_bytes()); }
                }
            } else {
                for &v in values {
                    unsafe { buf.extend_unchecked(&(v as u32).to_le_bytes()); }
                }
            }
        }
        IntPacking::Bits64 => {
            buf.reserve(values.len() * 8);
            if use_offset {
                for &v in values {
                    unsafe { buf.extend_unchecked(&v.wrapping_sub(min).to_le_bytes()); }
                }
            } else {
                #[cfg(target_endian = "little")]
                unsafe {
                    let bytes = std::slice::from_raw_parts(
                        values.as_ptr() as *const u8,
                        values.len() * 8
                    );
                    buf.extend_unchecked(bytes);
                }
                #[cfg(target_endian = "big")]
                for &v in values {
                    unsafe { buf.extend_unchecked(&v.to_le_bytes()); }
                }
            }
        }
    }

    (packing, use_offset)
}

// =============================================================================
// String Encoding - Inline or batched
// =============================================================================

/// Encode strings inline (length-prefixed, no dictionary)
///
/// This is faster for serialization when strings are mostly unique.
pub fn encode_strings_inline(strings: &[&str], buf: &mut UltraBuffer) {
    // Calculate total size
    let total_len: usize = strings.iter().map(|s| s.len()).sum();
    let header_size = strings.len() * 4; // Max 4 bytes per length
    buf.reserve(header_size + total_len);

    for s in strings {
        let len = s.len();
        // Encode length as varint (inline for speed)
        if len < 128 {
            unsafe { buf.push_unchecked(len as u8); }
        } else if len < 16384 {
            unsafe {
                buf.push_unchecked((len as u8) | 0x80);
                buf.push_unchecked((len >> 7) as u8);
            }
        } else {
            // Rare: use standard varint encoding
            encode_varint_to_ultra(len as u64, buf);
        }
        // Copy string bytes
        unsafe { buf.extend_unchecked(s.as_bytes()); }
    }
}

/// Fast varint encoding to UltraBuffer
#[inline(always)]
pub fn encode_varint_to_ultra(mut value: u64, buf: &mut UltraBuffer) {
    buf.reserve(10); // Max varint size
    while value >= 0x80 {
        unsafe { buf.push_unchecked((value as u8) | 0x80); }
        value >>= 7;
    }
    unsafe { buf.push_unchecked(value as u8); }
}

// =============================================================================
// UltraEncode Trait - Columnar batch encoding
// =============================================================================

/// Magic bytes for Ultra format
pub const ULTRA_MAGIC: [u8; 4] = [0x55, 0x4C, 0x54, 0x01]; // "ULT\x01"

/// Ultra format version
pub const ULTRA_VERSION: u8 = 1;

/// Trait for ultra-fast columnar encoding
///
/// Implementors define columns and encode in batches for maximum throughput.
pub trait UltraEncode {
    /// Number of columns in the struct
    fn column_count() -> usize;

    /// Collect all values for each column from a slice of items
    fn collect_columns(items: &[Self], collectors: &mut ColumnCollectors) where Self: Sized;

    /// Encode a slice of items to bytes
    fn ultra_encode_slice(items: &[Self]) -> Vec<u8> where Self: Sized {
        if items.is_empty() {
            let mut buf = UltraBuffer::with_capacity(16);
            buf.extend(&ULTRA_MAGIC);
            buf.push(ULTRA_VERSION);
            buf.push(0); // flags
            encode_varint_to_ultra(0, &mut buf); // 0 items
            return buf.into_vec();
        }

        // Estimate capacity: ~20 bytes per item is a reasonable starting point
        let estimated_size = items.len() * 20 + 64;
        let mut buf = UltraBuffer::with_capacity(estimated_size);

        // Write header
        buf.extend(&ULTRA_MAGIC);
        buf.push(ULTRA_VERSION);
        buf.push(0); // flags (reserved)

        // Write item count
        encode_varint_to_ultra(items.len() as u64, &mut buf);

        // Write column count
        encode_varint_to_ultra(Self::column_count() as u64, &mut buf);

        // Collect all columns
        let mut collectors = ColumnCollectors::new(Self::column_count(), items.len());
        Self::collect_columns(items, &mut collectors);

        // Encode each column with adaptive packing
        collectors.encode_all(&mut buf);

        buf.into_vec()
    }
}

// =============================================================================
// Direct Column Encoders - Write to buffer without intermediate storage
// =============================================================================

/// Direct u32 column encoder - collects and encodes in one pass
pub struct DirectU32Encoder {
    values: Vec<u32>,
}

impl DirectU32Encoder {
    /// Create a new encoder with specified capacity
    #[inline]
    pub fn with_capacity(cap: usize) -> Self {
        Self { values: Vec::with_capacity(cap) }
    }

    /// Push a u32 value to the encoder
    #[inline(always)]
    pub fn push(&mut self, value: u32) {
        self.values.push(value);
    }

    /// Encode values to the destination buffer
    pub fn encode_to(&self, buf: &mut UltraBuffer) {
        buf.push(ColumnType::U32 as u8);
        pack_u32_adaptive(&self.values, buf);
    }
}

/// Direct string encoder - writes directly without intermediate `Vec<String>`
pub struct DirectStringEncoder {
    data: UltraBuffer,
    count: usize,
}

impl DirectStringEncoder {
    /// Create a new encoder with specified capacity
    #[inline]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            data: UltraBuffer::with_capacity(cap * 16), // estimate 16 bytes per string
            count: 0,
        }
    }

    /// Push a string value to the encoder
    #[inline(always)]
    pub fn push(&mut self, s: &str) {
        let len = s.len();
        self.data.reserve(len + 4);

        // Inline varint for length
        if len < 128 {
            unsafe { self.data.push_unchecked(len as u8); }
        } else if len < 16384 {
            unsafe {
                self.data.push_unchecked((len as u8) | 0x80);
                self.data.push_unchecked((len >> 7) as u8);
            }
        } else {
            encode_varint_to_ultra(len as u64, &mut self.data);
        }
        unsafe { self.data.extend_unchecked(s.as_bytes()); }
        self.count += 1;
    }

    /// Encode collected strings to the destination buffer
    pub fn encode_to(self, buf: &mut UltraBuffer) {
        buf.push(ColumnType::String as u8);
        buf.extend(self.data.as_slice());
    }
}

/// Trait for direct encoding without intermediate collection
pub trait UltraEncodeDirect {
    /// Encode directly to buffer with no intermediate allocations
    fn ultra_encode_direct(items: &[Self]) -> Vec<u8> where Self: Sized;
}

/// Column data type tags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ColumnType {
    /// Unsigned 32-bit integer
    U32 = 0,
    /// Unsigned 64-bit integer
    U64 = 1,
    /// Signed 32-bit integer
    I32 = 2,
    /// Signed 64-bit integer
    I64 = 3,
    /// 32-bit float
    F32 = 4,
    /// 64-bit float
    F64 = 5,
    /// Boolean
    Bool = 6,
    /// String
    String = 7,
}

/// Collected column data for batch encoding
pub struct ColumnCollectors {
    columns: Vec<ColumnData>,
}

/// Data for a single column
pub enum ColumnData {
    /// Vector of u32 values
    U32(Vec<u32>),
    /// Vector of u64 values
    U64(Vec<u64>),
    /// Vector of i32 values
    I32(Vec<i32>),
    /// Vector of i64 values
    I64(Vec<i64>),
    /// Vector of f32 values
    F32(Vec<f32>),
    /// Vector of f64 values
    F64(Vec<f64>),
    /// Vector of boolean values
    Bool(Vec<bool>),
    /// Vector of string values
    String(Vec<String>),
}

impl ColumnCollectors {
    /// Create new collectors with expected capacity
    pub fn new(column_count: usize, row_count: usize) -> Self {
        let columns = (0..column_count)
            .map(|_| ColumnData::U32(Vec::with_capacity(row_count)))
            .collect();
        Self { columns }
    }

    /// Initialize column with specific type
    pub fn init_column(&mut self, idx: usize, col_type: ColumnType, capacity: usize) {
        self.columns[idx] = match col_type {
            ColumnType::U32 => ColumnData::U32(Vec::with_capacity(capacity)),
            ColumnType::U64 => ColumnData::U64(Vec::with_capacity(capacity)),
            ColumnType::I32 => ColumnData::I32(Vec::with_capacity(capacity)),
            ColumnType::I64 => ColumnData::I64(Vec::with_capacity(capacity)),
            ColumnType::F32 => ColumnData::F32(Vec::with_capacity(capacity)),
            ColumnType::F64 => ColumnData::F64(Vec::with_capacity(capacity)),
            ColumnType::Bool => ColumnData::Bool(Vec::with_capacity(capacity)),
            ColumnType::String => ColumnData::String(Vec::with_capacity(capacity)),
        };
    }

    /// Push u32 value to column
    #[inline(always)]
    pub fn push_u32(&mut self, col: usize, value: u32) {
        if let ColumnData::U32(ref mut v) = self.columns[col] {
            v.push(value);
        }
    }

    /// Push u64 value to column
    #[inline(always)]
    pub fn push_u64(&mut self, col: usize, value: u64) {
        if let ColumnData::U64(ref mut v) = self.columns[col] {
            v.push(value);
        }
    }

    /// Push i32 value to column
    #[inline(always)]
    pub fn push_i32(&mut self, col: usize, value: i32) {
        if let ColumnData::I32(ref mut v) = self.columns[col] {
            v.push(value);
        }
    }

    /// Push i64 value to column
    #[inline(always)]
    pub fn push_i64(&mut self, col: usize, value: i64) {
        if let ColumnData::I64(ref mut v) = self.columns[col] {
            v.push(value);
        }
    }

    /// Push f32 value to column
    #[inline(always)]
    pub fn push_f32(&mut self, col: usize, value: f32) {
        if let ColumnData::F32(ref mut v) = self.columns[col] {
            v.push(value);
        }
    }

    /// Push f64 value to column
    #[inline(always)]
    pub fn push_f64(&mut self, col: usize, value: f64) {
        if let ColumnData::F64(ref mut v) = self.columns[col] {
            v.push(value);
        }
    }

    /// Push bool value to column
    #[inline(always)]
    pub fn push_bool(&mut self, col: usize, value: bool) {
        if let ColumnData::Bool(ref mut v) = self.columns[col] {
            v.push(value);
        }
    }

    /// Push string value to column
    #[inline(always)]
    pub fn push_string(&mut self, col: usize, value: &str) {
        if let ColumnData::String(ref mut v) = self.columns[col] {
            v.push(value.to_string());
        }
    }

    /// Encode all columns to buffer
    pub fn encode_all(&self, buf: &mut UltraBuffer) {
        for col in &self.columns {
            match col {
                ColumnData::U32(values) => {
                    buf.push(ColumnType::U32 as u8);
                    pack_u32_adaptive(values, buf);
                }
                ColumnData::U64(values) => {
                    buf.push(ColumnType::U64 as u8);
                    pack_u64_adaptive(values, buf);
                }
                ColumnData::I32(values) => {
                    buf.push(ColumnType::I32 as u8);
                    // Encode as u32 with zigzag
                    let unsigned: Vec<u32> = values.iter()
                        .map(|&v| ((v << 1) ^ (v >> 31)) as u32)
                        .collect();
                    pack_u32_adaptive(&unsigned, buf);
                }
                ColumnData::I64(values) => {
                    buf.push(ColumnType::I64 as u8);
                    // Encode as u64 with zigzag
                    let unsigned: Vec<u64> = values.iter()
                        .map(|&v| ((v << 1) ^ (v >> 63)) as u64)
                        .collect();
                    pack_u64_adaptive(&unsigned, buf);
                }
                ColumnData::F32(values) => {
                    buf.push(ColumnType::F32 as u8);
                    encode_f32_column(values, buf);
                }
                ColumnData::F64(values) => {
                    buf.push(ColumnType::F64 as u8);
                    encode_f64_column(values, buf);
                }
                ColumnData::Bool(values) => {
                    buf.push(ColumnType::Bool as u8);
                    encode_bool_column(values, buf);
                }
                ColumnData::String(values) => {
                    buf.push(ColumnType::String as u8);
                    encode_string_column(values, buf);
                }
            }
        }
    }
}

/// Encode f32 column (fixed width, direct copy on little-endian)
fn encode_f32_column(values: &[f32], buf: &mut UltraBuffer) {
    buf.reserve(values.len() * 4);
    #[cfg(target_endian = "little")]
    unsafe {
        let bytes = std::slice::from_raw_parts(
            values.as_ptr() as *const u8,
            values.len() * 4
        );
        buf.extend_unchecked(bytes);
    }
    #[cfg(target_endian = "big")]
    for &v in values {
        unsafe { buf.extend_unchecked(&v.to_le_bytes()); }
    }
}

/// Encode f64 column (fixed width, direct copy on little-endian)
fn encode_f64_column(values: &[f64], buf: &mut UltraBuffer) {
    buf.reserve(values.len() * 8);
    #[cfg(target_endian = "little")]
    unsafe {
        let bytes = std::slice::from_raw_parts(
            values.as_ptr() as *const u8,
            values.len() * 8
        );
        buf.extend_unchecked(bytes);
    }
    #[cfg(target_endian = "big")]
    for &v in values {
        unsafe { buf.extend_unchecked(&v.to_le_bytes()); }
    }
}

/// Encode bool column (bit-packed, 8 bools per byte)
fn encode_bool_column(values: &[bool], buf: &mut UltraBuffer) {
    let bytes_needed = values.len().div_ceil(8);
    buf.reserve(bytes_needed);

    let chunks = values.chunks(8);
    for chunk in chunks {
        let mut byte = 0u8;
        for (i, &b) in chunk.iter().enumerate() {
            if b {
                byte |= 1 << i;
            }
        }
        unsafe { buf.push_unchecked(byte); }
    }
}

/// Encode string column (length-prefixed, concatenated)
fn encode_string_column(values: &[String], buf: &mut UltraBuffer) {
    // First pass: calculate total size
    let total_bytes: usize = values.iter().map(|s| s.len()).sum();
    let max_len_bytes = values.len() * 4; // Conservative estimate for length varints
    buf.reserve(max_len_bytes + total_bytes);

    // Second pass: encode
    for s in values {
        let len = s.len();
        if len < 128 {
            unsafe { buf.push_unchecked(len as u8); }
        } else if len < 16384 {
            unsafe {
                buf.push_unchecked((len as u8) | 0x80);
                buf.push_unchecked((len >> 7) as u8);
            }
        } else {
            encode_varint_to_ultra(len as u64, buf);
        }
        unsafe { buf.extend_unchecked(s.as_bytes()); }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ultra_buffer_basic() {
        let mut buf = UltraBuffer::with_capacity(100);

        buf.push(1);
        buf.push(2);
        buf.push(3);

        assert_eq!(buf.as_slice(), &[1, 2, 3]);
        assert_eq!(buf.len(), 3);
    }

    #[test]
    fn test_ultra_buffer_extend() {
        let mut buf = UltraBuffer::with_capacity(100);

        buf.extend(&[1, 2, 3, 4, 5]);

        assert_eq!(buf.as_slice(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_ultra_buffer_grow() {
        let mut buf = UltraBuffer::with_capacity(4);

        for i in 0..100u8 {
            buf.push(i);
        }

        assert_eq!(buf.len(), 100);
        assert!(buf.capacity() >= 100);
    }

    #[test]
    fn test_pack_u32_small_values() {
        let values = vec![1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut buf = UltraBuffer::with_capacity(100);

        let (packing, offset) = pack_u32_adaptive(&values, &mut buf);

        assert_eq!(packing, IntPacking::Bits8);
        assert!(!offset);
        // Header (1) + data (10 * 1)
        assert_eq!(buf.len(), 11);
    }

    #[test]
    fn test_pack_u32_with_offset() {
        // Values 1000-1009 should use offset packing
        let values: Vec<u32> = (1000..1010).collect();
        let mut buf = UltraBuffer::with_capacity(100);

        let (packing, offset) = pack_u32_adaptive(&values, &mut buf);

        assert_eq!(packing, IntPacking::Bits8);
        assert!(offset);
    }

    #[test]
    fn test_pack_u32_large_values() {
        let values = vec![100000u32, 200000, 300000, 400000, 500000];
        let mut buf = UltraBuffer::with_capacity(100);

        let (packing, _) = pack_u32_adaptive(&values, &mut buf);

        assert_eq!(packing, IntPacking::Bits32);
    }

    #[test]
    fn test_bool_column() {
        let values = vec![true, false, true, true, false, true, false, false, true];
        let mut buf = UltraBuffer::with_capacity(100);

        encode_bool_column(&values, &mut buf);

        // 9 bools = 2 bytes
        assert_eq!(buf.len(), 2);
        // First byte: bits 0,2,3,5 set = 0b00101101 = 45
        assert_eq!(buf.as_slice()[0], 0b00101101);
        // Second byte: bit 0 set = 1
        assert_eq!(buf.as_slice()[1], 0b00000001);
    }
}
