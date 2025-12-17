//! TBF Writer utilities for Iceberg integration
//!
//! Provides utilities to write TBF data compatible with Iceberg workflows.
//!
//! # Note
//!
//! This module provides standalone TBF writing capabilities that can be used
//! alongside Iceberg tables. Direct trait implementations for `FileWriter` and
//! `FileWriterBuilder` are planned for a future release once the iceberg-rust
//! API stabilizes.

use arrow_array::RecordBatch;
use iceberg::spec::Schema as IcebergSchema;

use super::arrow_convert::{iceberg_schema_to_tbf, ArrowToTbf};
use crate::tbf::TableSchema;

/// TBF writer configuration
#[derive(Clone)]
pub struct TbfWriterConfig {
    /// TBF schema with encoding hints
    pub tbf_schema: TableSchema,
}

impl TbfWriterConfig {
    /// Create config from Iceberg schema
    pub fn from_iceberg_schema(schema: &IcebergSchema) -> Self {
        Self {
            tbf_schema: iceberg_schema_to_tbf(schema),
        }
    }

    /// Create config with custom TBF schema
    pub fn with_tbf_schema(tbf_schema: TableSchema) -> Self {
        Self { tbf_schema }
    }
}

/// Standalone TBF file writer
///
/// This writer can be used to write TBF data that can later be registered
/// with Iceberg tables.
pub struct TbfFileWriter {
    /// TBF schema
    tbf_schema: TableSchema,
    /// Accumulated data
    buffer: Vec<u8>,
    /// Row count
    row_count: usize,
}

impl TbfFileWriter {
    /// Create a new TBF file writer
    pub fn new(config: TbfWriterConfig) -> Self {
        Self {
            tbf_schema: config.tbf_schema,
            buffer: Vec::new(),
            row_count: 0,
        }
    }

    /// Write a record batch
    pub fn write(&mut self, batch: &RecordBatch) {
        let tbf_bytes = batch.encode_to_tbf(&self.tbf_schema);

        if self.buffer.is_empty() {
            self.buffer = tbf_bytes;
        } else {
            self.buffer.extend_from_slice(&tbf_bytes);
        }

        self.row_count += batch.num_rows();
    }

    /// Get the current row count
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Get the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer.len()
    }

    /// Consume the writer and return the TBF data
    pub fn finish(self) -> TbfFileData {
        TbfFileData {
            data: self.buffer,
            row_count: self.row_count,
        }
    }
}

/// Completed TBF file data
pub struct TbfFileData {
    /// The TBF encoded data
    pub data: Vec<u8>,
    /// Number of rows
    pub row_count: usize,
}

impl TbfFileData {
    /// Get the data as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Consume and return the data
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Get file size in bytes
    pub fn file_size(&self) -> usize {
        self.data.len()
    }
}

/// Builder for TbfFileWriter with fluent API
#[derive(Clone)]
pub struct TbfFileWriterBuilder {
    config: Option<TbfWriterConfig>,
}

impl TbfFileWriterBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { config: None }
    }

    /// Set the Iceberg schema (will derive TBF schema automatically)
    pub fn with_iceberg_schema(mut self, schema: &IcebergSchema) -> Self {
        self.config = Some(TbfWriterConfig::from_iceberg_schema(schema));
        self
    }

    /// Set a custom TBF schema
    pub fn with_tbf_schema(mut self, schema: TableSchema) -> Self {
        self.config = Some(TbfWriterConfig::with_tbf_schema(schema));
        self
    }

    /// Build the writer
    pub fn build(self) -> TbfFileWriter {
        let config = self.config.expect("Schema must be set before building");
        TbfFileWriter::new(config)
    }
}

impl Default for TbfFileWriterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use arrow_array::{Int32Array, StringArray};
    use arrow_schema::{DataType, Field, Schema as ArrowSchema};
    use iceberg::spec::{NestedField, PrimitiveType, Schema, Type};

    #[test]
    fn test_writer_builder() {
        let iceberg_schema = Schema::builder()
            .with_fields(vec![
                Arc::new(NestedField::required(1, "id", Type::Primitive(PrimitiveType::Int))),
                Arc::new(NestedField::required(
                    2,
                    "name",
                    Type::Primitive(PrimitiveType::String),
                )),
            ])
            .build()
            .unwrap();

        let writer = TbfFileWriterBuilder::new()
            .with_iceberg_schema(&iceberg_schema)
            .build();

        assert_eq!(writer.row_count(), 0);
        assert_eq!(writer.buffer_size(), 0);
    }

    #[test]
    fn test_write_and_finish() {
        let arrow_schema = Arc::new(ArrowSchema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        let iceberg_schema = Schema::builder()
            .with_fields(vec![
                Arc::new(NestedField::required(1, "id", Type::Primitive(PrimitiveType::Int))),
                Arc::new(NestedField::required(
                    2,
                    "name",
                    Type::Primitive(PrimitiveType::String),
                )),
            ])
            .build()
            .unwrap();

        let mut writer = TbfFileWriterBuilder::new()
            .with_iceberg_schema(&iceberg_schema)
            .build();

        let id_array = Int32Array::from(vec![1, 2, 3]);
        let name_array = StringArray::from(vec!["Alice", "Bob", "Carol"]);

        let batch = RecordBatch::try_new(
            arrow_schema,
            vec![Arc::new(id_array), Arc::new(name_array)],
        )
        .unwrap();

        writer.write(&batch);

        assert_eq!(writer.row_count(), 3);
        assert!(writer.buffer_size() > 0);

        let data = writer.finish();
        assert_eq!(data.row_count, 3);
        assert!(data.file_size() > 0);
    }
}
