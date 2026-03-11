/// Token types for Tauq lexer
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Directive (e.g. `!def`, `!use`)
    Directive(String),
    /// Identifier
    Ident(String),
    /// String literal
    String(String),
    /// Signed integer literal
    Integer(i64),
    /// Unsigned integer literal (for values > i64::MAX)
    UnsignedInteger(u64),
    /// Floating point literal
    Float(f64),
    /// Boolean literal
    Bool(bool),
    /// Null literal
    Null,
    /// Triple dash separator `---`
    TripleDash,
    /// Colon separator `:`
    Colon,
    /// Semicolon separator `;`
    Semi,
    /// Newline separator
    Newline,
    /// Left brace `{`
    LBrace,
    /// Right brace `}`
    RBrace,
    /// Left bracket `[`
    LBracket,
    /// Right bracket `]`
    RBracket,
}

/// Source location for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    /// 1-based line number
    pub line: usize,
    /// 1-based column number
    pub column: usize,
    /// byte offset from start
    pub offset: usize,
}

impl Location {
    /// Create a new location
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }
}

/// A token with its source location
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    /// The token definition
    pub token: Token,
    /// Start location of the token
    pub start: Location,
    /// End location of the token
    pub end: Location,
}
