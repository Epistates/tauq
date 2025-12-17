//! Traits for compile-time TBF encoding/decoding
//!
//! These traits enable `#[derive(TbfEncode, TbfDecode)]` to generate
//! optimized binary serialization without type tags.

use super::dictionary::{BorrowedDictionary, StringDictionary};
use super::schema::{Schema, SchemaType};
use super::varint::{encode_varint, decode_varint, encode_signed_varint, decode_signed_varint};
use crate::error::{InterpretError, TauqError};

/// Trait for types that can be encoded to TBF format
///
/// Implement this trait to enable compile-time optimized binary encoding.
/// Use `#[derive(TbfEncode)]` for automatic implementation.
///
/// # Example
///
/// ```ignore
/// use tauq::tbf::TbfEncode;
///
/// #[derive(TbfEncode)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// let user = User { id: 1, name: "Alice".into() };
/// let bytes = user.tbf_encode();
/// ```
pub trait TbfEncode {
    /// Encode this value to the buffer
    fn tbf_encode_to(&self, buf: &mut Vec<u8>, dict: &mut StringDictionary);

    /// Get the schema for this type
    fn tbf_schema() -> Schema where Self: Sized {
        Schema::new(std::any::type_name::<Self>())
    }

    /// Get the SchemaType for this type (for field type inference)
    fn tbf_schema_type() -> SchemaType where Self: Sized {
        SchemaType::Map // Default for complex types
    }

    /// Get the number of fields (0 for primitives)
    fn tbf_field_count() -> usize where Self: Sized {
        0
    }

    /// Encode to a new Vec with header
    fn tbf_encode(&self) -> Vec<u8> where Self: Sized {
        use super::{TBF_MAGIC, TBF_VERSION, FLAG_DICTIONARY};

        let mut dict = StringDictionary::new();
        let mut data_buf = Vec::with_capacity(256);

        self.tbf_encode_to(&mut data_buf, &mut dict);

        // Encode dictionary
        let mut dict_buf = Vec::new();
        dict.encode(&mut dict_buf);

        // Build final output
        let mut result = Vec::with_capacity(8 + dict_buf.len() + data_buf.len());

        // Header
        result.extend_from_slice(&TBF_MAGIC);
        result.push(TBF_VERSION);
        result.push(FLAG_DICTIONARY);
        result.extend_from_slice(&[0u8; 2]); // Reserved

        // Dictionary
        result.extend_from_slice(&dict_buf);

        // Data
        result.extend_from_slice(&data_buf);

        result
    }

    /// Encode a slice of items with schema optimization
    fn tbf_encode_slice(items: &[Self]) -> Vec<u8> where Self: Sized {
        use super::{TBF_MAGIC, TBF_VERSION, FLAG_DICTIONARY};

        let mut dict = StringDictionary::new();
        let mut data_buf = Vec::with_capacity(items.len() * 64);

        // Write count
        encode_varint(items.len() as u64, &mut data_buf);

        // Encode all items (schema-based - no type tags)
        for item in items {
            item.tbf_encode_to(&mut data_buf, &mut dict);
        }

        // Build final output
        let mut dict_buf = Vec::new();
        dict.encode(&mut dict_buf);

        let mut result = Vec::with_capacity(8 + dict_buf.len() + data_buf.len());

        // Header
        result.extend_from_slice(&TBF_MAGIC);
        result.push(TBF_VERSION);
        result.push(FLAG_DICTIONARY);
        result.extend_from_slice(&[0u8; 2]);

        // Dictionary and data
        result.extend_from_slice(&dict_buf);
        result.extend_from_slice(&data_buf);

        result
    }
}

/// Trait for types that can be decoded from TBF format
///
/// Implement this trait to enable compile-time optimized binary decoding.
/// Use `#[derive(TbfDecode)]` for automatic implementation.
///
/// # Example
///
/// ```ignore
/// use tauq::tbf::TbfDecode;
///
/// #[derive(TbfDecode)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// let bytes = /* ... */;
/// let user = User::tbf_decode(&bytes).unwrap();
/// ```
pub trait TbfDecode: Sized {
    /// Decode from buffer at position
    fn tbf_decode_from(
        buf: &[u8],
        pos: &mut usize,
        dict: &BorrowedDictionary,
    ) -> Result<Self, TauqError>;

    /// Decode from a complete TBF buffer (with header)
    fn tbf_decode(bytes: &[u8]) -> Result<Self, TauqError> {
        use super::{TBF_MAGIC, TBF_VERSION};

        // Verify header
        if bytes.len() < 8 {
            return Err(TauqError::Interpret(InterpretError::new(
                "Buffer too small for TBF header",
            )));
        }

        if bytes[0..4] != TBF_MAGIC {
            return Err(TauqError::Interpret(InterpretError::new(
                "Invalid TBF magic bytes",
            )));
        }

        if bytes[4] != TBF_VERSION {
            return Err(TauqError::Interpret(InterpretError::new(
                format!("Unsupported TBF version: {}", bytes[4]),
            )));
        }

        let mut pos = 8;

        // Decode dictionary
        let (dict, dict_len) = BorrowedDictionary::decode(&bytes[pos..])?;
        pos += dict_len;

        // Decode value
        Self::tbf_decode_from(bytes, &mut pos, &dict)
    }

    /// Decode a slice of items
    fn tbf_decode_slice(bytes: &[u8]) -> Result<Vec<Self>, TauqError> {
        use super::{TBF_MAGIC, TBF_VERSION};

        // Verify header
        if bytes.len() < 8 {
            return Err(TauqError::Interpret(InterpretError::new(
                "Buffer too small for TBF header",
            )));
        }

        if bytes[0..4] != TBF_MAGIC {
            return Err(TauqError::Interpret(InterpretError::new(
                "Invalid TBF magic bytes",
            )));
        }

        if bytes[4] != TBF_VERSION {
            return Err(TauqError::Interpret(InterpretError::new(
                format!("Unsupported TBF version: {}", bytes[4]),
            )));
        }

        let mut pos = 8;

        // Decode dictionary
        let (dict, dict_len) = BorrowedDictionary::decode(&bytes[pos..])?;
        pos += dict_len;

        // Read count
        let (count, len) = decode_varint(&bytes[pos..])?;
        pos += len;

        // Decode items
        let mut items = Vec::with_capacity(count as usize);
        for _ in 0..count {
            items.push(Self::tbf_decode_from(bytes, &mut pos, &dict)?);
        }

        Ok(items)
    }
}

// =============================================================================
// Primitive Implementations
// =============================================================================

impl TbfEncode for bool {
    fn tbf_encode_to(&self, buf: &mut Vec<u8>, _dict: &mut StringDictionary) {
        buf.push(if *self { 1 } else { 0 });
    }

    fn tbf_schema_type() -> SchemaType {
        SchemaType::Bool
    }
}

impl TbfDecode for bool {
    fn tbf_decode_from(buf: &[u8], pos: &mut usize, _dict: &BorrowedDictionary) -> Result<Self, TauqError> {
        if *pos >= buf.len() {
            return Err(TauqError::Interpret(InterpretError::new("Unexpected end of buffer")));
        }
        let value = buf[*pos] != 0;
        *pos += 1;
        Ok(value)
    }
}

macro_rules! impl_unsigned {
    ($($ty:ty),*) => {
        $(
            impl TbfEncode for $ty {
                fn tbf_encode_to(&self, buf: &mut Vec<u8>, _dict: &mut StringDictionary) {
                    encode_varint(*self as u64, buf);
                }

                fn tbf_schema_type() -> SchemaType {
                    SchemaType::UInt
                }
            }

            impl TbfDecode for $ty {
                fn tbf_decode_from(buf: &[u8], pos: &mut usize, _dict: &BorrowedDictionary) -> Result<Self, TauqError> {
                    let (value, len) = decode_varint(&buf[*pos..])?;
                    *pos += len;
                    Ok(value as $ty)
                }
            }
        )*
    };
}

macro_rules! impl_signed {
    ($($ty:ty),*) => {
        $(
            impl TbfEncode for $ty {
                fn tbf_encode_to(&self, buf: &mut Vec<u8>, _dict: &mut StringDictionary) {
                    encode_signed_varint(*self as i64, buf);
                }

                fn tbf_schema_type() -> SchemaType {
                    SchemaType::Int
                }
            }

            impl TbfDecode for $ty {
                fn tbf_decode_from(buf: &[u8], pos: &mut usize, _dict: &BorrowedDictionary) -> Result<Self, TauqError> {
                    let (value, len) = decode_signed_varint(&buf[*pos..])?;
                    *pos += len;
                    Ok(value as $ty)
                }
            }
        )*
    };
}

impl_unsigned!(u8, u16, u32, u64, usize);
impl_signed!(i8, i16, i32, i64, isize);

impl TbfEncode for f32 {
    fn tbf_encode_to(&self, buf: &mut Vec<u8>, _dict: &mut StringDictionary) {
        buf.extend_from_slice(&self.to_le_bytes());
    }

    fn tbf_schema_type() -> SchemaType {
        SchemaType::F32
    }
}

impl TbfDecode for f32 {
    fn tbf_decode_from(buf: &[u8], pos: &mut usize, _dict: &BorrowedDictionary) -> Result<Self, TauqError> {
        if *pos + 4 > buf.len() {
            return Err(TauqError::Interpret(InterpretError::new("Unexpected end of buffer")));
        }
        let bytes: [u8; 4] = buf[*pos..*pos + 4].try_into().unwrap();
        *pos += 4;
        Ok(f32::from_le_bytes(bytes))
    }
}

impl TbfEncode for f64 {
    fn tbf_encode_to(&self, buf: &mut Vec<u8>, _dict: &mut StringDictionary) {
        buf.extend_from_slice(&self.to_le_bytes());
    }

    fn tbf_schema_type() -> SchemaType {
        SchemaType::F64
    }
}

impl TbfDecode for f64 {
    fn tbf_decode_from(buf: &[u8], pos: &mut usize, _dict: &BorrowedDictionary) -> Result<Self, TauqError> {
        if *pos + 8 > buf.len() {
            return Err(TauqError::Interpret(InterpretError::new("Unexpected end of buffer")));
        }
        let bytes: [u8; 8] = buf[*pos..*pos + 8].try_into().unwrap();
        *pos += 8;
        Ok(f64::from_le_bytes(bytes))
    }
}

impl TbfEncode for String {
    fn tbf_encode_to(&self, buf: &mut Vec<u8>, dict: &mut StringDictionary) {
        let idx = dict.intern(self);
        encode_varint(idx as u64, buf);
    }

    fn tbf_schema_type() -> SchemaType {
        SchemaType::String
    }
}

impl TbfDecode for String {
    fn tbf_decode_from(buf: &[u8], pos: &mut usize, dict: &BorrowedDictionary) -> Result<Self, TauqError> {
        let (idx, len) = decode_varint(&buf[*pos..])?;
        *pos += len;
        dict.get(idx as u32)
            .map(|s| s.to_string())
            .ok_or_else(|| TauqError::Interpret(InterpretError::new("Invalid string index")))
    }
}

impl TbfEncode for &str {
    fn tbf_encode_to(&self, buf: &mut Vec<u8>, dict: &mut StringDictionary) {
        let idx = dict.intern(self);
        encode_varint(idx as u64, buf);
    }

    fn tbf_schema_type() -> SchemaType {
        SchemaType::String
    }
}

// =============================================================================
// Container Implementations
// =============================================================================

impl<T: TbfEncode> TbfEncode for Vec<T> {
    fn tbf_encode_to(&self, buf: &mut Vec<u8>, dict: &mut StringDictionary) {
        encode_varint(self.len() as u64, buf);
        for item in self {
            item.tbf_encode_to(buf, dict);
        }
    }

    fn tbf_schema_type() -> SchemaType {
        SchemaType::Seq
    }
}

impl<T: TbfDecode> TbfDecode for Vec<T> {
    fn tbf_decode_from(buf: &[u8], pos: &mut usize, dict: &BorrowedDictionary) -> Result<Self, TauqError> {
        let (count, len) = decode_varint(&buf[*pos..])?;
        *pos += len;

        let mut items = Vec::with_capacity(count as usize);
        for _ in 0..count {
            items.push(T::tbf_decode_from(buf, pos, dict)?);
        }
        Ok(items)
    }
}

impl<T: TbfEncode> TbfEncode for Option<T> {
    fn tbf_encode_to(&self, buf: &mut Vec<u8>, dict: &mut StringDictionary) {
        match self {
            None => buf.push(0),
            Some(v) => {
                buf.push(1);
                v.tbf_encode_to(buf, dict);
            }
        }
    }

    fn tbf_schema_type() -> SchemaType {
        SchemaType::Option
    }
}

impl<T: TbfDecode> TbfDecode for Option<T> {
    fn tbf_decode_from(buf: &[u8], pos: &mut usize, dict: &BorrowedDictionary) -> Result<Self, TauqError> {
        if *pos >= buf.len() {
            return Err(TauqError::Interpret(InterpretError::new("Unexpected end of buffer")));
        }

        let tag = buf[*pos];
        *pos += 1;

        match tag {
            0 => Ok(None),
            1 => Ok(Some(T::tbf_decode_from(buf, pos, dict)?)),
            _ => Err(TauqError::Interpret(InterpretError::new("Invalid Option tag"))),
        }
    }
}

impl<T: TbfEncode> TbfEncode for Box<T> {
    fn tbf_encode_to(&self, buf: &mut Vec<u8>, dict: &mut StringDictionary) {
        (**self).tbf_encode_to(buf, dict);
    }

    fn tbf_schema_type() -> SchemaType {
        T::tbf_schema_type()
    }
}

impl<T: TbfDecode> TbfDecode for Box<T> {
    fn tbf_decode_from(buf: &[u8], pos: &mut usize, dict: &BorrowedDictionary) -> Result<Self, TauqError> {
        Ok(Box::new(T::tbf_decode_from(buf, pos, dict)?))
    }
}

// =============================================================================
// Tuple Implementations
// =============================================================================

impl TbfEncode for () {
    fn tbf_encode_to(&self, _buf: &mut Vec<u8>, _dict: &mut StringDictionary) {}
}

impl TbfDecode for () {
    fn tbf_decode_from(_buf: &[u8], _pos: &mut usize, _dict: &BorrowedDictionary) -> Result<Self, TauqError> {
        Ok(())
    }
}

macro_rules! impl_tuple {
    ($($idx:tt: $T:ident),+) => {
        impl<$($T: TbfEncode),+> TbfEncode for ($($T,)+) {
            fn tbf_encode_to(&self, buf: &mut Vec<u8>, dict: &mut StringDictionary) {
                $(self.$idx.tbf_encode_to(buf, dict);)+
            }
        }

        impl<$($T: TbfDecode),+> TbfDecode for ($($T,)+) {
            fn tbf_decode_from(buf: &[u8], pos: &mut usize, dict: &BorrowedDictionary) -> Result<Self, TauqError> {
                Ok(($($T::tbf_decode_from(buf, pos, dict)?,)+))
            }
        }
    };
}

impl_tuple!(0: T0);
impl_tuple!(0: T0, 1: T1);
impl_tuple!(0: T0, 1: T1, 2: T2);
impl_tuple!(0: T0, 1: T1, 2: T2, 3: T3);
impl_tuple!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4);
impl_tuple!(0: T0, 1: T1, 2: T2, 3: T3, 4: T4, 5: T5);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_roundtrip() {
        // Bool
        let v = true;
        let bytes = v.tbf_encode();
        assert_eq!(bool::tbf_decode(&bytes).unwrap(), v);

        // Integers
        let v: u32 = 12345;
        let bytes = v.tbf_encode();
        assert_eq!(u32::tbf_decode(&bytes).unwrap(), v);

        let v: i64 = -987654321;
        let bytes = v.tbf_encode();
        assert_eq!(i64::tbf_decode(&bytes).unwrap(), v);

        // Floats
        let v: f64 = 3.14159265358979;
        let bytes = v.tbf_encode();
        assert_eq!(f64::tbf_decode(&bytes).unwrap(), v);
    }

    #[test]
    fn test_string_roundtrip() {
        let v = String::from("Hello, TBF!");
        let bytes = v.tbf_encode();
        assert_eq!(String::tbf_decode(&bytes).unwrap(), v);
    }

    #[test]
    fn test_vec_roundtrip() {
        let v: Vec<u32> = vec![1, 2, 3, 4, 5];
        let bytes = v.tbf_encode();
        assert_eq!(Vec::<u32>::tbf_decode(&bytes).unwrap(), v);
    }

    #[test]
    fn test_option_roundtrip() {
        let some: Option<i32> = Some(42);
        let none: Option<i32> = None;

        let bytes = some.tbf_encode();
        assert_eq!(Option::<i32>::tbf_decode(&bytes).unwrap(), some);

        let bytes = none.tbf_encode();
        assert_eq!(Option::<i32>::tbf_decode(&bytes).unwrap(), none);
    }

    #[test]
    fn test_tuple_roundtrip() {
        let v = (1u32, String::from("test"), true);
        let bytes = v.tbf_encode();
        let decoded: (u32, String, bool) = TbfDecode::tbf_decode(&bytes).unwrap();
        assert_eq!(decoded, v);
    }
}
