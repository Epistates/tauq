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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
            TauqError::Interpret(InterpretError::new(format!(
                "Invalid string index: {}",
                idx
            )))
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
        let variant_de = StringDeserializer {
            value: self.variant,
        };
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

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::Deserializer::deserialize_struct(&mut *self.de, "", fields, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tbf::{TBF_MAGIC, TBF_VERSION};

    /// Build the minimal valid 9-byte TBF buffer:
    ///   [magic 4B][version 1B][flags 1B][reserved 2B][dict-count varint 0x00]
    fn minimal_valid_header() -> Vec<u8> {
        let mut buf = Vec::with_capacity(9);
        buf.extend_from_slice(&TBF_MAGIC); // bytes 0-3: magic
        buf.push(TBF_VERSION); // byte  4:   version
        buf.push(0x00); // byte  5:   flags
        buf.push(0x00); // byte  6:   reserved lo
        buf.push(0x00); // byte  7:   reserved hi
        buf.push(0x00); // byte  8:   dictionary count = 0 (varint)
        buf
    }

    // -----------------------------------------------------------------------
    // Empty slice
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_slice_is_error() {
        let result = TbfDeserializer::new(&[]);
        assert!(result.is_err(), "empty slice must be rejected");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.contains("too short") || msg.contains("short"),
            "error should mention data length: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // Too-short slice (4 bytes — has magic length but no version byte)
    // -----------------------------------------------------------------------

    #[test]
    fn test_four_byte_slice_is_error() {
        // Four bytes is the magic length but the header requires at least 8.
        let data = &TBF_MAGIC[..];
        let result = TbfDeserializer::new(data);
        assert!(
            result.is_err(),
            "4-byte slice must be rejected as too short"
        );
    }

    // -----------------------------------------------------------------------
    // Wrong magic bytes
    // -----------------------------------------------------------------------

    #[test]
    fn test_wrong_magic_is_error() {
        let mut data = minimal_valid_header();
        // Corrupt the first magic byte.
        data[0] = 0x00;
        let result = TbfDeserializer::new(&data);
        assert!(result.is_err(), "wrong magic bytes must be rejected");
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.to_lowercase().contains("magic"),
            "error should mention magic bytes: {msg}"
        );
    }

    #[test]
    fn test_all_zero_magic_is_error() {
        let mut data = minimal_valid_header();
        data[0] = 0x00;
        data[1] = 0x00;
        data[2] = 0x00;
        data[3] = 0x00;
        let result = TbfDeserializer::new(&data);
        assert!(result.is_err(), "all-zero magic must be rejected");
    }

    // -----------------------------------------------------------------------
    // Unsupported version (0xFF > TBF_VERSION)
    // -----------------------------------------------------------------------

    #[test]
    fn test_version_0xff_is_error() {
        let mut data = minimal_valid_header();
        data[4] = 0xFF;
        let result = TbfDeserializer::new(&data);
        assert!(
            result.is_err(),
            "version 0xFF must be rejected as unsupported"
        );
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.to_lowercase().contains("version") || msg.to_lowercase().contains("unsupported"),
            "error should mention version: {msg}"
        );
    }

    #[test]
    fn test_version_just_above_current_is_error() {
        let mut data = minimal_valid_header();
        data[4] = TBF_VERSION + 1;
        let result = TbfDeserializer::new(&data);
        assert!(
            result.is_err(),
            "version {} must be rejected",
            TBF_VERSION + 1
        );
    }

    // -----------------------------------------------------------------------
    // Current version is accepted
    // -----------------------------------------------------------------------

    #[test]
    fn test_current_version_is_accepted() {
        let data = minimal_valid_header();
        let result = TbfDeserializer::new(&data);
        assert!(
            result.is_ok(),
            "current TBF_VERSION ({}) must be accepted",
            TBF_VERSION
        );
    }

    #[test]
    fn test_version_zero_is_accepted() {
        // Version 0 is <= TBF_VERSION so the check `version > TBF_VERSION`
        // passes.
        let mut data = minimal_valid_header();
        data[4] = 0x00;
        let result = TbfDeserializer::new(&data);
        assert!(result.is_ok(), "version 0 should be accepted");
    }

    // -----------------------------------------------------------------------
    // Valid header + empty dictionary + invalid type tag byte
    // -----------------------------------------------------------------------

    #[test]
    fn test_invalid_type_tag_after_valid_header_is_error() {
        // Build a valid 9-byte header (magic + version + flags + reserved +
        // empty-dictionary varint), then append an invalid type tag byte.
        // TypeTag::from_u8 only recognises 0..=23; 0xFF has no mapping.
        let mut data = minimal_valid_header();
        data.push(0xFF); // byte 9: invalid type tag

        // Constructing the deserializer must succeed (header is valid)…
        let mut de =
            TbfDeserializer::new(&data).expect("header is valid; construction should succeed");

        // …but reading the tag must fail.
        let result = de.read_tag();
        assert!(
            result.is_err(),
            "invalid type tag byte 0xFF must produce an error"
        );
        let msg = result.err().unwrap().to_string();
        assert!(
            msg.to_lowercase().contains("tag") || msg.to_lowercase().contains("invalid"),
            "error should mention invalid tag: {msg}"
        );
    }

    #[test]
    fn test_read_tag_on_empty_body_is_error() {
        // A valid header with no data section at all — read_tag should return
        // an "unexpected end of data" error rather than panicking.
        let data = minimal_valid_header();
        let mut de = TbfDeserializer::new(&data).expect("valid header must be accepted");

        let result = de.read_tag();
        assert!(result.is_err(), "reading a tag past end of data must fail");
    }

    // -----------------------------------------------------------------------
    // State accessors
    // -----------------------------------------------------------------------

    #[test]
    fn test_new_deserializer_position_after_header() {
        let data = minimal_valid_header();
        let de = TbfDeserializer::new(&data).unwrap();
        // After the 8-byte header + 1-byte empty-dictionary varint, pos == 9.
        assert_eq!(de.position(), 9);
    }

    #[test]
    fn test_new_deserializer_is_empty_when_no_data_section() {
        let data = minimal_valid_header();
        let de = TbfDeserializer::new(&data).unwrap();
        assert!(
            de.is_empty(),
            "no data section means is_empty() should be true"
        );
    }
}
