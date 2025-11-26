// Tauq (τq) - Token-Efficient Data Notation
//
// Time constant meets charge density
// 44% fewer tokens than JSON (54% for flat data)
// Line-by-line parsing architecture
// Beautiful, minimal syntax

pub mod error;
pub mod tauq;

pub mod c_bindings;
#[cfg(feature = "java-bindings")]
pub mod java_bindings;
#[cfg(feature = "python-bindings")]
pub mod python_bindings;

pub use error::TauqError;
pub use tauq::{Formatter, Lexer, Parser, StreamingParser};
pub use tauq::{json_to_tauq, json_to_tauq_optimized, json_to_tauq_ultra, minify_tauq};
pub use tauq::Delimiter;

/// Parse Tauq source to JSON
pub fn compile_tauq(source: &str) -> Result<serde_json::Value, error::TauqError> {
    let mut parser = tauq::Parser::new(source);
    let result = parser.parse().map_err(error::TauqError::Parse)?;
    Ok(result)
}

/// Execute TauqQ (Tauq Query with transformations)
pub fn compile_tauqq(source: &str, safe_mode: bool) -> Result<serde_json::Value, error::TauqError> {
    let processed = process_tauqq(source, safe_mode)?;
    compile_tauq(&processed)
}

/// Process TauqQ directives without parsing (returns processed Tauq source)
pub fn process_tauqq(source: &str, safe_mode: bool) -> Result<String, error::TauqError> {
    let mut vars = std::collections::HashMap::new();
    tauq::tauqq::process(source, &mut vars, safe_mode)
        .map_err(|e| error::TauqError::Interpret(error::InterpretError::new(e)))
}

/// Format JSON to Tauq syntax
pub fn format_to_tauq(json: &serde_json::Value) -> String {
    tauq::json_to_tauq(json)
}

/// Minify Tauq to single line
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
