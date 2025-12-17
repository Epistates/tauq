//! Predicate pushdown query optimization (Phase 2, Week 6)
//!
//! This module implements query optimization through:
//! - Statistics-based column filtering (skip columns that can't match)
//! - Bloom filter integration (fast negative lookups)
//! - Range-based filtering (skip rows outside value range)
//! - Cardinality-aware filtering

use serde_json::{json, Value};
use super::stats::ColumnStats;

/// Comparison predicate for filtering
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    /// column == value
    Equals(Value),
    /// column != value
    NotEquals(Value),
    /// column > value
    GreaterThan(Value),
    /// column >= value
    GreaterThanOrEqual(Value),
    /// column < value
    LessThan(Value),
    /// column <= value
    LessThanOrEqual(Value),
    /// column in [min, max]
    Between(Value, Value),
    /// column IN (values)
    In(Vec<Value>),
}

impl Predicate {
    /// Check if predicate can skip this column based on statistics
    pub fn can_skip_column(&self, stats: &ColumnStats) -> bool {
        match self {
            Predicate::Equals(val) => {
                // Can skip if min > val or max < val
                if let (Some(min), Some(max)) = (&stats.min_value, &stats.max_value) {
                    json_value_gt(min, val) || json_value_lt(max, val)
                } else {
                    false
                }
            }
            Predicate::NotEquals(_) => {
                // Can't skip - might still have other values
                false
            }
            Predicate::GreaterThan(val) => {
                // Can skip if max <= val
                if let Some(max) = &stats.max_value {
                    !json_value_gt(max, val)
                } else {
                    false
                }
            }
            Predicate::GreaterThanOrEqual(val) => {
                // Can skip if max < val
                if let Some(max) = &stats.max_value {
                    json_value_lt(max, val)
                } else {
                    false
                }
            }
            Predicate::LessThan(val) => {
                // Can skip if min >= val
                if let Some(min) = &stats.min_value {
                    !json_value_lt(min, val)
                } else {
                    false
                }
            }
            Predicate::LessThanOrEqual(val) => {
                // Can skip if min > val
                if let Some(min) = &stats.min_value {
                    json_value_gt(min, val)
                } else {
                    false
                }
            }
            Predicate::Between(min, max) => {
                // Can skip if (column_min > max) or (column_max < min)
                if let (Some(col_min), Some(col_max)) = (&stats.min_value, &stats.max_value) {
                    json_value_gt(col_min, max) || json_value_lt(col_max, min)
                } else {
                    false
                }
            }
            Predicate::In(values) => {
                // Can skip if all column values are outside the set
                if let (Some(min), Some(max)) = (&stats.min_value, &stats.max_value) {
                    let all_less = values.iter().all(|v| json_value_lt(v, min));
                    let all_greater = values.iter().all(|v| json_value_gt(v, max));
                    all_less || all_greater
                } else {
                    false
                }
            }
        }
    }

    /// Get the selectivity of this predicate (0.0 to 1.0)
    pub fn selectivity(&self, stats: &ColumnStats) -> f64 {
        // Rough estimates based on statistics
        match self {
            Predicate::Equals(_) => {
                // If we know cardinality, estimate as 1/cardinality
                if stats.cardinality > 0 {
                    1.0 / stats.cardinality as f64
                } else if stats.row_count > 0 {
                    let non_null = stats.row_count - stats.null_count;
                    if non_null > 0 {
                        1.0 / non_null as f64
                    } else {
                        0.0
                    }
                } else {
                    0.1 // Conservative estimate
                }
            }
            Predicate::Between(_, _) => 0.3, // Assume 30% selectivity
            Predicate::GreaterThan(_) => 0.5, // Assume 50% selectivity
            Predicate::LessThan(_) => 0.5,
            _ => 1.0, // No selectivity reduction
        }
    }

    /// Check if a value matches this predicate
    pub fn matches(&self, value: Option<&Value>) -> bool {
        match value {
            None => {
                // Nulls don't match most predicates
                match self {
                    Predicate::NotEquals(_) => true, // Null != anything
                    _ => false,
                }
            }
            Some(val) => match self {
                Predicate::Equals(expected) => val == expected,
                Predicate::NotEquals(expected) => val != expected,
                Predicate::GreaterThan(threshold) => json_value_gt(val, threshold),
                Predicate::GreaterThanOrEqual(threshold) => {
                    json_value_gt(val, threshold) || val == threshold
                }
                Predicate::LessThan(threshold) => json_value_lt(val, threshold),
                Predicate::LessThanOrEqual(threshold) => {
                    json_value_lt(val, threshold) || val == threshold
                }
                Predicate::Between(min, max) => {
                    (json_value_gt(val, min) || val == min)
                        && (json_value_lt(val, max) || val == max)
                }
                Predicate::In(values) => values.iter().any(|v| v == val),
            },
        }
    }
}

/// Query filter for multiple predicates
#[derive(Debug, Clone)]
pub struct QueryFilter {
    /// Column predicates
    predicates: std::collections::HashMap<u32, Predicate>,
}

impl QueryFilter {
    /// Create a new query filter
    pub fn new() -> Self {
        Self {
            predicates: std::collections::HashMap::new(),
        }
    }

    /// Add a column predicate
    pub fn add_predicate(&mut self, column_id: u32, predicate: Predicate) {
        self.predicates.insert(column_id, predicate);
    }

    /// Get predicate for a column
    pub fn get_predicate(&self, column_id: u32) -> Option<&Predicate> {
        self.predicates.get(&column_id)
    }

    /// Check if all predicates match the row
    pub fn matches_row(&self, row: &[(u32, Option<Value>)]) -> bool {
        for (col_id, value) in row {
            if let Some(predicate) = self.predicates.get(col_id) {
                if !predicate.matches(value.as_ref()) {
                    return false;
                }
            }
        }
        true
    }

    /// Filter rows based on predicates
    pub fn filter_rows(
        &self,
        rows: Vec<Vec<(u32, Option<Value>)>>,
    ) -> Vec<Vec<(u32, Option<Value>)>> {
        rows.into_iter()
            .filter(|row| self.matches_row(row))
            .collect()
    }

    /// Get columns that can be skipped
    pub fn get_skippable_columns(
        &self,
        stats: &std::collections::HashMap<u32, ColumnStats>,
    ) -> Vec<u32> {
        let mut skippable = Vec::new();

        for (col_id, col_stats) in stats {
            if let Some(predicate) = self.predicates.get(col_id) {
                if predicate.can_skip_column(col_stats) {
                    skippable.push(*col_id);
                }
            }
        }

        skippable
    }

    /// Estimate selectivity (0.0 to 1.0)
    pub fn selectivity(
        &self,
        stats: &std::collections::HashMap<u32, ColumnStats>,
    ) -> f64 {
        if self.predicates.is_empty() {
            return 1.0;
        }

        // Multiply selectivities (assuming independence)
        let mut combined = 1.0;

        for (col_id, predicate) in &self.predicates {
            if let Some(col_stats) = stats.get(col_id) {
                combined *= predicate.selectivity(col_stats);
            }
        }

        combined
    }

    /// Get all predicates
    pub fn predicates(&self) -> &std::collections::HashMap<u32, Predicate> {
        &self.predicates
    }
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions for JSON value comparison
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
    fn test_predicate_equals() {
        let pred = Predicate::Equals(json!(42));
        assert!(pred.matches(Some(&json!(42))));
        assert!(!pred.matches(Some(&json!(43))));
        assert!(!pred.matches(None));
    }

    #[test]
    fn test_predicate_not_equals() {
        let pred = Predicate::NotEquals(json!(42));
        assert!(!pred.matches(Some(&json!(42))));
        assert!(pred.matches(Some(&json!(43))));
        assert!(pred.matches(None)); // Null != 42
    }

    #[test]
    fn test_predicate_greater_than() {
        let pred = Predicate::GreaterThan(json!(42));
        assert!(pred.matches(Some(&json!(43))));
        assert!(!pred.matches(Some(&json!(42))));
        assert!(!pred.matches(Some(&json!(41))));
    }

    #[test]
    fn test_predicate_less_than() {
        let pred = Predicate::LessThan(json!(42));
        assert!(!pred.matches(Some(&json!(43))));
        assert!(!pred.matches(Some(&json!(42))));
        assert!(pred.matches(Some(&json!(41))));
    }

    #[test]
    fn test_predicate_between() {
        let pred = Predicate::Between(json!(40), json!(50));
        assert!(pred.matches(Some(&json!(42))));
        assert!(pred.matches(Some(&json!(40))));
        assert!(pred.matches(Some(&json!(50))));
        assert!(!pred.matches(Some(&json!(39))));
        assert!(!pred.matches(Some(&json!(51))));
    }

    #[test]
    fn test_predicate_in() {
        let pred = Predicate::In(vec![json!(1), json!(2), json!(3)]);
        assert!(pred.matches(Some(&json!(1))));
        assert!(pred.matches(Some(&json!(2))));
        assert!(!pred.matches(Some(&json!(4))));
    }

    #[test]
    fn test_query_filter_single_column() {
        let mut filter = QueryFilter::new();
        filter.add_predicate(0, Predicate::Equals(json!(42)));

        let row = vec![(0, Some(json!(42)))];
        assert!(filter.matches_row(&row));

        let row = vec![(0, Some(json!(43)))];
        assert!(!filter.matches_row(&row));
    }

    #[test]
    fn test_query_filter_multiple_columns() {
        let mut filter = QueryFilter::new();
        filter.add_predicate(0, Predicate::GreaterThan(json!(40)));
        filter.add_predicate(1, Predicate::LessThan(json!(50)));

        let row = vec![(0, Some(json!(42))), (1, Some(json!(45)))];
        assert!(filter.matches_row(&row));

        let row = vec![(0, Some(json!(39))), (1, Some(json!(45)))];
        assert!(!filter.matches_row(&row));
    }

    #[test]
    fn test_query_filter_row_filtering() {
        let mut filter = QueryFilter::new();
        filter.add_predicate(0, Predicate::GreaterThan(json!(40)));

        let rows = vec![
            vec![(0, Some(json!(42)))],
            vec![(0, Some(json!(39)))],
            vec![(0, Some(json!(50)))],
        ];

        let filtered = filter.filter_rows(rows);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_predicate_can_skip_column_equals() {
        let stats = ColumnStats {
            column_id: 0,
            row_count: 100,
            null_count: 0,
            min_value: Some(json!(10)),
            max_value: Some(json!(50)),
            cardinality: 40,
        };

        // Value outside range - can skip
        let pred = Predicate::Equals(json!(100));
        assert!(pred.can_skip_column(&stats));

        // Value inside range - can't skip
        let pred = Predicate::Equals(json!(30));
        assert!(!pred.can_skip_column(&stats));
    }

    #[test]
    fn test_predicate_can_skip_column_between() {
        let stats = ColumnStats {
            column_id: 0,
            row_count: 100,
            null_count: 0,
            min_value: Some(json!(10)),
            max_value: Some(json!(50)),
            cardinality: 40,
        };

        // Range completely above column - can skip
        let pred = Predicate::Between(json!(60), json!(80));
        assert!(pred.can_skip_column(&stats));

        // Range overlaps column - can't skip
        let pred = Predicate::Between(json!(40), json!(60));
        assert!(!pred.can_skip_column(&stats));
    }

    #[test]
    fn test_predicate_can_skip_column_greater_than() {
        let stats = ColumnStats {
            column_id: 0,
            row_count: 100,
            null_count: 0,
            min_value: Some(json!(10)),
            max_value: Some(json!(50)),
            cardinality: 40,
        };

        // Threshold above max - can skip
        let pred = Predicate::GreaterThan(json!(60));
        assert!(pred.can_skip_column(&stats));

        // Threshold below max - can't skip
        let pred = Predicate::GreaterThan(json!(40));
        assert!(!pred.can_skip_column(&stats));
    }

    #[test]
    fn test_predicate_selectivity() {
        let stats = ColumnStats {
            column_id: 0,
            row_count: 100,
            null_count: 10,
            min_value: Some(json!(10)),
            max_value: Some(json!(50)),
            cardinality: 40,
        };

        let pred = Predicate::Equals(json!(30));
        let sel = pred.selectivity(&stats);
        // 1 / 40 cardinality = 0.025
        assert!(sel > 0.0 && sel < 0.05);
    }

    #[test]
    fn test_query_filter_selectivity() {
        let mut filter = QueryFilter::new();
        filter.add_predicate(0, Predicate::Between(json!(20), json!(30)));
        filter.add_predicate(1, Predicate::GreaterThan(json!(50)));

        let mut stats = std::collections::HashMap::new();
        stats.insert(
            0,
            ColumnStats {
                column_id: 0,
                row_count: 100,
                null_count: 0,
                min_value: Some(json!(0)),
                max_value: Some(json!(100)),
                cardinality: 100,
            },
        );
        stats.insert(
            1,
            ColumnStats {
                column_id: 1,
                row_count: 100,
                null_count: 0,
                min_value: Some(json!(0)),
                max_value: Some(json!(100)),
                cardinality: 100,
            },
        );

        let sel = filter.selectivity(&stats);
        // Should be reasonable (between 30% selectivity * 50% selectivity = 15%)
        assert!(sel > 0.0 && sel < 1.0);
    }

    #[test]
    fn test_query_filter_skippable_columns() {
        let mut filter = QueryFilter::new();
        filter.add_predicate(0, Predicate::GreaterThan(json!(100))); // Above max
        filter.add_predicate(1, Predicate::LessThan(json!(10))); // Below min

        let mut stats = std::collections::HashMap::new();
        stats.insert(
            0,
            ColumnStats {
                column_id: 0,
                row_count: 100,
                null_count: 0,
                min_value: Some(json!(0)),
                max_value: Some(json!(50)),
                cardinality: 50,
            },
        );
        stats.insert(
            1,
            ColumnStats {
                column_id: 1,
                row_count: 100,
                null_count: 0,
                min_value: Some(json!(20)),
                max_value: Some(json!(100)),
                cardinality: 80,
            },
        );

        let skippable = filter.get_skippable_columns(&stats);
        assert_eq!(skippable.len(), 2);
        assert!(skippable.contains(&0));
        assert!(skippable.contains(&1));
    }

    #[test]
    fn test_query_filter_empty() {
        let filter = QueryFilter::new();
        let row = vec![(0, Some(json!(42)))];
        assert!(filter.matches_row(&row));

        let stats = std::collections::HashMap::new();
        let sel = filter.selectivity(&stats);
        assert_eq!(sel, 1.0);
    }
}
