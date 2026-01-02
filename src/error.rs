// Tauq Error Types
//
// Clean, helpful error messages for Tauq compilation

use thiserror::Error;

/// Span information for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// 1-based line number
    pub line: usize,
    /// 1-based column number
    pub column: usize,
}

impl Span {
    /// Create a new span
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Top-level Tauq error type
#[derive(Debug, Error)]
pub enum TauqError {
    /// Lexical error (invalid token)
    #[error("{0}")]
    Lex(#[from] LexError),

    /// Parse error (invalid syntax)
    #[error("{0}")]
    Parse(#[from] ParseError),

    /// Interpretation error (runtime/logic error)
    #[error("{0}")]
    Interpret(#[from] InterpretError),

    /// I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Lexer error
#[derive(Debug, Clone, PartialEq, Error)]
#[error("Lexer error at line {}, column {}: {message}", span.line, span.column)]
pub struct LexError {
    /// Error message
    pub message: String,
    /// Location of the error
    pub span: Span,
}

impl LexError {
    /// Create a new lexical error
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

/// Parser error with optional hint
#[derive(Debug, Clone, Error)]
pub struct ParseError {
    /// Error message
    pub message: String,
    /// Location of the error
    pub span: Span,
    /// Optional hint for fixing the error
    pub hint: Option<String>,
}

impl ParseError {
    /// Create a new parse error
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            hint: None,
        }
    }

    /// Add a hint to the error
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Parse error at line {}, column {}: {}",
            self.span.line, self.span.column, self.message
        )?;
        if let Some(hint) = &self.hint {
            write!(f, "\n  Hint: {}", hint)?;
        }
        Ok(())
    }
}

/// Interpreter error
#[derive(Debug, Clone, Error)]
pub struct InterpretError {
    /// Error message
    pub message: String,
    /// Location of the error (optional)
    pub span: Option<Span>,
}

impl InterpretError {
    /// Create a new interpreter error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
        }
    }

    /// Add location info to the error
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

impl std::fmt::Display for InterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(span) = &self.span {
            write!(
                f,
                "Interpretation error at line {}, column {}: {}",
                span.line, span.column, self.message
            )
        } else {
            write!(f, "Interpretation error: {}", self.message)
        }
    }
}
