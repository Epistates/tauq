# TBF Specification v1.0

**Tauq Binary Format (TBF)** is a schema-aware columnar binary encoding for structured data.

## 1. File Structure

```
┌─────────────────────────────────────────┐
│ Magic Number (4 bytes): 0x54424601      │
│ ("TBF" + version 1)                     │
├─────────────────────────────────────────┤
│ Row Count (varint)                      │
├─────────────────────────────────────────┤
│ Column Count (varint)                   │
├─────────────────────────────────────────┤
│ Column 1 Data                           │
│   ├── Column Type (varint)              │
│   ├── Value Count (varint)              │
│   └── Encoded Values                    │
├─────────────────────────────────────────┤
│ Column 2 Data                           │
│   └── ...                               │
├─────────────────────────────────────────┤
│ Column N Data                           │
│   └── ...                               │
└─────────────────────────────────────────┘
```

## 2. Magic Number

| Bytes | Value | Description |
|-------|-------|-------------|
| 0-2 | `0x54 0x42 0x46` | ASCII "TBF" |
| 3 | `0x01` | Format version |

## 3. Varint Encoding

Variable-length integer encoding using continuation bits:

```
Bit 7 = 1: More bytes follow
Bit 7 = 0: Last byte

Examples:
  0         -> 0x00
  127       -> 0x7F
  128       -> 0x80 0x01
  16383     -> 0xFF 0x7F
  16384     -> 0x80 0x80 0x01
```

**Signed Varints**: Use ZigZag encoding before varint:
```
zigzag(n) = (n << 1) ^ (n >> 63)
```

## 4. Column Types

| Type ID | Name | Description |
|---------|------|-------------|
| 0 | Bool | Bit-packed booleans |
| 1 | Float32 | IEEE 754 single precision |
| 2 | Float64 | IEEE 754 double precision |
| 3 | VarInt | Variable-length signed integer |
| 4 | Dictionary | Dictionary-encoded strings |
| 5 | Inline | Length-prefixed strings |
| 6 | I8 | Signed 8-bit integer |
| 7 | I16 | Signed 16-bit little-endian |
| 8 | I32 | Signed 32-bit little-endian |
| 9 | I64 | Signed 64-bit little-endian |
| 10 | U8 | Unsigned 8-bit integer |
| 11 | U16 | Unsigned 16-bit little-endian |
| 12 | U32 | Unsigned 32-bit little-endian |
| 13 | U64 | Unsigned 64-bit little-endian |

## 5. Column Encodings

### 5.1 Boolean (Type 0)

Bit-packed, 8 values per byte:

```
[type:0][count:varint][packed_bytes...]

Bit order: LSB first
Padding: Trailing bits in last byte are 0
```

Example: `[true, false, true, true, false, false, true, false, true]`
```
Byte 0: 01001101 (bits 0-7)
Byte 1: 00000001 (bit 8, padded)
```

### 5.2 Float32 (Type 1)

IEEE 754 single precision, little-endian:

```
[type:1][count:varint][f32 values...]
```

### 5.3 Float64 (Type 2)

IEEE 754 double precision, little-endian:

```
[type:2][count:varint][f64 values...]
```

### 5.4 VarInt (Type 3)

ZigZag + varint encoded signed integers:

```
[type:3][count:varint][zigzag varints...]
```

### 5.5 Dictionary String (Type 4)

Dictionary-compressed strings:

```
[type:4][count:varint][dict_size:varint]
[dict_entry_0_len:varint][dict_entry_0_bytes...]
[dict_entry_1_len:varint][dict_entry_1_bytes...]
...
[indices as packed integers]
```

Index encoding uses minimum bits needed:
- 1-256 entries: 8-bit indices
- 257-65536 entries: 16-bit indices
- Larger: 32-bit indices

### 5.6 Inline String (Type 5)

Length-prefixed UTF-8 strings:

```
[type:5][count:varint]
[len_0:varint][bytes_0...]
[len_1:varint][bytes_1...]
...
```

### 5.7 Fixed-Width Integers (Types 6-13)

```
[type:N][count:varint][values...]
```

All multi-byte integers are little-endian.

## 6. Offset Encoding

Offset encodings are not a separate column type but a schema hint for the encoder:

```rust
// Logical value = stored_value + offset
FieldEncoding::U8Offset { offset: 18 }
```

The decoder must know the offset to reconstruct original values.

## 7. Empty File

A file with zero rows:

```
[0x54 0x42 0x46 0x01] [0x00]
(magic)               (row_count = 0)
```

## 8. Example Encoding

Data:
```json
[
  {"id": 1, "name": "Alice", "active": true},
  {"id": 2, "name": "Bob", "active": false},
  {"id": 3, "name": "Alice", "active": true}
]
```

TBF with schema `[U16, Dictionary, Bool]`:

```
54 42 46 01     # Magic
03              # Row count: 3
03              # Column count: 3

# Column 0: id (U16)
0B              # Type: U16 (11)
03              # Count: 3
01 00           # 1
02 00           # 2
03 00           # 3

# Column 1: name (Dictionary)
04              # Type: Dictionary (4)
03              # Count: 3
02              # Dict size: 2
05 41 6C 69 63 65  # "Alice" (len=5)
03 42 6F 62        # "Bob" (len=3)
00 01 00        # Indices: [0, 1, 0]

# Column 2: active (Bool)
00              # Type: Bool (0)
03              # Count: 3
05              # Packed: 0b00000101 (true, false, true)
```

Total: 35 bytes vs 89 bytes JSON (61% smaller)

## 9. Conformance

A conforming TBF encoder MUST:
1. Write the magic header `0x54424601`
2. Use little-endian byte order
3. Encode all varints correctly
4. Bit-pack booleans LSB-first

A conforming TBF decoder MUST:
1. Validate the magic header
2. Handle all column types
3. Reject malformed varints
4. Handle empty files gracefully

## 10. Security Considerations

- Decoders SHOULD limit maximum string length
- Decoders SHOULD limit dictionary size
- Decoders SHOULD validate varint does not exceed 10 bytes
- Decoders SHOULD check row/column counts against file size

## 11. MIME Type

Recommended: `application/x-tbf`

## 12. File Extension

Recommended: `.tbf`
