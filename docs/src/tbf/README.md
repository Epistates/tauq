# Tauq Binary Format (TBF)

**TBF** is Tauq's high-performance binary serialization format, designed for maximum efficiency when token counts don't matter but speed and size do.

## When to Use TBF

| Scenario | Use TBF | Use TQN (Text) |
|----------|---------|----------------|
| LLM input/output | No | **Yes** |
| Database storage | **Yes** | No |
| Network protocols | **Yes** | No |
| Config files | No | **Yes** |
| Data interchange | **Yes** | Depends |
| Apache Iceberg tables | **Yes** | No |

## Key Features

### Compact Binary Encoding
- **Up to 83% smaller than JSON** for structured data (with schema-aware encoding)
- **44-56% reduction** with generic serde (CLI default, no schema knowledge)
- Adaptive integer encoding (uses minimum bytes needed)
- Dictionary compression for repeated strings
- Columnar encoding for tabular data

### Schema-Based Optimization
- Type-aware encoding with schema hints
- Offset-based encoding for constrained ranges
- Zero-copy deserialization where possible

### Iceberg Integration
- Native integration with Apache Iceberg tables
- Arrow RecordBatch conversion
- Compatible with Iceberg's columnar file format ecosystem

## Quick Example

```rust
use tbf_derive::{TbfEncode, TbfDecode};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, TbfEncode, TbfDecode)]
struct Employee {
    id: u32,
    name: String,
    age: u32,
    salary: f64,
}

// Encoding
let employees = vec![
    Employee { id: 1, name: "Alice".into(), age: 30, salary: 75000.0 },
    Employee { id: 2, name: "Bob".into(), age: 25, salary: 65000.0 },
];

let mut buf = Vec::new();
let mut dict = tauq::tbf::StringDictionary::new();
employees.tbf_encode_to(&mut buf, &mut dict);
```

## Compression: Two Paths

TBF offers two approaches with different compression levels:

### 1. Generic Serde Encoding (CLI - Recommended for Quick Conversion)
```bash
# Convert TQN to TBF at the command line
$ tauq build data.tqn --format tbf -o data.tbf
```
- **Achieves**: ~44-56% reduction from JSON
- **Pros**: No setup required, works with any data, automatic handling
- **Use when**: Quick conversions, dynamic data, no compile-time schema
- **Example**: 94 KB JSON → 41 KB TBF

### 2. Schema-Aware Encoding (Rust API - Recommended for Best Compression)
```rust
use tbf_derive::{TbfEncode, TbfDecode};

#[derive(Serialize, Deserialize, TbfEncode, TbfDecode)]
struct User {
    id: u32,
    age: u32,
    name: String,
}

// Generates optimized byte stream
let users = vec![/* ... */];
let mut buf = Vec::new();
let mut dict = tauq::tbf::StringDictionary::new();
users.tbf_encode_to(&mut buf, &mut dict);
```
- **Achieves**: ~83% reduction from JSON (leverages schema + columnar encoding)
- **Pros**: Optimal compression, zero-copy deserialization, type safety
- **Use when**: Rust projects, known schema, maximum compression needed
- **Example**: 92 KB JSON → 16 KB TBF

### 3. Iceberg Integration (Data Lakes)
```rust
use tauq::tbf_iceberg::TbfFileWriter;

// Write directly to Iceberg tables with columnar encoding
TbfFileWriter::new(schema).write_records(records)?;
```
- **Achieves**: ~83% reduction (full columnar optimization)
- **Pros**: Data lake integration, distributed processing, time-series friendly
- **Use when**: Iceberg tables, Arrow workflows, distributed systems

## Size Comparison

| Format | 1000 Employee Records | vs JSON |
|--------|----------------------|---------|
| JSON (minified) | 92 KB | baseline |
| TQN (text) | 43 KB | -53% |
| TBF (binary) | 16 KB | **-83%** |

## Next Steps

- [Overview](overview.md) - Detailed format description
- [Schema-Based Encoding](schema_encoding.md) - Type hints and optimization
- [Apache Iceberg Integration](iceberg.md) - Using TBF with Iceberg tables
