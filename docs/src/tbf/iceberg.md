# Apache Iceberg Integration

Tauq's `tbf_iceberg` module provides seamless integration with Apache Iceberg, enabling TBF as a file format for Iceberg tables.

## Overview

Apache Iceberg is a high-performance table format for huge analytic datasets. The `tbf_iceberg` module allows you to:

- Convert Arrow RecordBatches to TBF format
- Write TBF data compatible with Iceberg workflows
- Map Iceberg/Arrow schemas to TBF encoding hints

## Enabling the Feature

Add the `iceberg` feature to your `Cargo.toml`:

```toml
[dependencies]
tauq = { version = "0.2", features = ["iceberg"] }
```

## Basic Usage

### Converting Arrow RecordBatch to TBF

```rust
use tauq::tbf_iceberg::{arrow_schema_to_tbf, ArrowToTbf};
use arrow_array::RecordBatch;

// Convert Arrow schema to TBF schema
let tbf_schema = arrow_schema_to_tbf(&arrow_schema);

// Encode RecordBatch to TBF bytes
let tbf_bytes = record_batch.encode_to_tbf(&tbf_schema);
```

### Using the TbfFileWriter

```rust
use tauq::tbf_iceberg::{TbfFileWriterBuilder, TbfFileWriter};
use iceberg::spec::Schema as IcebergSchema;

// Create writer from Iceberg schema
let mut writer = TbfFileWriterBuilder::new()
    .with_iceberg_schema(&iceberg_schema)
    .build();

// Write batches
writer.write(&batch1);
writer.write(&batch2);

// Finish and get the TBF data
let tbf_data = writer.finish();

// Access the bytes
let bytes: Vec<u8> = tbf_data.into_bytes();
println!("Wrote {} rows, {} bytes", tbf_data.row_count, bytes.len());
```

## Schema Conversion

### Arrow to TBF Type Mapping

| Arrow Type | TBF Encoding |
|------------|--------------|
| `Int8` | `I8` |
| `Int16` | `I16` |
| `Int32`, `Date32` | `I32` |
| `Int64`, `Date64`, `Timestamp` | `I64` |
| `UInt8` | `U8` |
| `UInt16` | `U16` |
| `UInt32` | `U32` |
| `UInt64` | `U64` |
| `Float32` | `Float32` |
| `Float64` | `Float64` |
| `Utf8`, `LargeUtf8` | `Dictionary` |
| `Boolean` | `Bool` |

### Iceberg to TBF Type Mapping

| Iceberg Type | TBF Encoding |
|--------------|--------------|
| `Boolean` | `Bool` |
| `Int` | `I32` |
| `Long` | `I64` |
| `Float` | `Float32` |
| `Double` | `Float64` |
| `String` | `Dictionary` |
| `Binary`, `Fixed` | `Inline` |
| `Date` | `I32` |
| `Time` | `I64` |
| `Timestamp`, `Timestamptz` | `I64` |
| `Decimal` | `VarInt` |
| `Uuid` | `Inline` |

## Complete Example

```rust
use std::sync::Arc;
use arrow_array::{Int32Array, StringArray, RecordBatch};
use arrow_schema::{DataType, Field, Schema as ArrowSchema};
use iceberg::spec::{NestedField, PrimitiveType, Schema, Type};
use tauq::tbf_iceberg::{TbfFileWriterBuilder, ArrowToTbf, arrow_schema_to_tbf};

fn main() {
    // Define Iceberg schema
    let iceberg_schema = Schema::builder()
        .with_fields(vec![
            Arc::new(NestedField::required(1, "id", Type::Primitive(PrimitiveType::Int))),
            Arc::new(NestedField::required(2, "name", Type::Primitive(PrimitiveType::String))),
        ])
        .build()
        .unwrap();

    // Create writer
    let mut writer = TbfFileWriterBuilder::new()
        .with_iceberg_schema(&iceberg_schema)
        .build();

    // Create Arrow data
    let arrow_schema = Arc::new(ArrowSchema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("name", DataType::Utf8, false),
    ]));

    let batch = RecordBatch::try_new(
        arrow_schema,
        vec![
            Arc::new(Int32Array::from(vec![1, 2, 3])),
            Arc::new(StringArray::from(vec!["Alice", "Bob", "Carol"])),
        ],
    ).unwrap();

    // Write and finish
    writer.write(&batch);
    let data = writer.finish();

    println!("TBF output: {} bytes for {} rows", data.file_size(), data.row_count);
}
```

## Custom TBF Schema

For fine-grained control over encoding:

```rust
use tauq::tbf::{TableSchemaBuilder, FieldEncoding};
use tauq::tbf_iceberg::TbfFileWriterBuilder;

// Build custom schema with specific encodings
let tbf_schema = TableSchemaBuilder::new()
    .column("id", FieldEncoding::U16)  // Override: use U16 instead of I32
    .column("name", FieldEncoding::Dictionary)
    .column("age", FieldEncoding::U8Offset { offset: 18 })
    .build();

let writer = TbfFileWriterBuilder::new()
    .with_tbf_schema(tbf_schema)
    .build();
```

## Integration Patterns

### Pattern 1: Batch Processing

```rust
// Process large datasets in batches
for batch in record_batch_reader {
    writer.write(&batch);
}
let data = writer.finish();
write_to_object_store(&data.into_bytes()).await?;
```

### Pattern 2: Direct Conversion

```rust
// One-shot conversion for smaller datasets
let tbf_schema = arrow_schema_to_tbf(&schema);
let bytes = batch.encode_to_tbf(&tbf_schema);
```

### Pattern 3: Streaming with Custom Schema

```rust
// Stream with optimized schema
let optimized_schema = analyze_data_and_build_schema(&sample_batch);
let mut writer = TbfFileWriterBuilder::new()
    .with_tbf_schema(optimized_schema)
    .build();

while let Some(batch) = stream.next().await {
    writer.write(&batch);
}
```

## Performance Considerations

1. **Schema Optimization**: Pre-analyze your data to choose optimal encodings
2. **Batch Size**: Larger batches amortize header overhead
3. **Dictionary Encoding**: Best for columns with <10% cardinality
4. **Column Order**: Put frequently-accessed columns first

## Comparison with Parquet

| Aspect | TBF | Parquet |
|--------|-----|---------|
| Encoding speed | Faster | Slower |
| Compression | Good | Better |
| Random access | Limited | Full |
| Ecosystem support | Growing | Mature |
| Best for | Fast writes, streaming | Analytics, archival |

TBF excels when write speed matters more than maximum compression, particularly for streaming ingestion pipelines.
