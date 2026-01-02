//! Column statistics for query optimization
//!
//! Statistics (min, max, null count) enable:
//! - Predicate pushdown (skip columns that can't match)
//! - Cardinality estimation (for query planning)
//! - Data profiling (understand data distribution)
//!
//! Statistics are stored in file footer for random access.

use super::varint::{encode_varint, decode_varint};
use crate::error::{TauqError, InterpretError};
use serde_json::{json, Value};

/// Statistics for a single column
#[derive(Debug, Clone)]
pub struct ColumnStats {
    /// Column identifier (field index)
    pub column_id: u32,

    /// Number of null values in this column
    pub null_count: u64,

    /// Minimum value (if orderable)
    pub min_value: Option<Value>,

    /// Maximum value (if orderable)
    pub max_value: Option<Value>,

    /// Approximate distinct value count (cardinality)
    pub cardinality: u32,

    /// Total number of rows
    pub row_count: u64,
}

impl ColumnStats {
    /// Create new column statistics
    pub fn new(column_id: u32, row_count: u64) -> Self {
        Self {
            column_id,
            null_count: 0,
            min_value: None,
            max_value: None,
            cardinality: 0,
            row_count,
        }
    }

    /// Check if value might be contained in column (based on stats)
    pub fn may_contain(&self, value: &Value) -> bool {
        match (self.min_value.as_ref(), self.max_value.as_ref()) {
            (Some(min), Some(max)) => {
                // Value is in range if: value >= min && value <= max
                !json_value_lt(value, min) && !json_value_gt(value, max)
            }
            _ => true, // Unknown range, assume it may contain
        }
    }

    /// Check if column can definitely be skipped for a range predicate
    ///
    /// Returns true if column cannot possibly contain values in [min, max]
    pub fn can_skip_range(&self, min: &Value, max: &Value) -> bool {
        match (self.min_value.as_ref(), self.max_value.as_ref()) {
            (Some(col_min), Some(col_max)) => {
                // Can skip if: col_max < min OR col_min > max
                json_value_lt(col_max, min) || json_value_gt(col_min, max)
            }
            _ => false, // Can't determine, don't skip
        }
    }

    /// Update statistics with a new value
    pub fn update(&mut self, value: Option<&Value>) {
        match value {
            Some(v) => {
                // Update min/max for orderable types
                if self.min_value.is_none() {
                    self.min_value = Some(v.clone());
                } else if let Some(min) = self.min_value.as_mut()
                    && json_value_lt(v, min)
                {
                    *min = v.clone();
                }

                if self.max_value.is_none() {
                    self.max_value = Some(v.clone());
                } else if let Some(max) = self.max_value.as_mut()
                    && json_value_gt(v, max)
                {
                    *max = v.clone();
                }

                // Update cardinality (simple approximation)
                // TODO: Use HyperLogLog for unbounded cardinality
                self.cardinality = self.cardinality.saturating_add(1);
            }
            None => {
                self.null_count += 1;
            }
        }
    }

    /// Encode statistics to bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        // Column ID
        encode_varint(self.column_id as u64, &mut buffer);

        // Null count
        encode_varint(self.null_count, &mut buffer);

        // Min value
        if let Some(min) = &self.min_value {
            buffer.push(1); // Has min
            encode_json_value(min, &mut buffer);
        } else {
            buffer.push(0); // No min
        }

        // Max value
        if let Some(max) = &self.max_value {
            buffer.push(1); // Has max
            encode_json_value(max, &mut buffer);
        } else {
            buffer.push(0); // No max
        }

        // Cardinality
        encode_varint(self.cardinality as u64, &mut buffer);

        // Row count
        encode_varint(self.row_count, &mut buffer);

        buffer
    }

    /// Decode statistics from bytes
    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), TauqError> {
        let mut offset = 0;

        // Column ID
        let (column_id, size) = decode_varint(&bytes[offset..])?;
        offset += size;

        // Null count
        let (null_count, size) = decode_varint(&bytes[offset..])?;
        offset += size;

        // Min value
        let min_value = if bytes[offset] == 1 {
            offset += 1;
            let (val, size) = decode_json_value(&bytes[offset..])?;
            offset += size;
            Some(val)
        } else {
            offset += 1;
            None
        };

        // Max value
        let max_value = if bytes[offset] == 1 {
            offset += 1;
            let (val, size) = decode_json_value(&bytes[offset..])?;
            offset += size;
            Some(val)
        } else {
            offset += 1;
            None
        };

        // Cardinality
        let (cardinality, size) = decode_varint(&bytes[offset..])?;
        offset += size;

        // Row count
        let (row_count, size) = decode_varint(&bytes[offset..])?;
        offset += size;

        Ok((
            ColumnStats {
                column_id: column_id as u32,
                null_count,
                min_value,
                max_value,
                cardinality: cardinality as u32,
                row_count,
            },
            offset,
        ))
    }
}

/// Encode JSON value as compact bytes
fn encode_json_value(value: &Value, buf: &mut Vec<u8>) {
    // Type tag (0=null, 1=bool, 2=number, 3=string)
    match value {
        Value::Null => {
            buf.push(0);
        }
        Value::Bool(b) => {
            buf.push(1);
            buf.push(if *b { 1 } else { 0 });
        }
        Value::Number(n) => {
            buf.push(2);
            // Encode number as f64 bits
            if let Some(f) = n.as_f64() {
                buf.extend_from_slice(&f.to_le_bytes());
            } else {
                // Fallback to i64
                let i = n.as_i64().unwrap_or(0);
                buf.extend_from_slice(&i.to_le_bytes());
            }
        }
        Value::String(s) => {
            buf.push(3);
            encode_varint(s.len() as u64, buf);
            buf.extend_from_slice(s.as_bytes());
        }
        _ => {
            // For complex types, use null
            buf.push(0);
        }
    }
}

/// Decode JSON value from bytes
fn decode_json_value(bytes: &[u8]) -> Result<(Value, usize), TauqError> {
    if bytes.is_empty() {
        return Err(TauqError::Interpret(InterpretError::new(
            "Cannot decode JSON value: empty buffer",
        )));
    }

    let tag = bytes[0];
    let mut offset = 1;

    match tag {
        0 => Ok((Value::Null, 1)),
        1 => {
            if bytes.len() < 2 {
                return Err(TauqError::Interpret(InterpretError::new("Invalid bool value")));
            }
            let b = bytes[1] != 0;
            Ok((Value::Bool(b), 2))
        }
        2 => {
            // Try f64 first (8 bytes)
            if bytes.len() >= offset + 8 {
                let bytes_arr: [u8; 8] = [
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                    bytes[offset + 4],
                    bytes[offset + 5],
                    bytes[offset + 6],
                    bytes[offset + 7],
                ];
                let f = f64::from_le_bytes(bytes_arr);
                Ok((json!(f), offset + 8))
            } else {
                Err(TauqError::Interpret(InterpretError::new("Invalid number value")))
            }
        }
        3 => {
            // String
            let (len, size) = decode_varint(&bytes[offset..])?;
            offset += size;
            let len = len as usize;
            if bytes.len() < offset + len {
                return Err(TauqError::Interpret(InterpretError::new("Invalid string value")));
            }
            let s = String::from_utf8(bytes[offset..offset + len].to_vec())
                .map_err(|_| TauqError::Interpret(InterpretError::new("Invalid UTF-8 string")))?;
            Ok((Value::String(s), offset + len))
        }
        _ => Err(TauqError::Interpret(InterpretError::new(
            format!("Unknown JSON value type tag: {}", tag),
        ))),
    }
}

/// Compare two JSON values for less-than (for statistics)
fn json_value_lt(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(an), Value::Number(bn)) => {
            if let (Some(af), Some(bf)) = (an.as_f64(), bn.as_f64()) {
                af < bf
            } else {
                false
            }
        }
        (Value::String(as_), Value::String(bs)) => as_ < bs,
        _ => false,
    }
}

/// Compare two JSON values for greater-than (for statistics)
fn json_value_gt(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(an), Value::Number(bn)) => {
            if let (Some(af), Some(bf)) = (an.as_f64(), bn.as_f64()) {
                af > bf
            } else {
                false
            }
        }
        (Value::String(as_), Value::String(bs)) => as_ > bs,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_may_contain() {
        let mut stats = ColumnStats::new(0, 100);
        stats.min_value = Some(json!(10));
        stats.max_value = Some(json!(50));

        assert!(stats.may_contain(&json!(25)));
        assert!(stats.may_contain(&json!(10)));
        assert!(stats.may_contain(&json!(50)));
        assert!(!stats.may_contain(&json!(5)));
        assert!(!stats.may_contain(&json!(100)));
    }

    #[test]
    fn test_stats_can_skip_range() {
        let mut stats = ColumnStats::new(0, 100);
        stats.min_value = Some(json!(10));
        stats.max_value = Some(json!(50));

        // Query: [60, 80] - outside range
        assert!(stats.can_skip_range(&json!(60), &json!(80)));

        // Query: [0, 5] - outside range
        assert!(stats.can_skip_range(&json!(0), &json!(5)));

        // Query: [25, 75] - overlaps
        assert!(!stats.can_skip_range(&json!(25), &json!(75)));

        // Query: [10, 50] - exact match
        assert!(!stats.can_skip_range(&json!(10), &json!(50)));
    }

    #[test]
    fn test_stats_encode_decode() {
        let mut stats = ColumnStats::new(42, 1000);
        stats.null_count = 5;
        stats.min_value = Some(json!(10));
        stats.max_value = Some(json!(100));
        stats.cardinality = 95;

        let encoded = stats.encode();
        let (decoded, _) = ColumnStats::decode(&encoded).unwrap();

        assert_eq!(decoded.column_id, 42);
        assert_eq!(decoded.null_count, 5);

        // Compare numeric values (encoding converts to f64, so compare semantically)
        if let (Some(Value::Number(min)), Some(Value::Number(orig_min))) =
            (decoded.min_value.as_ref(), stats.min_value.as_ref()) {
            assert_eq!(min.as_f64(), orig_min.as_f64());
        } else {
            panic!("min_value mismatch");
        }

        if let (Some(Value::Number(max)), Some(Value::Number(orig_max))) =
            (decoded.max_value.as_ref(), stats.max_value.as_ref()) {
            assert_eq!(max.as_f64(), orig_max.as_f64());
        } else {
            panic!("max_value mismatch");
        }

        assert_eq!(decoded.cardinality, 95);
        assert_eq!(decoded.row_count, 1000);
    }

    #[test]
    fn test_stats_update() {
        let mut stats = ColumnStats::new(0, 0);

        stats.update(Some(&json!(5)));
        assert_eq!(stats.min_value, Some(json!(5)));
        assert_eq!(stats.max_value, Some(json!(5)));

        stats.update(Some(&json!(10)));
        assert_eq!(stats.min_value, Some(json!(5)));
        assert_eq!(stats.max_value, Some(json!(10)));

        stats.update(Some(&json!(1)));
        assert_eq!(stats.min_value, Some(json!(1)));
        assert_eq!(stats.max_value, Some(json!(10)));

        stats.update(None);
        assert_eq!(stats.null_count, 1);
    }
}
