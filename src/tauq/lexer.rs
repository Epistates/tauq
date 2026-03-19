use super::token::{Location, SpannedToken, Token};
use crate::error::{LexError, Span};
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
    /// Error recorded when an unterminated string literal is encountered
    pub lex_error: Option<crate::error::LexError>,
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
            lex_error: None,
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
        loop {
            self.skip_whitespace();

            let start = self.location();
            let ch = self.advance()?;

            let token = match ch {
                // Commas and comments are skipped iteratively to avoid
                // stack overflow on adversarial input (e.g. thousands of
                // consecutive commas).
                ',' => continue,
                '#' => {
                    self.skip_comment();
                    continue;
                }
                '!' => self.lex_directive(),
                ':' => Token::Colon,
                ';' => Token::Semi,
                '\n' => Token::Newline,
                '{' => Token::LBrace,
                '}' => Token::RBrace,
                '[' => Token::LBracket,
                ']' => Token::RBracket,
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
            return Some(SpannedToken { token, start, end });
        }
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
        let open_line = self.line;
        let open_column = self.column.saturating_sub(1); // position of the opening '"'
        let mut s = String::new();
        let mut closed = false;
        while let Some(&ch) = self.peek() {
            match ch {
                '"' => {
                    self.advance();
                    closed = true;
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
        if !closed && self.lex_error.is_none() {
            self.lex_error = Some(LexError::new(
                "unterminated string literal",
                Span::new(open_line, open_column),
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tauq::token::Token;

    /// Collect all tokens from an input string into a Vec.
    fn lex_all(input: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();
        while let Some(spanned) = lexer.next_token() {
            tokens.push(spanned.token);
        }
        tokens
    }

    /// Lex a single token and return it, panicking if the input is empty.
    fn lex_one(input: &str) -> Token {
        let mut lexer = Lexer::new(input);
        lexer
            .next_token()
            .expect("expected at least one token")
            .token
    }

    // -----------------------------------------------------------------------
    // Empty input
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_input_yields_no_tokens() {
        assert!(lex_all("").is_empty());
    }

    // -----------------------------------------------------------------------
    // Single-character structural tokens
    // -----------------------------------------------------------------------

    #[test]
    fn test_single_char_colon() {
        assert_eq!(lex_one(":"), Token::Colon);
    }

    #[test]
    fn test_single_char_semi() {
        assert_eq!(lex_one(";"), Token::Semi);
    }

    #[test]
    fn test_single_char_lbrace() {
        assert_eq!(lex_one("{"), Token::LBrace);
    }

    #[test]
    fn test_single_char_rbrace() {
        assert_eq!(lex_one("}"), Token::RBrace);
    }

    #[test]
    fn test_single_char_lbracket() {
        assert_eq!(lex_one("["), Token::LBracket);
    }

    #[test]
    fn test_single_char_rbracket() {
        assert_eq!(lex_one("]"), Token::RBracket);
    }

    #[test]
    fn test_single_newline() {
        assert_eq!(lex_one("\n"), Token::Newline);
    }

    // -----------------------------------------------------------------------
    // Comma treated as whitespace
    // -----------------------------------------------------------------------

    #[test]
    fn test_comma_is_whitespace_between_tokens() {
        // A comma between two values should be ignored; only the two values
        // appear in the token stream.
        let tokens = lex_all("42,99");
        assert_eq!(tokens, vec![Token::Integer(42), Token::Integer(99)]);
    }

    #[test]
    fn test_comma_only_input_yields_no_tokens() {
        assert!(lex_all(",,,").is_empty());
    }

    #[test]
    fn test_comma_between_structural_tokens() {
        let tokens = lex_all("{,}");
        assert_eq!(tokens, vec![Token::LBrace, Token::RBrace]);
    }

    // -----------------------------------------------------------------------
    // Triple-dash token
    // -----------------------------------------------------------------------

    #[test]
    fn test_triple_dash_token() {
        assert_eq!(lex_one("---"), Token::TripleDash);
    }

    #[test]
    fn test_triple_dash_followed_by_tokens() {
        let tokens = lex_all("--- foo");
        assert_eq!(
            tokens,
            vec![Token::TripleDash, Token::Ident("foo".to_string())]
        );
    }

    #[test]
    fn test_double_dash_is_not_triple_dash() {
        // "--" followed by a letter should produce a negative-sign bareword
        // (parsed as an Ident since "--" is not a valid number).
        let tokens = lex_all("--");
        assert_eq!(tokens.len(), 1);
        assert!(matches!(&tokens[0], Token::Ident(s) if s == "--"));
    }

    // -----------------------------------------------------------------------
    // Directive tokens
    // -----------------------------------------------------------------------

    #[test]
    fn test_directive_def() {
        assert_eq!(lex_one("!def"), Token::Directive("def".to_string()));
    }

    #[test]
    fn test_directive_use() {
        assert_eq!(lex_one("!use"), Token::Directive("use".to_string()));
    }

    #[test]
    fn test_directive_alphanumeric() {
        assert_eq!(
            lex_one("!my_directive123"),
            Token::Directive("my_directive123".to_string())
        );
    }

    #[test]
    fn test_directive_stops_at_non_alphanumeric() {
        // Only "def" should be captured; ":" is its own token.
        let tokens = lex_all("!def:");
        assert_eq!(
            tokens,
            vec![Token::Directive("def".to_string()), Token::Colon]
        );
    }

    // -----------------------------------------------------------------------
    // Bareword dispatch — integers
    // -----------------------------------------------------------------------

    #[test]
    fn test_integer_zero() {
        assert_eq!(lex_one("0"), Token::Integer(0));
    }

    #[test]
    fn test_positive_integer() {
        assert_eq!(lex_one("42"), Token::Integer(42));
    }

    #[test]
    fn test_negative_integer() {
        assert_eq!(lex_one("-7"), Token::Integer(-7));
    }

    #[test]
    fn test_i64_max() {
        let s = i64::MAX.to_string();
        assert_eq!(lex_one(&s), Token::Integer(i64::MAX));
    }

    // -----------------------------------------------------------------------
    // Bareword dispatch — unsigned integers (> i64::MAX)
    // -----------------------------------------------------------------------

    #[test]
    fn test_unsigned_integer_above_i64_max() {
        // i64::MAX + 1 cannot be represented as i64, falls back to u64.
        let value: u64 = i64::MAX as u64 + 1;
        let s = value.to_string();
        assert_eq!(lex_one(&s), Token::UnsignedInteger(value));
    }

    #[test]
    fn test_u64_max() {
        let s = u64::MAX.to_string();
        assert_eq!(lex_one(&s), Token::UnsignedInteger(u64::MAX));
    }

    // -----------------------------------------------------------------------
    // Bareword dispatch — floats
    // -----------------------------------------------------------------------

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_float_simple() {
        assert_eq!(lex_one("3.14"), Token::Float(3.14));
    }

    #[test]
    fn test_float_scientific_notation() {
        assert_eq!(lex_one("1e10"), Token::Float(1e10));
    }

    #[test]
    fn test_negative_float() {
        assert_eq!(lex_one("-0.5"), Token::Float(-0.5));
    }

    // -----------------------------------------------------------------------
    // Bareword dispatch — booleans and null
    // -----------------------------------------------------------------------

    #[test]
    fn test_bool_true() {
        assert_eq!(lex_one("true"), Token::Bool(true));
    }

    #[test]
    fn test_bool_false() {
        assert_eq!(lex_one("false"), Token::Bool(false));
    }

    #[test]
    fn test_null() {
        assert_eq!(lex_one("null"), Token::Null);
    }

    // -----------------------------------------------------------------------
    // Bareword dispatch — identifiers
    // -----------------------------------------------------------------------

    #[test]
    fn test_identifier_simple() {
        assert_eq!(lex_one("foo"), Token::Ident("foo".to_string()));
    }

    #[test]
    fn test_identifier_with_underscores_and_digits() {
        assert_eq!(lex_one("my_key_2"), Token::Ident("my_key_2".to_string()));
    }

    #[test]
    fn test_identifier_true_prefix_not_bool() {
        // "trueish" is not "true", so it must be an identifier.
        assert_eq!(lex_one("trueish"), Token::Ident("trueish".to_string()));
    }

    // -----------------------------------------------------------------------
    // String literals and escape sequences
    // -----------------------------------------------------------------------

    #[test]
    fn test_string_empty() {
        assert_eq!(lex_one(r#""""#), Token::String(String::new()));
    }

    #[test]
    fn test_string_plain() {
        assert_eq!(lex_one(r#""hello""#), Token::String("hello".to_string()));
    }

    #[test]
    fn test_escape_newline() {
        // Input: "\n" (the two characters backslash and n inside quotes)
        assert_eq!(lex_one(r#""\n""#), Token::String("\n".to_string()));
    }

    #[test]
    fn test_escape_carriage_return() {
        assert_eq!(lex_one(r#""\r""#), Token::String("\r".to_string()));
    }

    #[test]
    fn test_escape_tab() {
        assert_eq!(lex_one(r#""\t""#), Token::String("\t".to_string()));
    }

    #[test]
    fn test_escape_backslash() {
        assert_eq!(lex_one(r#""\\""#), Token::String("\\".to_string()));
    }

    #[test]
    fn test_escape_double_quote() {
        assert_eq!(lex_one(r#""\"""#), Token::String("\"".to_string()));
    }

    #[test]
    fn test_escape_unknown_sequence_preserved() {
        // An unrecognized escape like \x should produce the literal characters
        // backslash and 'x'.
        assert_eq!(lex_one(r#""\x""#), Token::String("\\x".to_string()));
    }

    #[test]
    fn test_string_all_escapes_combined() {
        // "\n\r\t\\\""  =>  newline, CR, tab, backslash, double-quote
        let expected = "\n\r\t\\\"".to_string();
        assert_eq!(lex_one(r#""\n\r\t\\\"" "#), Token::String(expected));
    }

    // -----------------------------------------------------------------------
    // Multi-byte UTF-8 characters
    // -----------------------------------------------------------------------

    #[test]
    fn test_utf8_string_content() {
        // Japanese characters (3 bytes each in UTF-8)
        assert_eq!(lex_one(r#""日本語""#), Token::String("日本語".to_string()));
    }

    #[test]
    fn test_utf8_emoji_in_string() {
        // Emoji is 4 bytes in UTF-8
        assert_eq!(lex_one(r#""🦀""#), Token::String("🦀".to_string()));
    }

    #[test]
    fn test_utf8_identifier() {
        // Multi-byte characters in an identifier-like bareword.
        // The lexer stops at whitespace/delimiters only, so a UTF-8 bareword
        // that doesn't parse as a number falls through to Ident.
        let tokens = lex_all("αβγ");
        assert_eq!(tokens.len(), 1);
        assert!(matches!(&tokens[0], Token::Ident(s) if s == "αβγ"));
    }

    #[test]
    fn test_utf8_byte_offset_tracking() {
        // Verify that byte-level offset accounting for multi-byte chars does
        // not corrupt subsequent token spans.  We simply check that both
        // tokens are produced correctly.
        let tokens = lex_all(r#""日本" 42"#);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::String("日本".to_string()));
        assert_eq!(tokens[1], Token::Integer(42));
    }

    // -----------------------------------------------------------------------
    // Integer overflow flag
    // -----------------------------------------------------------------------

    #[test]
    fn test_overflow_flag_not_set_for_normal_input() {
        let mut lexer = Lexer::new("hello world");
        while lexer.next_token().is_some() {}
        assert!(!lexer.overflow_occurred);
    }

    #[test]
    fn test_overflow_flag_field_is_accessible() {
        // The flag starts false and the struct is accessible from within the
        // module.  Triggering a real usize overflow would require a
        // pathologically large input, so we verify the initial state and that
        // normal lexing leaves it unchanged.
        let lexer = Lexer::new("");
        assert!(!lexer.overflow_occurred);
    }

    // -----------------------------------------------------------------------
    // Comment skipping
    // -----------------------------------------------------------------------

    #[test]
    fn test_comment_is_skipped() {
        let tokens = lex_all("# this is a comment\n42");
        assert_eq!(tokens, vec![Token::Newline, Token::Integer(42)]);
    }

    #[test]
    fn test_inline_comment_skipped() {
        let tokens = lex_all("foo # comment\nbar");
        assert_eq!(
            tokens,
            vec![
                Token::Ident("foo".to_string()),
                Token::Newline,
                Token::Ident("bar".to_string()),
            ]
        );
    }

    // -----------------------------------------------------------------------
    // Span / location sanity checks
    // -----------------------------------------------------------------------

    #[test]
    fn test_spanned_token_start_at_beginning() {
        let mut lexer = Lexer::new("abc");
        let spanned = lexer.next_token().unwrap();
        assert_eq!(spanned.start.offset, 0);
        assert_eq!(spanned.start.line, 1);
        assert_eq!(spanned.start.column, 1);
    }

    #[test]
    fn test_spanned_token_end_advances() {
        let mut lexer = Lexer::new("abc");
        let spanned = lexer.next_token().unwrap();
        // After consuming 3 bytes the end offset should be >= 3.
        assert!(spanned.end.offset >= 3);
    }
}
