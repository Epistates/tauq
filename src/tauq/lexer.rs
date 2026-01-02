use super::token::{Location, SpannedToken, Token};
use std::iter::Peekable;
use std::str::Chars;

/// Lexer for tokenizing Tauq source code
pub struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<Chars<'a>>,
    // Position tracking (using checked arithmetic for safety)
    offset: usize, // byte offset
    line: usize,   // 1-based line number
    column: usize, // 1-based column number
    /// Flag indicating overflow occurred (lexer continues but positions may be inaccurate)
    overflow_occurred: bool,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().peekable(),
            offset: 0,
            line: 1,
            column: 1,
            overflow_occurred: false,
        }
    }

    /// Get current location
    fn location(&self) -> Location {
        Location::new(self.line, self.column, self.offset)
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            // Use checked arithmetic to prevent overflow
            self.offset = self.offset.checked_add(ch.len_utf8()).unwrap_or_else(|| {
                self.overflow_occurred = true;
                self.offset // Keep the old value on overflow
            });
            if ch == '\n' {
                self.line = self.line.checked_add(1).unwrap_or_else(|| {
                    self.overflow_occurred = true;
                    self.line
                });
                self.column = 1;
            } else {
                self.column = self.column.checked_add(1).unwrap_or_else(|| {
                    self.overflow_occurred = true;
                    self.column
                });
            }
        }
        c
    }

    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    /// Get the next token from the input
    pub fn next_token(&mut self) -> Option<SpannedToken> {
        self.skip_whitespace();

        let start = self.location();
        let ch = self.advance()?;

        let token = match ch {
            '!' => self.lex_directive(),
            ':' => Token::Colon,
            ';' => Token::Semi,
            '\n' => Token::Newline,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            ',' => return self.next_token(), // Treat comma as whitespace/separator
            '#' => {
                self.skip_comment();
                return self.next_token();
            }
            '"' => self.lex_string(),
            '-' => {
                // Check for ---
                let mut lookahead = self.chars.clone();
                if lookahead.next() == Some('-') && lookahead.next() == Some('-') {
                    self.advance(); // consume 2nd -
                    self.advance(); // consume 3rd -
                    Token::TripleDash
                } else {
                    self.lex_bareword(ch)
                }
            }
            _ => self.lex_bareword(ch),
        };

        let end = self.location();
        Some(SpannedToken { token, start, end })
    }

    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.peek() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        while let Some(&ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
    }

    fn lex_directive(&mut self) -> Token {
        let mut name = String::new();
        while let Some(&ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                // Safe: we just checked peek() returned Some
                if let Some(c) = self.advance() {
                    name.push(c);
                }
            } else {
                break;
            }
        }
        Token::Directive(name)
    }

    fn lex_string(&mut self) -> Token {
        let mut s = String::new();
        while let Some(&ch) = self.peek() {
            match ch {
                '"' => {
                    self.advance();
                    break;
                }
                '\\' => {
                    self.advance(); // consume backslash
                    if let Some(escaped) = self.advance() {
                        match escaped {
                            '"' => s.push('"'),
                            '\\' => s.push('\\'),
                            'n' => s.push('\n'),
                            'r' => s.push('\r'),
                            't' => s.push('\t'),
                            _ => {
                                s.push('\\');
                                s.push(escaped);
                            }
                        }
                    }
                }
                _ => {
                    // Safe: we just checked peek() returned Some
                    if let Some(c) = self.advance() {
                        s.push(c);
                    }
                }
            }
        }
        Token::String(s)
    }

    fn lex_bareword(&mut self, first: char) -> Token {
        let mut s = String::from(first);

        while let Some(&ch) = self.peek() {
            // Stop at delimiters
            if ch.is_whitespace() || "{}[],:;\"#\n".contains(ch) {
                break;
            }
            // Safe: we just checked peek() returned Some
            if let Some(c) = self.advance() {
                s.push(c);
            }
        }

        // Try to parse as number with precision fallback
        if let Ok(i) = s.parse::<i64>() {
            Token::Integer(i)
        } else if let Ok(u) = s.parse::<u64>() {
            Token::UnsignedInteger(u)
        } else if let Ok(f) = s.parse::<f64>() {
            Token::Float(f)
        } else {
            match s.as_str() {
                "true" => Token::Bool(true),
                "false" => Token::Bool(false),
                "null" => Token::Null,
                _ => Token::Ident(s),
            }
        }
    }

    /// Get the source input (useful for error messages)
    pub fn source(&self) -> &'a str {
        self.input
    }
}
