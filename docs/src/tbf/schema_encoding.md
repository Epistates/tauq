# Schema-Based Encoding

TBF's schema-based encoding allows you to specify type hints that optimize encoding for your specific data characteristics.

## Why Schema Encoding Matters

**Generic encoding (CLI)**: ~44-56% of JSON size (no schema knowledge)
```bash
tauq build data.tqn --format tbf  # Uses generic serde
```

**Schema-aware encoding (Rust API)**: ~17% of JSON size (~84% reduction)
```rust
#[derive(TableEncode)]
struct Data { /* ... */ }
// Uses compile-time schema + columnar layout
```

This section covers how to achieve the best compression through schema annotations.

## The TableEncode Trait

The `TableEncode` derive macro generates optimized encoding based on field annotations:

```rust
use tauq::tbf::TableEncode;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, TableEncode)]
struct Employee {
    #[tauq(encoding = "u16")]
    id: u32,

    #[tauq(encoding = "dictionary")]
    department: String,

    #[tauq(encoding = "u8", offset = 18)]
    age: u32,

    #[tauq(encoding = "f32")]
    rating: f64,
}
```

## Available Encodings

### Integer Encodings

```rust
#[tauq(encoding = "u8")]   // 0-255
#[tauq(encoding = "u16")]  // 0-65,535
#[tauq(encoding = "u32")]  // 0-4,294,967,295
#[tauq(encoding = "u64")]  // Full range

#[tauq(encoding = "i8")]   // -128 to 127
#[tauq(encoding = "i16")]  // -32,768 to 32,767
#[tauq(encoding = "i32")]  // Full i32 range
#[tauq(encoding = "i64")]  // Full i64 range

#[tauq(encoding = "varint")]  // Adaptive (default)
```

### Offset Encodings

Store values relative to a base offset to fit larger logical ranges into smaller types:

```rust
// Employee IDs 10000-10255 stored as 0-255
#[tauq(encoding = "u8", offset = 10000)]
employee_id: u32,

// Years 1900-2155 stored as 0-255
#[tauq(encoding = "u8", offset = 1900)]
birth_year: u32,

// Temperatures -40 to 215 stored as 0-255
#[tauq(encoding = "u8", offset = -40)]
temperature: i32,
```

### String Encodings

```rust
#[tauq(encoding = "dictionary")]  // Best for repeated values
#[tauq(encoding = "inline")]      // Best for unique values
#[tauq(encoding = "auto")]        // Analyze and pick best (default)
```

### Float Encodings

```rust
#[tauq(encoding = "f32")]  // 4 bytes, ~7 significant digits
#[tauq(encoding = "f64")]  // 8 bytes, ~15 significant digits (default)
```

## Manual Schema Building

For dynamic schemas or non-derive use cases:

```rust
use tauq::tbf::{TableSchemaBuilder, FieldEncoding};

let schema = TableSchemaBuilder::new()
    .column("id", FieldEncoding::U16)
    .column("name", FieldEncoding::Dictionary)
    .column("department", FieldEncoding::Dictionary)
    .column("age", FieldEncoding::U8Offset { offset: 18 })
    .column("salary", FieldEncoding::Float64)
    .column("active", FieldEncoding::Bool)
    .build();
```

## Adaptive Encoders

For cases where you don't know the data distribution upfront:

### AdaptiveIntEncoder

Collects all values first, then picks optimal encoding:

```rust
use tauq::tbf::AdaptiveIntEncoder;

let mut encoder = AdaptiveIntEncoder::new(FieldEncoding::Auto, 1000);
for value in values {
    encoder.push(value);
}
encoder.encode_to(&mut buffer);
```

### AdaptiveStringEncoder

Builds dictionary if beneficial, falls back to inline:

```rust
use tauq::tbf::AdaptiveStringEncoder;

let mut encoder = AdaptiveStringEncoder::new(FieldEncoding::Auto, 1000);
for s in strings {
    encoder.push(&s);
}
encoder.encode_to(&mut buffer);
```

## Best Practices

### 1. Use Offset Encoding for Constrained Ranges

```rust
// Bad: wastes 3 bytes per value
#[tauq(encoding = "u32")]
age: u32,  // Always 0-150

// Good: uses 1 byte per value
#[tauq(encoding = "u8")]
age: u32,  // Clamped to 0-255
```

### 2. Use Dictionary for Low-Cardinality Strings

```rust
// Department names repeat across thousands of employees
#[tauq(encoding = "dictionary")]
department: String,  // "Engineering", "Sales", "HR", etc.
```

### 3. Use f32 When Precision Allows

```rust
// Rating doesn't need 15 digits of precision
#[tauq(encoding = "f32")]
rating: f64,  // 4.5, 3.8, etc.
```

### 4. Consider Data Distribution

```rust
// If most IDs are small but some are large, use varint
#[tauq(encoding = "varint")]
user_id: u64,

// If all IDs are uniformly distributed, use fixed-width
#[tauq(encoding = "u64")]
random_id: u64,
```

## Size Impact

Example with 10,000 employees:

| Schema Configuration | Size | vs Default |
|---------------------|------|------------|
| All defaults (Auto) | 180 KB | baseline |
| Optimized types | 95 KB | -47% |
| With offset encoding | 82 KB | -54% |
| Full optimization | 68 KB | -62% |

## Encoding Selection Guide

| Data Characteristic | Recommended Encoding |
|---------------------|---------------------|
| Small positive integers (0-255) | `u8` |
| Bounded range with known min | `u8/u16` + offset |
| Mostly small, occasionally large | `varint` |
| Uniformly distributed | Fixed-width (`u32`, `u64`) |
| Few unique strings, many repeats | `dictionary` |
| Unique strings (UUIDs, etc.) | `inline` |
| Unknown string distribution | `auto` |
