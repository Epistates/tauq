use super::token::{Location, SpannedToken, Token};
use std::iter::Peekable;
use std::str::Chars;

pub struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<Chars<'a>>,
    // Position tracking
    offset: usize, // byte offset
    line: usize,   // 1-based line number
    column: usize, // 1-based column number
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().peekable(),
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    /// Get current location
    fn location(&self) -> Location {
        Location::new(self.line, self.column, self.offset)
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            self.offset += ch.len_utf8();
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        c
    }

    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

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
                name.push(self.advance().unwrap());
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
                    s.push(self.advance().unwrap());
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
            s.push(self.advance().unwrap());
        }

        // Try to parse as number
        if let Ok(n) = s.parse::<f64>() {
            Token::Number(n)
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
