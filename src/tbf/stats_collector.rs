//! Optional statistics collection during encoding
//!
//! This module provides utilities for collecting statistics during data encoding.
//! Statistics enable query optimization through:
//! - Predicate pushdown (skip columns that can't match)
//! - Cardinality estimation (for planning)
//! - Data profiling (understand distribution)

use super::stats::ColumnStats;
use super::bitmap::NullBitmap;
use crate::error::TauqError;
use std::collections::HashMap;
use serde_json::Value;

/// Collects statistics for multiple columns
#[derive(Debug, Clone)]
pub struct StatisticsCollector {
    /// Statistics per column ID
    columns: HashMap<u32, ColumnStats>,
    /// Null bitmaps per column ID (optional)
    bitmaps: HashMap<u32, NullBitmap>,
    /// Total row count
    row_count: u64,
    /// Enabled flag
    enabled: bool,
}

impl StatisticsCollector {
    /// Create a new statistics collector
    pub fn new() -> Self {
        Self {
            columns: HashMap::new(),
            bitmaps: HashMap::new(),
            row_count: 0,
            enabled: true,
        }
    }

    /// Create a disabled collector (statistics not collected)
    pub fn disabled() -> Self {
        Self {
            columns: HashMap::new(),
            bitmaps: HashMap::new(),
            row_count: 0,
            enabled: false,
        }
    }

    /// Check if statistics collection is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Update statistics for a column
    pub fn update_column(&mut self, column_id: u32, value: Option<&Value>) {
        if !self.enabled {
            return;
        }

        let stats = self
            .columns
            .entry(column_id)
            .or_insert_with(|| ColumnStats::new(column_id, self.row_count));

        stats.update(value);
    }

    /// Update null bitmap for a column
    pub fn update_bitmap(&mut self, column_id: u32, is_not_null: bool) {
        if !self.enabled {
            return;
        }

        let bitmap = self
            .bitmaps
            .entry(column_id)
            .or_insert_with(|| NullBitmap::new(1024));

        bitmap.push(is_not_null);
    }

    /// Finalize row and increment row count
    pub fn finish_row(&mut self) {
        if !self.enabled {
            return;
        }
        self.row_count += 1;
    }

    /// Get statistics for a specific column
    pub fn get_column_stats(&self, column_id: u32) -> Option<&ColumnStats> {
        self.columns.get(&column_id)
    }

    /// Get all column statistics
    pub fn get_all_stats(&self) -> impl Iterator<Item = (&u32, &ColumnStats)> {
        self.columns.iter()
    }

    /// Get null bitmap for a column
    pub fn get_bitmap(&self, column_id: u32) -> Option<&NullBitmap> {
        self.bitmaps.get(&column_id)
    }

    /// Get total row count
    pub fn row_count(&self) -> u64 {
        self.row_count
    }

    /// Encode all statistics to bytes
    pub fn encode_all(&self) -> Result<Vec<u8>, TauqError> {
        let mut buffer = Vec::new();

        // Write footer marker and version
        buffer.push(0xF1); // Footer marker
        buffer.push(1);    // Version

        // Write statistics count
        let count = self.columns.len() as u64;
        super::varint::encode_varint(count, &mut buffer);

        // Write each column's statistics
        for (_, stats) in self.columns.iter() {
            buffer.extend_from_slice(&stats.encode());
        }

        Ok(buffer)
    }

    /// Decode statistics from bytes
    pub fn decode_all(bytes: &[u8]) -> Result<(Self, usize), TauqError> {
        let mut offset = 0;

        if bytes.is_empty() {
            return Err(crate::error::TauqError::Interpret(
                crate::error::InterpretError::new("Cannot decode statistics: empty buffer"),
            ));
        }

        // Check footer marker
        if bytes[offset] != 0xF1 {
            return Err(crate::error::TauqError::Interpret(
                crate::error::InterpretError::new("Invalid statistics footer marker"),
            ));
        }
        offset += 1;

        // Check version
        let version = bytes[offset];
        offset += 1;
        if version != 1 {
            return Err(crate::error::TauqError::Interpret(
                crate::error::InterpretError::new("Unsupported statistics version"),
            ));
        }

        // Read statistics count
        let (count, size) = super::varint::decode_varint(&bytes[offset..])?;
        offset += size;

        let mut collector = StatisticsCollector::new();

        // Read each column's statistics
        for _ in 0..count {
            let (stats, size) = ColumnStats::decode(&bytes[offset..])?;
            collector.columns.insert(stats.column_id, stats);
            offset += size;
        }

        Ok((collector, offset))
    }
}

impl Default for StatisticsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_statistics_collector_basic() {
        let mut collector = StatisticsCollector::new();

        // Simulate recording column values
        collector.update_column(0, Some(&json!(10)));
        collector.update_column(0, Some(&json!(20)));
        collector.update_column(0, None);
        collector.update_column(1, Some(&json!("alice")));
        collector.update_column(1, Some(&json!("bob")));

        collector.finish_row();

        assert_eq!(collector.row_count(), 1);
        assert_eq!(collector.get_column_stats(0).unwrap().null_count, 1);
    }

    #[test]
    fn test_statistics_collector_disabled() {
        let mut collector = StatisticsCollector::disabled();

        collector.update_column(0, Some(&json!(10)));
        collector.finish_row();

        assert_eq!(collector.row_count(), 0);
        assert!(collector.get_column_stats(0).is_none());
    }

    #[test]
    fn test_statistics_collector_encode_decode() {
        let mut collector = StatisticsCollector::new();

        collector.update_column(0, Some(&json!(42)));
        collector.update_column(1, Some(&json!("test")));
        collector.finish_row();

        let encoded = collector.encode_all().unwrap();
        let (decoded, _) = StatisticsCollector::decode_all(&encoded).unwrap();

        assert_eq!(decoded.get_column_stats(0).unwrap().column_id, 0);
        assert_eq!(decoded.get_column_stats(1).unwrap().column_id, 1);
    }
}
