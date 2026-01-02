# TBF Overview

Tauq Binary Format (TBF) is a schema-aware binary encoding designed for high-throughput data processing.

## Format Structure

TBF files consist of:

1. **Magic Header** (4 bytes): `0x54 0x42 0x46 0x01` ("TBF" + version)
2. **Row Count** (varint): Number of records
3. **Column Count** (varint): Number of fields per record
4. **Column Data**: Each column encoded sequentially

```
[MAGIC:4][VERSION:1][ROW_COUNT:varint][COL_COUNT:varint][COL_1][COL_2]...[COL_N]
```

## Encoding Types

### Integer Encodings

| Encoding | Range | Bytes |
|----------|-------|-------|
| `I8` | -128 to 127 | 1 |
| `I16` | -32,768 to 32,767 | 2 |
| `I32` | -2B to 2B | 4 |
| `I64` | Full i64 range | 8 |
| `U8` | 0 to 255 | 1 |
| `U16` | 0 to 65,535 | 2 |
| `U32` | 0 to 4B | 4 |
| `U64` | Full u64 range | 8 |
| `VarInt` | Adaptive | 1-10 |

### Offset Encodings

Offset encodings store values relative to a base offset, useful for constrained ranges:

```rust
// Age values 18-100 stored as 0-82 in a single byte
#[tauq(encoding = "u8", offset = 18)]
age: u32,

// Years 2020-2275 stored as 0-255
#[tauq(encoding = "u8", offset = 2020)]
year: u32,
```

### String Encodings

| Encoding | Best For | Description |
|----------|----------|-------------|
| `Dictionary` | Repeated values | Builds dictionary, stores indices |
| `Inline` | Unique values | Length-prefixed UTF-8 |
| `Auto` | Unknown | Analyzes data, picks best |

### Float Encodings

| Encoding | Precision | Bytes |
|----------|-----------|-------|
| `Float32` | ~7 digits | 4 |
| `Float64` | ~15 digits | 8 |

## Columnar Layout

TBF uses columnar encoding for tabular data, storing all values of each column together:

```
Traditional (row-major):
[id:1, name:"Alice", age:30][id:2, name:"Bob", age:25]...

TBF (column-major):
[1, 2, 3, ...][Alice, Bob, Carol, ...][30, 25, 28, ...]
```

Benefits:
- Better compression (similar values together)
- SIMD-friendly processing
- Selective column reading

## Adaptive Encoding

The `AdaptiveIntEncoder` analyzes values and picks optimal encoding:

```rust
// Small range (0-255): uses U8
let ages = vec![25, 30, 28, 35, 42];

// Large range: uses VarInt
let ids = vec![1, 1000, 1000000, 999999999];
```

## Wire Format Details

### Varint Encoding

Variable-length integers use continuation bits:

```
Value 0-127:     [0xxxxxxx]
Value 128-16383: [1xxxxxxx][0xxxxxxx]
...continues for larger values
```

### Column Header

Each column starts with:
```
[COLUMN_TYPE:varint][VALUE_COUNT:varint][...data...]
```

Column types:
- 0: Bool (packed bits)
- 1: Float32
- 2: Float64
- 3: VarInt
- 4: Dictionary string
- 5: Inline string
- 6-13: Fixed-width integers (I8, I16, I32, I64, U8, U16, U32, U64)

## Performance Characteristics

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Encode | O(n) | Single pass |
| Decode | O(n) | Single pass |
| Random access | O(1) | With index |
| Compression | ~83% vs JSON | Depends on data |

## Example: Full Encoding

```rust
use tauq::tbf::{TableSchemaBuilder, FieldEncoding};

// Define schema
let schema = TableSchemaBuilder::new()
    .column("id", FieldEncoding::U16)
    .column("name", FieldEncoding::Dictionary)
    .column("age", FieldEncoding::U8Offset { offset: 18 })
    .column("salary", FieldEncoding::Float64)
    .build();

// Encode data
let bytes = data.encode_with_schema(&schema);
```
