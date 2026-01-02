//! TBF Decoder with direct serde integration

use super::dictionary::BorrowedDictionary;
use super::varint::*;
use super::{TBF_MAGIC, TBF_VERSION, TypeTag};
use crate::error::{InterpretError, TauqError};

/// TBF Deserializer - decodes TBF binary format directly to values
///
/// This implements serde's Deserializer trait for high-performance
/// deserialization with zero-copy string access where possible.
pub struct TbfDeserializer<'de> {
    /// Input data
    data: &'de [u8],
    /// Current position in the data
    pos: usize,
    /// String dictionary (zero-copy references into data)
    dict: BorrowedDictionary<'de>,
    /// Format flags
    flags: u8,
}

impl<'de> TbfDeserializer<'de> {
    /// Create a new deserializer from bytes
    pub fn new(data: &'de [u8]) -> Result<Self, TauqError> {
        if data.len() < 8 {
            return Err(TauqError::Interpret(InterpretError::new(
                "TBF data too short".to_string(),
            )));
        }

        // Verify magic
        if data[0..4] != TBF_MAGIC {
            return Err(TauqError::Interpret(InterpretError::new(
                "Invalid TBF magic bytes".to_string(),
            )));
        }

        let version = data[4];
        if version > TBF_VERSION {
            return Err(TauqError::Interpret(InterpretError::new(format!(
                "Unsupported TBF version: {}",
                version
            ))));
        }

        let flags = data[5];

        // Decode dictionary (zero-copy)
        let (dict, dict_len) = BorrowedDictionary::decode(&data[8..])?;

        Ok(Self {
            data,
            pos: 8 + dict_len,
            dict,
            flags,
        })
    }

    /// Get current position
    #[inline]
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Check if at end of data
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Peek at next byte without consuming
    #[inline]
    pub(crate) fn peek(&self) -> Result<u8, TauqError> {
        if self.pos >= self.data.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of data".to_string(),
            )));
        }
        Ok(self.data[self.pos])
    }

    /// Read next byte
    #[inline]
    pub(crate) fn read_byte(&mut self) -> Result<u8, TauqError> {
        if self.pos >= self.data.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of data".to_string(),
            )));
        }
        let byte = self.data[self.pos];
        self.pos += 1;
        Ok(byte)
    }

    /// Read type tag
    #[inline]
    pub(crate) fn read_tag(&mut self) -> Result<TypeTag, TauqError> {
        let byte = self.read_byte()?;
        TypeTag::from_u8(byte).ok_or_else(|| {
            TauqError::Interpret(InterpretError::new(format!("Invalid type tag: {}", byte)))
        })
    }

    /// Read a varint
    #[inline]
    pub(crate) fn read_varint(&mut self) -> Result<u64, TauqError> {
        let (value, len) = decode_varint(&self.data[self.pos..])?;
        self.pos += len;
        Ok(value)
    }

    /// Read a signed varint
    #[inline]
    pub(crate) fn read_signed_varint(&mut self) -> Result<i64, TauqError> {
        let (value, len) = decode_signed_varint(&self.data[self.pos..])?;
        self.pos += len;
        Ok(value)
    }

    /// Read u128 varint
    #[inline]
    pub(crate) fn read_u128_varint(&mut self) -> Result<u128, TauqError> {
        let (value, len) = decode_u128_varint(&self.data[self.pos..])?;
        self.pos += len;
        Ok(value)
    }

    /// Read i128 varint
    #[inline]
    pub(crate) fn read_i128_varint(&mut self) -> Result<i128, TauqError> {
        let (value, len) = decode_i128_varint(&self.data[self.pos..])?;
        self.pos += len;
        Ok(value)
    }

    /// Read f32
    #[inline]
    pub(crate) fn read_f32(&mut self) -> Result<f32, TauqError> {
        if self.pos + 4 > self.data.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of data".to_string(),
            )));
        }
        let bytes: [u8; 4] = self.data[self.pos..self.pos + 4].try_into().unwrap();
        self.pos += 4;
        Ok(f32::from_le_bytes(bytes))
    }

    /// Read f64
    #[inline]
    pub(crate) fn read_f64(&mut self) -> Result<f64, TauqError> {
        if self.pos + 8 > self.data.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of data".to_string(),
            )));
        }
        let bytes: [u8; 8] = self.data[self.pos..self.pos + 8].try_into().unwrap();
        self.pos += 8;
        Ok(f64::from_le_bytes(bytes))
    }

    /// Read a string from dictionary (zero-copy)
    #[inline]
    pub(crate) fn read_string(&mut self) -> Result<&'de str, TauqError> {
        let idx = self.read_varint()? as u32;
        self.dict.get(idx).ok_or_else(|| {
            TauqError::Interpret(InterpretError::new(format!("Invalid string index: {}", idx)))
        })
    }

    /// Read raw bytes
    #[inline]
    pub(crate) fn read_bytes(&mut self, len: usize) -> Result<&'de [u8], TauqError> {
        if self.pos + len > self.data.len() {
            return Err(TauqError::Interpret(InterpretError::new(
                "Unexpected end of data".to_string(),
            )));
        }
        let bytes = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(bytes)
    }
}

/// Sequence access for deserializing sequences
pub struct SeqAccess<'a, 'de> {
    de: &'a mut TbfDeserializer<'de>,
    remaining: usize,
}

impl<'a, 'de> SeqAccess<'a, 'de> {
    pub(crate) fn new(de: &'a mut TbfDeserializer<'de>, len: usize) -> Self {
        Self { de, remaining: len }
    }
}

impl<'a, 'de> serde::de::SeqAccess<'de> for SeqAccess<'a, 'de> {
    type Error = TauqError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}

/// Map access for deserializing maps and structs
pub struct MapAccess<'a, 'de> {
    de: &'a mut TbfDeserializer<'de>,
    remaining: usize,
}

impl<'a, 'de> MapAccess<'a, 'de> {
    pub(crate) fn new(de: &'a mut TbfDeserializer<'de>, len: usize) -> Self {
        Self { de, remaining: len }
    }
}

impl<'a, 'de> serde::de::MapAccess<'de> for MapAccess<'a, 'de> {
    type Error = TauqError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;

        // Read the key's type tag
        let tag = self.de.read_tag()?;
        if tag != TypeTag::String {
            return Err(TauqError::Interpret(InterpretError::new(
                "Map key must be string".to_string(),
            )));
        }

        // Use a key deserializer
        let key_de = StringDeserializer {
            value: self.de.read_string()?,
        };
        seed.deserialize(key_de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}

/// Simple string deserializer for map keys
struct StringDeserializer<'de> {
    value: &'de str,
}

impl<'de> serde::Deserializer<'de> for StringDeserializer<'de> {
    type Error = TauqError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.value)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.value)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.value)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.value)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any
    }
}

/// Enum access for deserializing enums
pub struct EnumAccess<'a, 'de> {
    /// Reference to the deserializer
    pub(crate) de: &'a mut TbfDeserializer<'de>,
    /// The variant name
    pub(crate) variant: &'de str,
}

impl<'a, 'de> serde::de::EnumAccess<'de> for EnumAccess<'a, 'de> {
    type Error = TauqError;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let variant_de = StringDeserializer { value: self.variant };
        let value = seed.deserialize(variant_de)?;
        Ok((value, self))
    }
}

impl<'a, 'de> serde::de::VariantAccess<'de> for EnumAccess<'a, 'de> {
    type Error = TauqError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::Deserializer::deserialize_tuple(&mut *self.de, len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::Deserializer::deserialize_struct(&mut *self.de, "", fields, visitor)
    }
}
