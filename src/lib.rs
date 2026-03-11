// Tauq (τq) - Token-Efficient Data Notation
//
// Time constant meets charge density
// 44% fewer tokens than JSON (54% for flat data)
// Line-by-line parsing architecture
// Beautiful, minimal syntax
#![warn(missing_docs)]

//! Tauq (τq) - Token-Efficient Data Notation
//!
//! A schema-driven data format achieving 44-54% fewer tokens than JSON.
//!
//! # Example
//! ```
//! use tauq::compile_tauq;
//!
//! let source = r#"
//! name Alice
//! age 30
//! "#;
//!
//! let json = compile_tauq(source).unwrap();
//! assert_eq!(json["name"], "Alice");
//! ```

/// Error types for Tauq
pub mod error;
/// Serde integration (optional)
pub mod serde_support;
/// Core Tauq parser and formatter
pub mod tauq;
/// Tauq Binary Format (TBF) - high-performance columnar storage
pub mod tbf;

/// C bindings for Tauq
pub mod c_bindings;
#[cfg(feature = "java-bindings")]
/// Java bindings for Tauq
pub mod java_bindings;
#[cfg(feature = "python-bindings")]
/// Python bindings for Tauq
pub mod python_bindings;
#[cfg(feature = "iceberg")]
/// Iceberg table format integration for TBF
pub mod tbf_iceberg;

pub use error::TauqError;
pub use serde_support::{from_bytes, from_file, from_str};
pub use tauq::Delimiter;
pub use tauq::{Formatter, Lexer, Parser, StreamingParser};
pub use tauq::{json_to_tauq, json_to_tauq_optimized, json_to_tauq_ultra, minify_tauq};

/// Maximum input size (100 MB) to prevent DoS via memory exhaustion
pub const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;

/// Maximum nesting depth for recursive structures
pub const MAX_NESTING_DEPTH: usize = 100;

/// Parse Tauq source to JSON
///
/// # Example
/// ```
/// let source = "name Alice\nage 30";
/// let json = tauq::compile_tauq(source).unwrap();
/// ```
///
/// # Errors
/// Returns `TauqError` if the source contains syntax errors.
pub fn compile_tauq(source: &str) -> Result<serde_json::Value, error::TauqError> {
    // Validate input size to prevent DoS
    if source.len() > MAX_INPUT_SIZE {
        return Err(error::TauqError::Interpret(error::InterpretError::new(
            format!(
                "Input too large: {} bytes (max {} bytes)",
                source.len(),
                MAX_INPUT_SIZE
            ),
        )));
    }
    let mut parser = tauq::Parser::new(source);
    let result = parser.parse().map_err(error::TauqError::Parse)?;
    Ok(result)
}

/// Execute TauqQ in safe mode (shell execution disabled) - **RECOMMENDED**
///
/// This is the safe default that should be used for untrusted input.
/// Shell directives (!emit, !run, !pipe) are disabled.
///
/// # Example
/// ```
/// let source = "!def User id name\n1 Alice";
/// let json = tauq::compile_tauqq_safe(source).unwrap();
/// ```
pub fn compile_tauqq_safe(source: &str) -> Result<serde_json::Value, error::TauqError> {
    compile_tauqq(source, true)
}

/// Execute TauqQ with shell execution enabled - **USE WITH CAUTION**
///
/// # Security Warning
/// This enables arbitrary shell command execution via !emit, !run, and !pipe directives.
/// Only use this with trusted input. For untrusted input, use `compile_tauqq_safe()` instead.
///
/// # Example
/// ```no_run
/// // Only use with trusted input!
/// let source = "!emit echo hello";
/// let json = tauq::compile_tauqq_unsafe(source).unwrap();
/// ```
pub fn compile_tauqq_unsafe(source: &str) -> Result<serde_json::Value, error::TauqError> {
    compile_tauqq(source, false)
}

/// Execute TauqQ (Tauq Query with transformations)
///
/// # Arguments
/// * `source` - The TauqQ source code
/// * `safe_mode` - If true, disables shell execution (!emit, !run, !pipe)
///
/// # Security Warning
/// When `safe_mode` is false, this allows arbitrary shell command execution.
/// Always use `safe_mode = true` for untrusted input.
pub fn compile_tauqq(source: &str, safe_mode: bool) -> Result<serde_json::Value, error::TauqError> {
    let processed = process_tauqq(source, safe_mode)?;
    compile_tauq(&processed)
}

/// Process TauqQ directives without parsing (returns processed Tauq source)
///
/// # Arguments
/// * `source` - The TauqQ source code
/// * `safe_mode` - If true, disables shell execution (!emit, !run, !pipe)
pub fn process_tauqq(source: &str, safe_mode: bool) -> Result<String, error::TauqError> {
    // Validate input size
    if source.len() > MAX_INPUT_SIZE {
        return Err(error::TauqError::Interpret(error::InterpretError::new(
            format!(
                "Input too large: {} bytes (max {} bytes)",
                source.len(),
                MAX_INPUT_SIZE
            ),
        )));
    }
    let mut vars = std::collections::HashMap::new();
    tauq::tauqq::process(source, &mut vars, safe_mode)
        .map_err(|e| error::TauqError::Interpret(error::InterpretError::new(e)))
}

/// Format JSON to Tauq syntax
///
/// Converts a JSON value to token-efficient Tauq notation.
pub fn format_to_tauq(json: &serde_json::Value) -> String {
    tauq::json_to_tauq(json)
}

/// Minify Tauq to single line
///
/// Produces a minified single-line Tauq representation.
pub fn minify_tauq_str(json: &serde_json::Value) -> String {
    tauq::minify_tauq(json)
}

/// Print an error with source code context
pub fn print_error_with_source(source: &str, error: &error::TauqError) {
    let span = match error {
        error::TauqError::Lex(e) => Some(e.span),
        error::TauqError::Parse(e) => Some(e.span),
        error::TauqError::Interpret(e) => e.span,
        error::TauqError::Io(_) => None,
    };

    if let Some(span) = span {
        let lines: Vec<&str> = source.lines().collect();
        // Spans are 1-based
        if span.line > 0 && span.line <= lines.len() {
            let line_idx = span.line - 1;
            let line = lines[line_idx];

            eprintln!("Error: {}", error);
            eprintln!("   |");
            eprintln!("{:2} | {}", span.line, line);

            let mut pointer = String::new();
            for _ in 0..span.column {
                pointer.push(' ');
            }
            pointer.push('^');

            eprintln!("   | {}", pointer);
            eprintln!("   |");
        } else {
            eprintln!("Error: {}", error);
        }
    } else {
        eprintln!("Error: {}", error);
    }
}
