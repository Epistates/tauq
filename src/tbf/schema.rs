//! Schema definitions for TBF
//!
//! Schemas enable type-tag-free encoding by defining the structure upfront.
//! This achieves significant size reduction for homogeneous data.

use super::varint::{encode_varint, decode_varint};
use super::dictionary::StringDictionary;
use crate::error::{InterpretError, TauqError};

/// Field type tag for schema definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SchemaType {
    /// Boolean
    Bool = 1,
    /// Signed integer (varint encoded)
    Int = 2,
    /// Unsigned integer (varint encoded)
    UInt = 3,
    /// 32-bit float
    F32 = 4,
    /// 64-bit float
    F64 = 5,
    /// String (dictionary indexed)
    String = 6,
    /// Raw bytes
    Bytes = 7,
    /// Optional value (None/Some)
    Option = 8,
    /// Sequence of values
    Seq = 9,
    /// Map/struct
    Map = 10,
    /// Nested schema reference
    SchemaRef = 11,
}

impl SchemaType {
    /// Convert from u8
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(SchemaType::Bool),
            2 => Some(SchemaType::Int),
            3 => Some(SchemaType::UInt),
            4 => Some(SchemaType::F32),
            5 => Some(SchemaType::F64),
            6 => Some(SchemaType::String),
            7 => Some(SchemaType::Bytes),
            8 => Some(SchemaType::Option),
            9 => Some(SchemaType::Seq),
            10 => Some(SchemaType::Map),
            11 => Some(SchemaType::SchemaRef),
            _ => None,
        }
    }
}

/// A field in a schema
#[derive(Debug, Clone)]
pub struct SchemaField {
    /// Field name (dictionary index when encoded)
    pub name: String,
    /// Field type
    pub typ: SchemaType,
    /// For Option/Seq: inner type
    pub inner_type: Option<SchemaType>,
    /// For SchemaRef: schema index
    pub schema_ref: Option<u32>,
}

impl SchemaField {
    /// Create a simple field
    pub fn new(name: impl Into<String>, typ: SchemaType) -> Self {
        Self {
            name: name.into(),
            typ,
            inner_type: None,
            schema_ref: None,
        }
    }

    /// Create an optional field
    pub fn optional(name: impl Into<String>, inner: SchemaType) -> Self {
        Self {
            name: name.into(),
            typ: SchemaType::Option,
            inner_type: Some(inner),
            schema_ref: None,
        }
    }

    /// Create a sequence field
    pub fn seq(name: impl Into<String>, inner: SchemaType) -> Self {
        Self {
            name: name.into(),
            typ: SchemaType::Seq,
            inner_type: Some(inner),
            schema_ref: None,
        }
    }

    /// Create a nested schema reference
    pub fn schema_ref(name: impl Into<String>, schema_idx: u32) -> Self {
        Self {
            name: name.into(),
            typ: SchemaType::SchemaRef,
            inner_type: None,
            schema_ref: Some(schema_idx),
        }
    }
}

/// Schema definition for a struct/record type
#[derive(Debug, Clone)]
pub struct Schema {
    /// Schema name (e.g., "Employee")
    pub name: String,
    /// Fields in order
    pub fields: Vec<SchemaField>,
}

impl Schema {
    /// Create a new schema
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
        }
    }

    /// Add a field
    pub fn field(mut self, field: SchemaField) -> Self {
        self.fields.push(field);
        self
    }

    /// Add a simple typed field
    pub fn add_field(&mut self, name: impl Into<String>, typ: SchemaType) {
        self.fields.push(SchemaField::new(name, typ));
    }

    /// Encode schema to bytes (dictionary indices for strings)
    pub fn encode(&self, buf: &mut Vec<u8>, dict: &mut StringDictionary) {
        // Schema name
        let name_idx = dict.intern(&self.name);
        encode_varint(name_idx as u64, buf);

        // Field count
        encode_varint(self.fields.len() as u64, buf);

        // Fields
        for field in &self.fields {
            // Field name
            let field_idx = dict.intern(&field.name);
            encode_varint(field_idx as u64, buf);

            // Field type
            buf.push(field.typ as u8);

            // Inner type if applicable
            if field.typ == SchemaType::Option || field.typ == SchemaType::Seq {
                buf.push(field.inner_type.unwrap_or(SchemaType::Int) as u8);
            }

            // Schema ref if applicable
            if field.typ == SchemaType::SchemaRef {
                encode_varint(field.schema_ref.unwrap_or(0) as u64, buf);
            }
        }
    }

    /// Decode schema from bytes
    pub fn decode(bytes: &[u8], dict: &super::dictionary::BorrowedDictionary) -> Result<(Self, usize), TauqError> {
        let mut pos = 0;

        // Schema name
        let (name_idx, len) = decode_varint(bytes)?;
        pos += len;
        let name = dict.get(name_idx as u32)
            .ok_or_else(|| TauqError::Interpret(InterpretError::new("Invalid schema name index")))?
            .to_string();

        // Field count
        let (field_count, len) = decode_varint(&bytes[pos..])?;
        pos += len;

        let mut fields = Vec::with_capacity(field_count as usize);

        for _ in 0..field_count {
            // Field name
            let (field_idx, len) = decode_varint(&bytes[pos..])?;
            pos += len;
            let field_name = dict.get(field_idx as u32)
                .ok_or_else(|| TauqError::Interpret(InterpretError::new("Invalid field name index")))?
                .to_string();

            // Field type
            let typ = SchemaType::from_u8(bytes[pos])
                .ok_or_else(|| TauqError::Interpret(InterpretError::new("Invalid schema type")))?;
            pos += 1;

            let mut field = SchemaField::new(field_name, typ);

            // Inner type if applicable
            if typ == SchemaType::Option || typ == SchemaType::Seq {
                field.inner_type = SchemaType::from_u8(bytes[pos]);
                pos += 1;
            }

            // Schema ref if applicable
            if typ == SchemaType::SchemaRef {
                let (ref_idx, len) = decode_varint(&bytes[pos..])?;
                pos += len;
                field.schema_ref = Some(ref_idx as u32);
            }

            fields.push(field);
        }

        Ok((Schema { name, fields }, pos))
    }
}

/// Schema registry for encoding/decoding
#[derive(Debug, Default)]
pub struct SchemaRegistry {
    schemas: Vec<Schema>,
}

impl SchemaRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a schema and return its index
    pub fn register(&mut self, schema: Schema) -> u32 {
        let idx = self.schemas.len() as u32;
        self.schemas.push(schema);
        idx
    }

    /// Get a schema by index
    pub fn get(&self, idx: u32) -> Option<&Schema> {
        self.schemas.get(idx as usize)
    }

    /// Number of schemas
    pub fn len(&self) -> usize {
        self.schemas.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }

    /// Encode all schemas
    pub fn encode(&self, buf: &mut Vec<u8>, dict: &mut StringDictionary) {
        encode_varint(self.schemas.len() as u64, buf);
        for schema in &self.schemas {
            schema.encode(buf, dict);
        }
    }

    /// Decode schemas from bytes
    pub fn decode(bytes: &[u8], dict: &super::dictionary::BorrowedDictionary) -> Result<(Self, usize), TauqError> {
        let mut pos = 0;

        let (count, len) = decode_varint(bytes)?;
        pos += len;

        let mut registry = Self::new();
        for _ in 0..count {
            let (schema, len) = Schema::decode(&bytes[pos..], dict)?;
            pos += len;
            registry.register(schema);
        }

        Ok((registry, pos))
    }
}

/// Infer schema from a serde_json::Value
pub fn infer_schema_from_json(value: &serde_json::Value, name: &str) -> Option<Schema> {
    match value {
        serde_json::Value::Array(arr) => {
            // Check if it's a homogeneous array of objects
            if let Some(serde_json::Value::Object(first)) = arr.first() {
                // Verify all items have same structure
                let first_keys: Vec<&String> = first.keys().collect();
                let all_same = arr.iter().all(|item| {
                    if let serde_json::Value::Object(obj) = item {
                        let keys: Vec<&String> = obj.keys().collect();
                        keys == first_keys
                    } else {
                        false
                    }
                });

                if all_same {
                    let mut schema = Schema::new(name);
                    for (key, value) in first {
                        let typ = json_value_to_schema_type(value);
                        schema.add_field(key, typ);
                    }
                    return Some(schema);
                }
            }
            None
        }
        serde_json::Value::Object(obj) => {
            let mut schema = Schema::new(name);
            for (key, value) in obj {
                let typ = json_value_to_schema_type(value);
                schema.add_field(key, typ);
            }
            Some(schema)
        }
        _ => None,
    }
}

/// Convert JSON value type to schema type
fn json_value_to_schema_type(value: &serde_json::Value) -> SchemaType {
    match value {
        serde_json::Value::Null => SchemaType::Option,
        serde_json::Value::Bool(_) => SchemaType::Bool,
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                SchemaType::Int
            } else if n.is_u64() {
                SchemaType::UInt
            } else {
                SchemaType::F64
            }
        }
        serde_json::Value::String(_) => SchemaType::String,
        serde_json::Value::Array(_) => SchemaType::Seq,
        serde_json::Value::Object(_) => SchemaType::Map,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let schema = Schema::new("Employee")
            .field(SchemaField::new("id", SchemaType::UInt))
            .field(SchemaField::new("name", SchemaType::String))
            .field(SchemaField::new("age", SchemaType::UInt))
            .field(SchemaField::optional("email", SchemaType::String));

        assert_eq!(schema.name, "Employee");
        assert_eq!(schema.fields.len(), 4);
        assert_eq!(schema.fields[0].name, "id");
        assert_eq!(schema.fields[3].typ, SchemaType::Option);
    }

    #[test]
    fn test_schema_roundtrip() {
        let schema = Schema::new("User")
            .field(SchemaField::new("id", SchemaType::UInt))
            .field(SchemaField::new("name", SchemaType::String));

        let mut dict = StringDictionary::new();
        let mut buf = Vec::new();
        schema.encode(&mut buf, &mut dict);

        // Encode dictionary
        let mut dict_buf = Vec::new();
        dict.encode(&mut dict_buf);

        // Decode
        let (borrowed_dict, _) = super::super::dictionary::BorrowedDictionary::decode(&dict_buf).unwrap();
        let (decoded, _) = Schema::decode(&buf, &borrowed_dict).unwrap();

        assert_eq!(decoded.name, "User");
        assert_eq!(decoded.fields.len(), 2);
        assert_eq!(decoded.fields[0].name, "id");
        assert_eq!(decoded.fields[1].name, "name");
    }

    #[test]
    fn test_infer_schema_from_json() {
        let json = serde_json::json!([
            {"id": 1, "name": "Alice", "active": true},
            {"id": 2, "name": "Bob", "active": false},
        ]);

        let schema = infer_schema_from_json(&json, "Users").unwrap();
        assert_eq!(schema.name, "Users");
        assert_eq!(schema.fields.len(), 3);
    }
}
