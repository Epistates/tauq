//! Arrow to TBF conversion utilities
//!
//! Converts Arrow schemas and data to TBF format.

use arrow_array::{
    Array, ArrayRef, BooleanArray, Float32Array, Float64Array, Int8Array, Int16Array, Int32Array,
    Int64Array, RecordBatch, StringArray, UInt8Array, UInt16Array, UInt32Array, UInt64Array,
};
use arrow_schema::{DataType, Schema as ArrowSchema};
use iceberg::spec::{PrimitiveType, Schema as IcebergSchema, Type};

use crate::tbf::{
    AdaptiveIntEncoder, AdaptiveStringEncoder, FieldEncoding, TableSchema, TableSchemaBuilder,
    UltraBuffer, encode_varint_fast, SCHEMA_MAGIC,
};

/// Convert Arrow schema to TBF TableSchema
pub fn arrow_schema_to_tbf(arrow_schema: &ArrowSchema) -> TableSchema {
    let mut builder = TableSchemaBuilder::new();

    for field in arrow_schema.fields() {
        let encoding = arrow_type_to_encoding(field.data_type());
        builder = builder.column(field.name(), encoding);
    }

    builder.build()
}

/// Convert Iceberg schema to TBF TableSchema
pub fn iceberg_schema_to_tbf(iceberg_schema: &IcebergSchema) -> TableSchema {
    let mut builder = TableSchemaBuilder::new();

    for field in iceberg_schema.as_struct().fields() {
        let encoding = iceberg_type_to_encoding(&field.field_type);
        builder = builder.column(&field.name, encoding);
    }

    builder.build()
}

/// Convert Arrow DataType to TBF FieldEncoding
fn arrow_type_to_encoding(dt: &DataType) -> FieldEncoding {
    match dt {
        DataType::Boolean => FieldEncoding::Bool,
        DataType::Int8 => FieldEncoding::I8,
        DataType::Int16 => FieldEncoding::I16,
        DataType::Int32 | DataType::Date32 => FieldEncoding::I32,
        DataType::Int64 | DataType::Date64 | DataType::Timestamp(_, _) | DataType::Time64(_) => {
            FieldEncoding::I64
        }
        DataType::UInt8 => FieldEncoding::U8,
        DataType::UInt16 => FieldEncoding::U16,
        DataType::UInt32 => FieldEncoding::U32,
        DataType::UInt64 => FieldEncoding::U64,
        DataType::Float32 => FieldEncoding::Float32,
        DataType::Float64 => FieldEncoding::Float64,
        DataType::Utf8 | DataType::LargeUtf8 => FieldEncoding::Dictionary,
        _ => FieldEncoding::Auto,
    }
}

/// Convert Iceberg type to TBF FieldEncoding
fn iceberg_type_to_encoding(ty: &Type) -> FieldEncoding {
    match ty {
        Type::Primitive(prim) => match prim {
            PrimitiveType::Boolean => FieldEncoding::Bool,
            PrimitiveType::Int => FieldEncoding::I32,
            PrimitiveType::Long => FieldEncoding::I64,
            PrimitiveType::Float => FieldEncoding::Float32,
            PrimitiveType::Double => FieldEncoding::Float64,
            PrimitiveType::String => FieldEncoding::Dictionary,
            PrimitiveType::Binary | PrimitiveType::Fixed(_) => FieldEncoding::Inline,
            PrimitiveType::Date => FieldEncoding::I32,
            PrimitiveType::Time => FieldEncoding::I64,
            PrimitiveType::Timestamp | PrimitiveType::Timestamptz |
            PrimitiveType::TimestampNs | PrimitiveType::TimestamptzNs => FieldEncoding::I64,
            PrimitiveType::Decimal { .. } => FieldEncoding::VarInt,
            PrimitiveType::Uuid => FieldEncoding::Inline,
        },
        Type::Struct(_) | Type::List(_) | Type::Map(_) => FieldEncoding::Auto,
    }
}

/// Trait for converting Arrow arrays to TBF columnar data
pub trait ArrowToTbf {
    /// Encode Arrow RecordBatch to TBF bytes
    fn encode_to_tbf(&self, schema: &TableSchema) -> Vec<u8>;
}

impl ArrowToTbf for RecordBatch {
    fn encode_to_tbf(&self, schema: &TableSchema) -> Vec<u8> {
        let n = self.num_rows();
        if n == 0 {
            let mut buf = UltraBuffer::with_capacity(16);
            buf.extend(&SCHEMA_MAGIC);
            buf.push(1);
            encode_varint_fast(0, &mut buf);
            return buf.into_vec();
        }

        let num_cols = self.num_columns();
        let mut encoders: Vec<ColumnEncoder> = Vec::with_capacity(num_cols);

        // Create encoders for each column based on schema
        for (i, col) in self.columns().iter().enumerate() {
            let encoding = schema.encoding(i).unwrap_or(FieldEncoding::Auto);
            encoders.push(ColumnEncoder::new(col.clone(), encoding, n));
        }

        // Encode all data
        for encoder in &mut encoders {
            encoder.collect_data();
        }

        // Estimate output size
        let estimated = n * num_cols * 8 + 512;
        let mut buf = UltraBuffer::with_capacity(estimated);

        // Header
        buf.extend(&SCHEMA_MAGIC);
        buf.push(1); // version
        encode_varint_fast(n as u64, &mut buf);
        encode_varint_fast(num_cols as u64, &mut buf);

        // Encode all columns
        for encoder in encoders {
            encoder.encode_to(&mut buf);
        }

        buf.into_vec()
    }
}

/// Column encoder that handles different Arrow array types
enum ColumnEncoder {
    Int(AdaptiveIntEncoder, ArrayRef),
    String(AdaptiveStringEncoder, ArrayRef),
    Bool(Vec<bool>, ArrayRef),
    Float32(Vec<f32>, ArrayRef),
    Float64(Vec<f64>, ArrayRef),
}

impl ColumnEncoder {
    fn new(array: ArrayRef, encoding: FieldEncoding, capacity: usize) -> Self {
        match array.data_type() {
            DataType::Boolean => ColumnEncoder::Bool(Vec::with_capacity(capacity), array),
            DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 |
            DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 |
            DataType::Date32 | DataType::Date64 | DataType::Time64(_) |
            DataType::Timestamp(_, _) => {
                ColumnEncoder::Int(AdaptiveIntEncoder::new(encoding, capacity), array)
            }
            DataType::Float32 => ColumnEncoder::Float32(Vec::with_capacity(capacity), array),
            DataType::Float64 => ColumnEncoder::Float64(Vec::with_capacity(capacity), array),
            DataType::Utf8 | DataType::LargeUtf8 => {
                ColumnEncoder::String(AdaptiveStringEncoder::new(encoding, capacity), array)
            }
            _ => {
                ColumnEncoder::String(AdaptiveStringEncoder::new(FieldEncoding::Inline, capacity), array)
            }
        }
    }

    fn collect_data(&mut self) {
        match self {
            ColumnEncoder::Int(enc, array) => collect_int_data(enc, array),
            ColumnEncoder::String(enc, array) => collect_string_data(enc, array),
            ColumnEncoder::Bool(vec, array) => {
                if let Some(arr) = array.as_any().downcast_ref::<BooleanArray>() {
                    for i in 0..arr.len() {
                        vec.push(arr.value(i));
                    }
                }
            }
            ColumnEncoder::Float32(vec, array) => {
                if let Some(arr) = array.as_any().downcast_ref::<Float32Array>() {
                    for i in 0..arr.len() {
                        vec.push(arr.value(i));
                    }
                }
            }
            ColumnEncoder::Float64(vec, array) => {
                if let Some(arr) = array.as_any().downcast_ref::<Float64Array>() {
                    for i in 0..arr.len() {
                        vec.push(arr.value(i));
                    }
                }
            }
        }
    }

    fn encode_to(self, buf: &mut UltraBuffer) {
        match self {
            ColumnEncoder::Int(enc, _) => enc.encode_to(buf),
            ColumnEncoder::String(enc, _) => enc.encode_to(buf),
            ColumnEncoder::Bool(vec, _) => encode_bool_column(&vec, buf),
            ColumnEncoder::Float32(vec, _) => encode_f32_column(&vec, buf),
            ColumnEncoder::Float64(vec, _) => encode_f64_column(&vec, buf),
        }
    }
}

fn collect_int_data(enc: &mut AdaptiveIntEncoder, array: &ArrayRef) {
    match array.data_type() {
        DataType::Int8 => {
            if let Some(arr) = array.as_any().downcast_ref::<Int8Array>() {
                for i in 0..arr.len() {
                    enc.push(arr.value(i) as i64);
                }
            }
        }
        DataType::Int16 => {
            if let Some(arr) = array.as_any().downcast_ref::<Int16Array>() {
                for i in 0..arr.len() {
                    enc.push(arr.value(i) as i64);
                }
            }
        }
        DataType::Int32 | DataType::Date32 => {
            if let Some(arr) = array.as_any().downcast_ref::<Int32Array>() {
                for i in 0..arr.len() {
                    enc.push(arr.value(i) as i64);
                }
            }
        }
        DataType::Int64 | DataType::Date64 | DataType::Time64(_) | DataType::Timestamp(_, _) => {
            if let Some(arr) = array.as_any().downcast_ref::<Int64Array>() {
                for i in 0..arr.len() {
                    enc.push(arr.value(i));
                }
            }
        }
        DataType::UInt8 => {
            if let Some(arr) = array.as_any().downcast_ref::<UInt8Array>() {
                for i in 0..arr.len() {
                    enc.push(arr.value(i) as i64);
                }
            }
        }
        DataType::UInt16 => {
            if let Some(arr) = array.as_any().downcast_ref::<UInt16Array>() {
                for i in 0..arr.len() {
                    enc.push(arr.value(i) as i64);
                }
            }
        }
        DataType::UInt32 => {
            if let Some(arr) = array.as_any().downcast_ref::<UInt32Array>() {
                for i in 0..arr.len() {
                    enc.push(arr.value(i) as i64);
                }
            }
        }
        DataType::UInt64 => {
            if let Some(arr) = array.as_any().downcast_ref::<UInt64Array>() {
                for i in 0..arr.len() {
                    enc.push(arr.value(i) as i64);
                }
            }
        }
        _ => {}
    }
}

fn collect_string_data(enc: &mut AdaptiveStringEncoder, array: &ArrayRef) {
    if let Some(arr) = array.as_any().downcast_ref::<StringArray>() {
        for i in 0..arr.len() {
            enc.push(arr.value(i));
        }
    }
}

fn encode_bool_column(values: &[bool], buf: &mut UltraBuffer) {
    let n = values.len();
    encode_varint_fast(0, buf); // column type: bool
    encode_varint_fast(n as u64, buf);

    let num_bytes = n.div_ceil(8);
    for byte_idx in 0..num_bytes {
        let mut byte = 0u8;
        for bit_idx in 0..8 {
            let val_idx = byte_idx * 8 + bit_idx;
            if val_idx < n && values[val_idx] {
                byte |= 1 << bit_idx;
            }
        }
        buf.push(byte);
    }
}

fn encode_f32_column(values: &[f32], buf: &mut UltraBuffer) {
    encode_varint_fast(1, buf); // column type: f32
    encode_varint_fast(values.len() as u64, buf);
    for &v in values {
        buf.extend(&v.to_le_bytes());
    }
}

fn encode_f64_column(values: &[f64], buf: &mut UltraBuffer) {
    encode_varint_fast(2, buf); // column type: f64
    encode_varint_fast(values.len() as u64, buf);
    for &v in values {
        buf.extend(&v.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_schema::Field;
    use std::sync::Arc;

    #[test]
    fn test_encode_simple_batch() {
        let schema = Arc::new(ArrowSchema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("value", DataType::Float64, false),
        ]));

        let id_array = Int32Array::from(vec![1, 2, 3]);
        let name_array = StringArray::from(vec!["Alice", "Bob", "Carol"]);
        let value_array = Float64Array::from(vec![1.0, 2.0, 3.0]);

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array),
                Arc::new(name_array),
                Arc::new(value_array),
            ],
        )
        .unwrap();

        let tbf_schema = arrow_schema_to_tbf(&schema);
        let bytes = batch.encode_to_tbf(&tbf_schema);

        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..4], &SCHEMA_MAGIC);
    }
}
