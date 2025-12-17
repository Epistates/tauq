//! TBF-Iceberg Integration
//!
//! This module provides integration between TBF (Tauq Binary Format) and Apache Iceberg,
//! enabling TBF to be used as a file format for Iceberg tables.
//!
//! # Features
//!
//! - **TbfFileWriter**: Implements Iceberg's `FileWriter` trait for TBF format
//! - **Arrow conversion**: Converts Arrow RecordBatch to TBF columnar encoding
//! - **Statistics tracking**: Collects column statistics (min/max, null counts) during writes
//!
//! # Example
//!
//! ```rust,ignore
//! use tauq::tbf_iceberg::{TbfWriterBuilder, TbfWriter};
//! use iceberg::writer::file_writer::FileWriterBuilder;
//!
//! let builder = TbfWriterBuilder::new(schema);
//! let writer = builder.build(output_file).await?;
//! writer.write(&record_batch).await?;
//! let data_files = writer.close().await?;
//! ```

mod arrow_convert;
mod writer;

pub use arrow_convert::{arrow_schema_to_tbf, ArrowToTbf};
pub use writer::{TbfFileWriter, TbfFileWriterBuilder};
