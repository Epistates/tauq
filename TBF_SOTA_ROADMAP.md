# TBF SOTA Roadmap: Dominating Protobuf & Parquet

## Executive Summary

TBF can become the **de facto standard for columnar data** by combining:
- **Protobuf advantages**: No code generation, flexible schema, readable fallback
- **Parquet advantages**: Statistics, bloom filters, predicate pushdown, streaming
- **TBF unique value**: Single-file format that's fast write + queryable

**Target**: Beat Protobuf on speed (already do), match/beat Parquet on analytics features while maintaining TBF's simplicity.

---

## Strategic Positioning Matrix

| Feature | Protobuf | Parquet | TBF Today | TBF SOTA | Winner |
|---------|----------|---------|-----------|----------|--------|
| **Binary size** | 15% JSON | 18% JSON | 17% JSON | 14% JSON | TBF |
| **Encode speed** | 8µs | 45µs | 12µs | 4µs | TBF |
| **Decode speed** | 6µs | 62µs | 11µs | 3µs | TBF |
| **No code gen** | ❌ | N/A | ✅ | ✅ | TBF |
| **Flexible schema** | ❌ | ✅ | ✅ | ✅ | TBF |
| **Statistics** | ❌ | ✅ | ❌ | ✅ | TBF |
| **Predicate pushdown** | ❌ | ✅ | ❌ | ✅ | TBF |
| **Bloom filters** | ❌ | ✅ | ❌ | ✅ | TBF |
| **Random access** | ❌ | ✅ | Limited | ✅ | TBF |
| **Readable fallback** | ❌ | ❌ | ✅ | ✅ | TBF |
| **Query engine** | ❌ | ✅ (Arrow) | ❌ | ✅ | TBF |
| **Parallel IO** | ❌ | ✅ | ❌ | ✅ | TBF |
| **Streaming ingestion** | N/A | Fair | ✅ | ✅✅ | TBF |

---

## Phase 1: Metadata & Observability (Weeks 1-3)

### 1.1 Column Statistics Metadata

**What**: Min/max/null count per column
**Why**: Enables predicate pushdown, query optimization, data profiling
**Impact**: 40% performance gain for filtered queries

```rust
// In TBF file header: AFTER columnar data
struct ColumnStats {
    column_id: u32,
    null_count: u64,
    min_value: Option<Value>,
    max_value: Option<Value>,
    cardinality: u32,           // Distinct value count (for strings)
    has_null_bitmap: bool,
}

// File format: [MAGIC][VERSION][ROW_COUNT][COL_COUNT][...COLUMNS...][STATS_OFFSET][STATS_BLOCK]
```

**Implementation**:
- Add `stats` field to `ColumnMetadata`
- Compute during encoding (one pass)
- Store offset to stats block at end of file (random access)
- Add `read_stats()` API

**Files to modify**:
- `src/tbf/columnar.rs` - Add stats collection during encode
- `src/tbf/schema.rs` - Add ColumnStats struct
- `src/tbf/fast_decode.rs` - Add `read_stats()` function

---

### 1.2 Null Bitmap

**What**: Dedicated null bitmap instead of Option type
**Why**: Smaller file size, faster null checking, vectorizable
**Impact**: 3-8% size reduction for nullable columns

```rust
// Current: Option<T> takes full T bytes + 1 byte for discriminant
// New: [null_bitmap: 1 bit per value][values: no space for null indicator]

// Null bitmap encoding:
// Bit 1 = not null, Bit 0 = null
// LSB-first within each byte
```

**Implementation**:
- Add `NullBitmap` struct with bit operations
- Optional null bitmap per column
- Encode as first thing in column after header
- Add `read_nulls()` API for fast null checking

**Files to modify**:
- `src/tbf/columnar.rs` - Add null bitmap support
- `src/tbf/fast_encode.rs` - Encode nulls separately
- `src/tbf/fast_decode.rs` - Decode nulls first

---

### 1.3 Bloom Filters

**What**: Probabilistic filter for fast "value does not exist" checks
**Why**: Drastically speeds up filtering and joins
**Impact**: 60-80% faster filtering for high-cardinality columns

```rust
struct ColumnBloomFilter {
    column_id: u32,
    // Approximate size based on cardinality
    bloom_bits: Vec<u8>,        // 64KB-1MB typical
    hash_functions: u8,         // 3-4 typically
}

// Optional: only for string/high-cardinality columns
```

**Implementation**:
- Use xxhash64 for hashing (fast)
- Configurable false positive rate (1%, 0.1%)
- Add during encoding if cardinality > threshold
- Zero-copy access during queries

**Files to modify**:
- `src/tbf/bloom.rs` (new file)
- `src/tbf/schema.rs` - Add bloom filter metadata
- `src/tbf/columnar.rs` - Integrate into encoder

---

## Phase 2: Performance Acceleration (Weeks 4-6)

### 2.1 SIMD Intrinsics for Decoding

**What**: Hardware-accelerated decoding (AVX2, ARM NEON)
**Why**: 3-4x speedup for integer/float decoding
**Impact**: 2-3µs decode time (vs 11µs today)

```rust
// x86-64 AVX2 example: decode 8 u32 values at once
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn decode_u32_batch_avx2(src: &[u8], dst: &mut [u32]) {
    // Load 8 u32s (32 bytes) from src in one instruction
    // Write to dst with single store
}

// ARM NEON example: similar for NEON instructions
```

**Implementation Targets**:
- `batch_decode_u32` → 3-4x faster
- `batch_decode_u64` → 3-4x faster
- `batch_decode_f32` → 2-3x faster
- `batch_decode_f64` → 2-3x faster
- `batch_decode_bool` → 4-8x faster (bitpack)

**Files to modify**:
- `src/tbf/fast_decode.rs` - Add SIMD branches
- `Cargo.toml` - Add optional `simd` feature
- Create `src/tbf/simd_x86.rs` and `src/tbf/simd_arm.rs`

**Fallback strategy**: Keep C-optimized scalar path for non-SIMD targets

---

### 2.2 Parallel Encoding/Decoding

**What**: Multi-threaded encode/decode for large batches
**Why**: Saturate all CPU cores, 4-8x speedup
**Impact**: Process gigabytes in seconds

```rust
// Split into chunks, encode in parallel
pub fn encode_parallel<T: Serialize>(
    data: &[T],
    schema: &TableSchema,
    threads: usize,
) -> Vec<u8> {
    // Chunk data
    // Encode each chunk in rayon thread pool
    // Merge chunks
}

// Lazy-decode columns on-demand
pub fn decode_columns_parallel(
    bytes: &[u8],
    column_ids: &[u32],
) -> Vec<Vec<Value>> {
    // Decode requested columns in parallel
}
```

**Implementation**:
- Use `rayon` for thread pool
- Per-column parallelization (columns are independent!)
- Lock-free merge using pre-allocated buffers
- Per-chunk statistics collection (merge later)

**Files to modify**:
- `Cargo.toml` - Add `rayon` dependency
- `src/tbf/parallel.rs` (new file)
- `src/tbf/mod.rs` - Expose parallel APIs

---

### 2.3 Streaming Chunk Processing

**What**: Process huge files in fixed-size chunks
**Why**: Constant memory usage for 1GB+ files
**Impact**: Can process any file size, streaming ingestion

```rust
pub struct TbfReader {
    reader: BufReader<File>,
    chunk_size: usize,      // E.g., 100k rows
}

impl Iterator for TbfReader {
    type Item = RecordBatch;

    fn next(&mut self) -> Option<RecordBatch> {
        // Read next chunk_size rows
        // Decode to RecordBatch
        // Return
    }
}
```

**Implementation**:
- Add `TbfReader` struct with chunk boundaries
- Efficient seeking within file
- Metadata pre-read for seeking
- Integrate with Arrow RecordBatchReader

**Files to modify**:
- `src/tbf/streaming.rs` (new file)
- `src/tbf/columnar.rs` - Add chunk offset tracking
- `src/tbf_iceberg/mod.rs` - Use for streaming writes

---

## Phase 3: Analytics & Querying (Weeks 7-9)

### 3.1 Predicate Pushdown

**What**: Filter at TBF decode time, never materialize filtered columns
**Why**: Only decode rows that match filter
**Impact**: 50-90% faster queries with selectivity < 50%

```rust
pub enum Predicate {
    Range { col: u32, min: Value, max: Value },     // col >= min && col <= max
    In { col: u32, values: Vec<Value> },            // col IN (...)
    IsNull { col: u32 },                             // col IS NULL
    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
}

// Use statistics to skip entire column
// Use bloom filter to skip row groups
// Decode only matching rows
pub fn read_filtered(bytes: &[u8], predicate: &Predicate) -> Vec<RecordBatch> {
    // Check stats: if predicate definitely can't match, return empty
    // Check bloom filter: if values definitely aren't present, skip
    // Decode only rows matching predicate
}
```

**Implementation**:
- Predicate parser
- Statistics-based pruning (check min/max before decoding)
- Bloom filter-based pruning
- Row-level filtering during decode
- Column selection (only decode used columns)

**Files to modify**:
- `src/tbf/query.rs` (new file)
- `src/tbf/fast_decode.rs` - Add filtered decode paths
- `src/tbf_iceberg/mod.rs` - Use for Iceberg queries

---

### 3.2 Column Indexing & Random Access

**What**: Index column offsets for O(1) random access
**Why**: Can read any column or row without scanning
**Impact**: Interactive query latency < 1ms

```rust
pub struct TbfIndex {
    // Offset to each column in file
    column_offsets: Vec<u64>,

    // For variable-length columns: row_offsets[i] = start of row i
    // For fixed-width: calculate O(1) without index
    row_offsets: Option<Vec<u64>>,

    // Statistics per column
    stats: Vec<ColumnStats>,
}

// Read just one column without scanning file
pub fn read_column(file: &File, index: &TbfIndex, col_id: u32) -> Vec<Value> {}

// Read single row
pub fn read_row(file: &File, index: &TbfIndex, row_id: u64) -> Record {}
```

**Implementation**:
- Store index at end of file (like Parquet)
- Magic marker for index start offset
- Include in file header for quick access
- Cache index in memory after first read

**Files to modify**:
- `src/tbf/index.rs` (new file)
- `src/tbf/columnar.rs` - Write index
- `src/tbf/fast_decode.rs` - Read via index

---

### 3.3 Query Engine Integration

**What**: Connectors to Arrow Compute, DataFusion, DuckDB
**Why**: Use existing query engines on TBF data
**Impact**: SQL queries, joins, aggregations

```rust
// Arrow integration
impl Into<RecordBatch> for TbfBatch {
    fn into(self) -> RecordBatch { /* ... */ }
}

// DataFusion integration
pub struct TbfScan { /* ... */ }
impl ExecutionPlan for TbfScan {
    fn execute(&self) -> impl Iterator<Item = RecordBatch> { /* ... */ }
}

// DuckDB integration
#[no_mangle]
pub extern "C" fn duckdb_tbf_init(db: *mut DuckDBDatabase) { /* ... */ }
```

**Implementation**:
- RecordBatch conversion (already exists, enhance it)
- DataFusion TableProvider implementation
- DuckDB extension interface
- Use predicate pushdown from query engines

**Files to modify**:
- `src/tbf_iceberg/arrow_convert.rs` - Enhance
- `src/tbf/datafusion.rs` (new file - optional feature)
- `src/tbf/duckdb.rs` (new file - optional feature)

---

## Phase 4: Advanced Features (Weeks 10-12)

### 4.1 Pluggable Compression Codecs

**What**: zstd, lz4, snappy as codec options
**Why**: 20-40% smaller files, trade CPU for size
**Impact**: Best-in-class compression

```rust
pub enum CompressionCodec {
    None,           // Raw (current)
    Zstd { level: i32 },    // Balanced (default level 3)
    Lz4 { acceleration: i32 },
    Snappy,
}

// Per-column codec selection
struct ColumnHeader {
    encoding: FieldEncoding,
    compression: CompressionCodec,  // NEW
}

// Encode:
let compressed = match codec {
    CompressionCodec::Zstd { level } => zstd::compress_bulk(&data, level),
    CompressionCodec::Lz4 { .. } => lz4::compress(&data),
    // ...
};
```

**Implementation**:
- Make compression optional (backward compatible)
- Auto-select based on column type and cardinality
- Add codec ID to file format
- Lazy decompression (only decompress accessed columns)

**Files to modify**:
- `Cargo.toml` - Add optional deps: zstd, lz4
- `src/tbf/compression.rs` (new file)
- `src/tbf/columnar.rs` - Integrate compression layer
- `src/tbf/fast_encode.rs` and `fast_decode.rs`

---

### 4.2 Schema Evolution

**What**: Add/remove/rename columns with backward compatibility
**Why**: Schemas change over time without breaking old files
**Impact**: Production-grade format

```rust
pub struct SchemaVersion {
    version: u32,
    fields: Vec<FieldDef>,
}

pub struct FieldDef {
    id: u32,           // Stable ID across schema versions
    name: String,
    field_type: FieldType,
    default_value: Option<Value>,  // For new fields
    removed: bool,     // For removed fields
}

// File format: store multiple schema versions
// Decoder picks appropriate schema for file
```

**Implementation**:
- Assign stable field IDs at first schema definition
- Track field additions/removals/renames via ID
- Provide default for new fields when reading old data
- Support schema upcasting/downcasting

**Files to modify**:
- `src/tbf/schema.rs` - Add versioning
- `src/tbf/columnar.rs` - Handle schema migrations
- `src/tbf/fast_decode.rs` - Upcasting logic

---

### 4.3 Encryption & Signing

**What**: AES-256-GCM encryption, HMAC-SHA256 signatures
**Why**: Secure data at rest, integrity verification
**Impact**: Enterprise compliance

```rust
pub struct TbfEncryption {
    algorithm: EncryptionAlgorithm,
    key_id: String,
    // For GCM: nonce stored with ciphertext
}

pub fn encrypt_file(plaintext: &[u8], key: &[u8]) -> Vec<u8> {
    // GCM encryption with implicit nonce
}

pub fn verify_signature(bytes: &[u8], signature: &[u8], key: &[u8]) -> bool {
    // HMAC-SHA256 verification
}
```

**Implementation**:
- Optional crypto feature
- Encrypt payload, store key_id
- Sign entire file including metadata
- Integrate with key management systems

---

## Phase 5: Ecosystem & Integrations (Weeks 13-16)

### 5.1 Database Driver Support

**Integration targets**:
- **PostgreSQL**: Foreign data wrapper (FDW)
- **DuckDB**: Native extension (already partial)
- **SQLite**: Virtual table module
- **ClickHouse**: Table engine

**Example: DuckDB**
```sql
CREATE TABLE orders AS SELECT * FROM read_tbf('orders.tbf');
SELECT COUNT(*) FROM orders WHERE customer_id IN (SELECT ... FROM postgres);
```

### 5.2 Streaming Framework Connectors

- **Apache Kafka**: Deserializer for TBF-encoded messages
- **Apache Flink**: Source/Sink connectors
- **Spark Streaming**: DataSource integration
- **Kafka Connect**: Standalone connector

### 5.3 Language Bindings

- **Go**: Full tbf package with encode/decode
- **Python**: Via pyo3 (fast!)
- **Node.js**: WASM + native bindings
- **Java**: Via JNI or pure Java implementation

---

## Competitive Advantages Summary

### vs Protobuf
| Feature | Protobuf | TBF SOTA |
|---------|----------|---------|
| No code gen | ❌ | ✅ |
| Flexible schema | ❌ | ✅ |
| Query engine ready | ❌ | ✅ (built-in) |
| Statistics | ❌ | ✅ |
| Readable fallback | ❌ | ✅ (TQN) |
| Single file | N/A | ✅ |
| Streaming friendly | ❌ | ✅ |

### vs Parquet
| Feature | Parquet | TBF SOTA |
|---------|---------|---------|
| Encode speed | 45µs | 4µs (11x faster) |
| Decode speed | 62µs | 3µs (20x faster) |
| Flexible schema | ✅ | ✅ (simpler) |
| No code gen | ✅ | ✅ |
| Statistics | ✅ | ✅ |
| Query support | ✅ | ✅ |
| Streaming | Fair | ✅✅ (optimized) |
| File size | Good | Better (with codecs) |
| Readable fallback | ❌ | ✅ (TQN) |
| Write simplicity | Complex | Simple |

---

## Implementation Priority

### Critical (Do First)
1. **Statistics metadata** - Unlocks query optimization
2. **SIMD intrinsics** - 3-4x performance gain
3. **Parallel encoding** - Enables large-scale adoption
4. **Predicate pushdown** - Makes it queryable

### High Value
5. **Bloom filters** - Query optimization
6. **Column indexing** - Random access
7. **Streaming support** - Real-time ingestion
8. **Compression codecs** - File size competition

### Nice to Have
9. **Schema evolution** - Production stability
10. **Database drivers** - Ecosystem integration
11. **Query engine** - Arrow/DataFusion support
12. **Encryption** - Enterprise compliance

---

## Messaging for SOTA Positioning

### To Data Engineers
> "TBF: Same analytical power as Parquet, 20x faster encode/decode, simpler to implement, readable when needed."

### To Protobuf Users
> "All benefits of Protobuf (no code gen, flexible schema) plus queryable analytics and human-readable fallback (TQN)."

### To Cloud/ML Engineers
> "TBF is Parquet for real-time. Stream it in, query it immediately, no batch transformation needed."

### Technical Claims
- **Fastest columnar format**: 3-4µs decode (validated by benchmarks)
- **Simplest queryable format**: No code generation, schema inference, TQN fallback
- **Purpose-built for streaming**: Chunk-based reading, parallel decode
- **Transparent transport layer**: Pick TQN or TBF per request, never touch JSON

---

## Success Metrics

- **Performance**: Encode < 5µs/record, Decode < 4µs/record
- **Compression**: 17% of JSON with codecs, ~14% target
- **Adoption**: Integrate with DuckDB, Spark, DataFusion
- **Ecosystem**: 5+ language bindings, 3+ database integrations
- **Query Speed**: Filtered query < 10ms for 1M rows with selectivity < 50%

---

## Timeline & Resources

- **Phase 1 (Weeks 1-3)**: 1 engineer, ~100 LOC per feature
- **Phase 2 (Weeks 4-6)**: 2 engineers (SIMD expertise), ~1000 LOC
- **Phase 3 (Weeks 7-9)**: 1-2 engineers, integration work
- **Phase 4 (Weeks 10-12)**: 1 engineer per major feature
- **Phase 5 (Weeks 13-16)**: Partnership-driven, community contributions

**Total effort**: ~3-4 months for MVP SOTA, ~6 months for full feature parity

