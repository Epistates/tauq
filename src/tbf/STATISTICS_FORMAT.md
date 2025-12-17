# TBF Statistics Format (Phase 1)

## Overview

Phase 1 adds optional column statistics to TBF files for query optimization. Statistics enable:
- Predicate pushdown (skip columns that can't match filter)
- Cardinality estimation (for query planning)
- Data profiling (understand distribution)

## File Format Extension

### Current TBF Structure
```
[Header (8 bytes)]
[Dictionary]
[Schemas (optional)]
[Data]
```

### Phase 1: Add Statistics Footer
```
[Header (8 bytes)]
[Dictionary]
[Schemas (optional)]
[Data]
[Statistics Footer (Phase 1)]
  ├─ Footer Marker (0xF1, 1 byte)
  ├─ Version (1 byte)
  ├─ Stat Count (varint)
  ├─ Column Statistics...
  └─ Footer Offset (u64, 8 bytes at end)
```

## Statistics Storage Format

### Column Statistics (per column)
```
[column_id:varint]
[null_count:varint]
[has_min:u8] [min_value:encoded]
[has_max:u8] [max_value:encoded]
[cardinality:varint]
[row_count:varint]
```

### JSON Value Encoding (for min/max)
```
Type Tag (1 byte):
  0: Null
  1: Bool (bool flag, 1 byte)
  2: Number (f64, 8 bytes)
  3: String (varint len + UTF-8 bytes)
```

## Backward Compatibility

- **Footer is optional**: Readers without statistics support skip to data
- **Version field**: Allows format evolution
- **Footer marker**: Enables detection of statistics section
- **No changes to existing data**: Statistics are additive only

## Implementation Timeline

### Phase 1 Weeks 1-3 (NOW)
- ✅ Statistics modules created (ColumnStats, NullBitmap, BloomFilter)
- ✅ Unit tests passing (16 tests)
- 🔄 File format specification (THIS DOC)
- ⏳ Encoder integration (optional footer writing)
- ⏳ Decoder integration (optional footer reading)

### Phase 2 Weeks 4-6
- Performance acceleration with SIMD
- Required statistics collection during encoding
- Query optimization using statistics

### Phase 3 Weeks 7-9
- Predicate pushdown implementation
- Column indexing
- Query engine integration

## Usage Example (Future)

```rust
// Enable statistics collection
let mut encoder = TbfEncoder::with_statistics();

// Encode data
encoder.encode_record(&record)?;

// Statistics are collected automatically
let stats = encoder.column_statistics();

// File contains stats for query optimization
let bytes = encoder.into_bytes();

// Reader can access statistics
let decoder = TbfDecoder::new(&bytes)?;
if let Some(stats) = decoder.column_statistics() {
    // Use for predicate pushdown, cardinality estimation, etc.
}
```

## Notes

- Statistics are computed during encoding for efficiency
- False positive rate for bloom filters: configurable (default 1%)
- Null bitmap: 3-8% overhead for nullable columns
- Statistics collection: Optional (can be disabled)
- Storage format is deterministic for consistency
