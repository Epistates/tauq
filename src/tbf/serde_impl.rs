//! Serde trait implementations for TBF
//!
//! This module implements the serde Serializer and Deserializer traits
//! for direct, high-performance binary serialization.

use super::encoder::TbfSerializer;
use super::decoder::{TbfDeserializer, SeqAccess, MapAccess, EnumAccess};
use super::varint::*;
use super::TypeTag;
use crate::error::{InterpretError, TauqError};
use serde::de::{self, Visitor};
use serde::ser;

// ============================================================================
// Error Implementation
// ============================================================================

impl ser::Error for TauqError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        TauqError::Interpret(InterpretError::new(msg.to_string()))
    }
}

impl de::Error for TauqError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        TauqError::Interpret(InterpretError::new(msg.to_string()))
    }
}

// ============================================================================
// Serializer Implementation
// ============================================================================

impl<'a> ser::Serializer for &'a mut TbfSerializer {
    type Ok = ();
    type Error = TauqError;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::Bool);
        self.buf.push(if v { 1 } else { 0 });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::I8);
        self.buf.push(v as u8);
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::I16);
        self.write_signed_varint(v as i64);
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::I32);
        self.write_signed_varint(v as i64);
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::I64);
        self.write_signed_varint(v);
        Ok(())
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::I128);
        encode_i128_varint(v, &mut self.buf);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::U8);
        self.buf.push(v);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::U16);
        self.write_varint(v as u64);
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::U32);
        self.write_varint(v as u64);
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::U64);
        self.write_varint(v);
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::U128);
        encode_u128_varint(v, &mut self.buf);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::F32);
        self.buf.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::F64);
        self.buf.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::Char);
        self.write_varint(v as u64);
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::String);
        self.write_string(v);
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::Bytes);
        self.write_varint(v.len() as u64);
        self.buf.extend_from_slice(v);
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::None);
        Ok(())
    }

    fn serialize_some<T: ?Sized + serde::Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::Some);
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.write_tag(TypeTag::Unit);
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        // Encode variant index only (more compact than string)
        self.write_varint(variant_index as u64);
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized + serde::Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + serde::Serialize>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.write_varint(variant_index as u64);
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.write_tag(TypeTag::Seq);
        self.write_varint(len.unwrap_or(0) as u64);
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.write_tag(TypeTag::Seq);
        self.write_varint(len as u64);
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.write_varint(variant_index as u64);
        self.write_tag(TypeTag::Seq);
        self.write_varint(len as u64);
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.write_tag(TypeTag::Map);
        self.write_varint(len.unwrap_or(0) as u64);
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.write_tag(TypeTag::Map);
        self.write_varint(len as u64);
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.write_varint(variant_index as u64);
        self.write_tag(TypeTag::Map);
        self.write_varint(len as u64);
        Ok(self)
    }
}

impl<'a> ser::SerializeSeq for &'a mut TbfSerializer {
    type Ok = ();
    type Error = TauqError;

    fn serialize_element<T: ?Sized + serde::Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut TbfSerializer {
    type Ok = ();
    type Error = TauqError;

    fn serialize_element<T: ?Sized + serde::Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut TbfSerializer {
    type Ok = ();
    type Error = TauqError;

    fn serialize_field<T: ?Sized + serde::Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut TbfSerializer {
    type Ok = ();
    type Error = TauqError;

    fn serialize_field<T: ?Sized + serde::Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut TbfSerializer {
    type Ok = ();
    type Error = TauqError;

    fn serialize_key<T: ?Sized + serde::Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        key.serialize(&mut **self)
    }

    fn serialize_value<T: ?Sized + serde::Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut TbfSerializer {
    type Ok = ();
    type Error = TauqError;

    fn serialize_field<T: ?Sized + serde::Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        // Write key as string
        self.write_tag(TypeTag::String);
        self.write_string(key);
        // Write value
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut TbfSerializer {
    type Ok = ();
    type Error = TauqError;

    fn serialize_field<T: ?Sized + serde::Serialize>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error> {
        self.write_tag(TypeTag::String);
        self.write_string(key);
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

// ============================================================================
// Deserializer Implementation
// ============================================================================

impl<'de> de::Deserializer<'de> for &mut TbfDeserializer<'de> {
    type Error = TauqError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::Null => visitor.visit_unit(),
            TypeTag::Bool => visitor.visit_bool(self.read_byte()? != 0),
            TypeTag::Int => visitor.visit_i64(self.read_signed_varint()?),
            TypeTag::Float => visitor.visit_f64(self.read_f64()?),
            TypeTag::String => visitor.visit_borrowed_str(self.read_string()?),
            TypeTag::Bytes => {
                let len = self.read_varint()? as usize;
                visitor.visit_borrowed_bytes(self.read_bytes(len)?)
            }
            TypeTag::Seq => {
                let len = self.read_varint()? as usize;
                visitor.visit_seq(SeqAccess::new(self, len))
            }
            TypeTag::Map => {
                let len = self.read_varint()? as usize;
                visitor.visit_map(MapAccess::new(self, len))
            }
            TypeTag::Unit => visitor.visit_unit(),
            TypeTag::None => visitor.visit_none(),
            TypeTag::Some => visitor.visit_some(self),
            TypeTag::I8 => visitor.visit_i8(self.read_byte()? as i8),
            TypeTag::I16 => visitor.visit_i16(self.read_signed_varint()? as i16),
            TypeTag::I32 => visitor.visit_i32(self.read_signed_varint()? as i32),
            TypeTag::I64 => visitor.visit_i64(self.read_signed_varint()?),
            TypeTag::I128 => visitor.visit_i128(self.read_i128_varint()?),
            TypeTag::U8 => visitor.visit_u8(self.read_byte()?),
            TypeTag::U16 => visitor.visit_u16(self.read_varint()? as u16),
            TypeTag::U32 => visitor.visit_u32(self.read_varint()? as u32),
            TypeTag::U64 => visitor.visit_u64(self.read_varint()?),
            TypeTag::U128 => visitor.visit_u128(self.read_u128_varint()?),
            TypeTag::F32 => visitor.visit_f32(self.read_f32()?),
            TypeTag::F64 => visitor.visit_f64(self.read_f64()?),
            TypeTag::Char => visitor.visit_char(char::from_u32(self.read_varint()? as u32).unwrap_or('\0')),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        if tag != TypeTag::Bool {
            return Err(TauqError::Interpret(InterpretError::new(
                format!("Expected bool, got {:?}", tag),
            )));
        }
        visitor.visit_bool(self.read_byte()? != 0)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::I8 => visitor.visit_i8(self.read_byte()? as i8),
            TypeTag::I16 | TypeTag::I32 | TypeTag::I64 | TypeTag::Int => {
                visitor.visit_i8(self.read_signed_varint()? as i8)
            }
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected i8, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::I8 => visitor.visit_i16(self.read_byte()? as i8 as i16),
            TypeTag::I16 | TypeTag::I32 | TypeTag::I64 | TypeTag::Int => {
                visitor.visit_i16(self.read_signed_varint()? as i16)
            }
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected i16, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::I8 => visitor.visit_i32(self.read_byte()? as i8 as i32),
            TypeTag::I16 | TypeTag::I32 | TypeTag::I64 | TypeTag::Int => {
                visitor.visit_i32(self.read_signed_varint()? as i32)
            }
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected i32, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::I8 => visitor.visit_i64(self.read_byte()? as i8 as i64),
            TypeTag::I16 | TypeTag::I32 | TypeTag::I64 | TypeTag::Int => {
                visitor.visit_i64(self.read_signed_varint()?)
            }
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected i64, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::I128 => visitor.visit_i128(self.read_i128_varint()?),
            TypeTag::I8 => visitor.visit_i128(self.read_byte()? as i8 as i128),
            TypeTag::I16 | TypeTag::I32 | TypeTag::I64 | TypeTag::Int => {
                visitor.visit_i128(self.read_signed_varint()? as i128)
            }
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected i128, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::U8 => visitor.visit_u8(self.read_byte()?),
            TypeTag::U16 | TypeTag::U32 | TypeTag::U64 => {
                visitor.visit_u8(self.read_varint()? as u8)
            }
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected u8, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::U8 => visitor.visit_u16(self.read_byte()? as u16),
            TypeTag::U16 | TypeTag::U32 | TypeTag::U64 => {
                visitor.visit_u16(self.read_varint()? as u16)
            }
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected u16, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::U8 => visitor.visit_u32(self.read_byte()? as u32),
            TypeTag::U16 | TypeTag::U32 | TypeTag::U64 => {
                visitor.visit_u32(self.read_varint()? as u32)
            }
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected u32, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::U8 => visitor.visit_u64(self.read_byte()? as u64),
            TypeTag::U16 | TypeTag::U32 | TypeTag::U64 => {
                visitor.visit_u64(self.read_varint()?)
            }
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected u64, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::U128 => visitor.visit_u128(self.read_u128_varint()?),
            TypeTag::U8 => visitor.visit_u128(self.read_byte()? as u128),
            TypeTag::U16 | TypeTag::U32 | TypeTag::U64 => {
                visitor.visit_u128(self.read_varint()? as u128)
            }
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected u128, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::F32 => visitor.visit_f32(self.read_f32()?),
            TypeTag::F64 | TypeTag::Float => visitor.visit_f32(self.read_f64()? as f32),
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected f32, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::F32 => visitor.visit_f64(self.read_f32()? as f64),
            TypeTag::F64 | TypeTag::Float => visitor.visit_f64(self.read_f64()?),
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected f64, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        if tag != TypeTag::Char {
            return Err(TauqError::Interpret(InterpretError::new(
                format!("Expected char, got {:?}", tag),
            )));
        }
        let code = self.read_varint()? as u32;
        visitor.visit_char(char::from_u32(code).unwrap_or('\0'))
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        if tag != TypeTag::String {
            return Err(TauqError::Interpret(InterpretError::new(
                format!("Expected string, got {:?}", tag),
            )));
        }
        visitor.visit_borrowed_str(self.read_string()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        if tag != TypeTag::Bytes {
            return Err(TauqError::Interpret(InterpretError::new(
                format!("Expected bytes, got {:?}", tag),
            )));
        }
        let len = self.read_varint()? as usize;
        visitor.visit_borrowed_bytes(self.read_bytes(len)?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        match tag {
            TypeTag::None | TypeTag::Null => visitor.visit_none(),
            TypeTag::Some => visitor.visit_some(self),
            _ => Err(TauqError::Interpret(InterpretError::new(
                format!("Expected option, got {:?}", tag),
            ))),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        if tag != TypeTag::Unit && tag != TypeTag::Null {
            return Err(TauqError::Interpret(InterpretError::new(
                format!("Expected unit, got {:?}", tag),
            )));
        }
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        if tag != TypeTag::Seq {
            return Err(TauqError::Interpret(InterpretError::new(
                format!("Expected seq, got {:?}", tag),
            )));
        }
        let len = self.read_varint()? as usize;
        visitor.visit_seq(SeqAccess::new(self, len))
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_tag()?;
        if tag != TypeTag::Map {
            return Err(TauqError::Interpret(InterpretError::new(
                format!("Expected map, got {:?}", tag),
            )));
        }
        let len = self.read_varint()? as usize;
        visitor.visit_map(MapAccess::new(self, len))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        // Read variant index
        let variant_idx = self.read_varint()? as u32;
        // For simplicity, just visit the variant string
        let variant = _variants.get(variant_idx as usize)
            .ok_or_else(|| TauqError::Interpret(InterpretError::new(
                format!("Invalid variant index: {}", variant_idx)
            )))?;
        visitor.visit_enum(EnumAccess { de: self, variant })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}
