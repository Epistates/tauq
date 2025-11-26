// Tauq Error Types
//
// Clean, helpful error messages for Tauq compilation

use thiserror::Error;

/// Span information for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Top-level Tauq error type
#[derive(Debug, Error)]
pub enum TauqError {
    #[error("{0}")]
    Lex(#[from] LexError),

    #[error("{0}")]
    Parse(#[from] ParseError),

    #[error("{0}")]
    Interpret(#[from] InterpretError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Lexer error
#[derive(Debug, Clone, PartialEq, Error)]
#[error("Lexer error at line {}, column {}: {message}", span.line, span.column)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl LexError {
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
    pub message: String,
    pub span: Span,
    pub hint: Option<String>,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            hint: None,
        }
    }

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
    pub message: String,
    pub span: Option<Span>,
}

impl InterpretError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
        }
    }

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
