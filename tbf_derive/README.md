# tbf_derive

**Derive macros for TBF (Tauq Binary Format) - high-performance compile-time schema generation.**

This crate provides `#[derive(TbfEncode)]` and `#[derive(TbfDecode)]` macros that generate optimized, schema-aware serialization code for Rust structs and enums.

## Features

- **Optimal Token Usage**: Generates code that serializes directly to the TBF format without type tags or field names, saving up to 83% vs JSON.
- **Generics Support**: Fully supports generic structs and enums with proper trait bounds.
- **Field Skipping**: Exclude fields from serialization using `#[tbf(skip)]`.
- **Safety First**: Generated decoders include robust bounds checking to prevent panics on untrusted input.
- **Enum Support**: Efficiently encodes enum variants using varints.

## Usage

Add `tauq` and `tbf_derive` to your `Cargo.toml`:

```toml
[dependencies]
tauq = "0.2"
tbf_derive = "0.2"
```

Then derive the traits:

```rust
use tbf_derive::{TbfEncode, TbfDecode};

#[derive(TbfEncode, TbfDecode, Debug, PartialEq)]
struct User<T> {
    id: u32,
    name: String,
    metadata: T,
    #[tbf(skip)]
    session_id: String, // Will be Default::default() on decode
}

// Serialization
let user = User {
    id: 1,
    name: "Alice".into(),
    metadata: 42u32,
    session_id: "secret".into(),
};
let mut buf = Vec::new();
let mut dict = tauq::tbf::StringDictionary::new();
user.tbf_encode_to(&mut buf, &mut dict);

// Deserialization
let mut pos = 0;
let borrowed_dict = dict.as_borrowed();
let decoded: User<u32> = User::tbf_decode_from(&buf, &mut pos, &borrowed_dict).unwrap();

assert_eq!(user.name, decoded.name);
assert_eq!(decoded.session_id, ""); // Default for String
```

## How it Works

The macros analyze your Rust types at compile time and generate a linear sequence of calls to `tauq::tbf` primitives. This removes all the overhead of dynamic dispatch or reflection during serialization.

The generated code for a `u32` field looks like:
```rust
let (v, len) = tauq::tbf::decode_varint(&buf[*pos..])?;
*pos += len;
let id = v as u32;
```

## Safety

All decoders perform strict bounds checking. If a binary stream is truncated, the decoder will return a `tauq::TauqError::Interpret(Unexpected EOF)` instead of panicking, making it suitable for processing data from the network.

## License

MIT
