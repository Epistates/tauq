#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Directive(String), // !def, !use
    Ident(String),
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    TripleDash, // ---
    Colon,      // :
    Semi,       // ;
    Newline,
    LBrace,
    RBrace,
    LBracket, // [
    RBracket, // ]
    Eof,
}

/// Source location for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub line: usize,   // 1-based line number
    pub column: usize, // 1-based column number
    pub offset: usize, // byte offset from start
}

impl Location {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub start: Location,
    pub end: Location,
}
