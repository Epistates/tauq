// Tauq Streaming Parser
//
// True streaming/iterator API for parsing Tauq data row-by-row.
// Memory-efficient for large datasets - only one record in memory at a time.

use super::lexer::Lexer;
use super::parser::{Context, FieldDef, TypeDef};
use super::token::{Location, SpannedToken, Token};
use crate::error::{ParseError, Span};
use serde_json::{Map, Value};

/// Streaming parser that yields records one at a time.
///
/// # Example
/// ```
/// use tauq::tauq::streaming::StreamingParser;
///
/// let input = "!def User id name\n1 Alice\n2 Bob";
/// let mut parser = StreamingParser::new(input);
///
/// while let Some(result) = parser.next_record() {
///     match result {
///         Ok(record) => println!("{}", record),
///         Err(e) => eprintln!("Error: {}", e),
///     }
/// }
/// ```
pub struct StreamingParser<'a> {
    lexer: Lexer<'a>,
    current_token: Option<SpannedToken>,
    peek_token: Option<SpannedToken>,
    context: Context,
    active_shape: Option<String>,
    pending_kv: Map<String, Value>,
    finished: bool,
}

impl<'a> StreamingParser<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut lexer = Lexer::new(source);
        let current_token = lexer.next_token();
        let peek_token = lexer.next_token();
        Self {
            lexer,
            current_token,
            peek_token,
            context: Context::new(),
            active_shape: None,
            pending_kv: Map::new(),
            finished: false,
        }
    }

    /// Get the next record from the stream.
    /// Returns `None` when the stream is exhausted.
    pub fn next_record(&mut self) -> Option<Result<Value, ParseError>> {
        if self.finished {
            return None;
        }

        loop {
            let st = match &self.current_token {
                Some(st) => st.clone(),
                None => {
                    self.finished = true;
                    // Flush any pending key-value pairs
                    if !self.pending_kv.is_empty() {
                        let result = Value::Object(std::mem::take(&mut self.pending_kv));
                        return Some(Ok(result));
                    }
                    return None;
                }
            };

            match &st.token {
                Token::Directive(d) => {
                    // Flush pending before directive
                    if !self.pending_kv.is_empty() {
                        let result = Value::Object(std::mem::take(&mut self.pending_kv));
                        return Some(Ok(result));
                    }

                    let d_str = d.clone();
                    self.advance();

                    if let Err(e) = self.handle_directive(&d_str) {
                        return Some(Err(e));
                    }
                }
                Token::Newline | Token::Semi => {
                    self.advance();
                }
                Token::RBrace => {
                    let loc = st.start;
                    return Some(Err(
                        self.make_error_at("Unexpected '}' - mismatched braces", loc)
                    ));
                }
                Token::RBracket => {
                    let loc = st.start;
                    return Some(Err(
                        self.make_error_at("Unexpected ']' - mismatched brackets", loc)
                    ));
                }
                _ => {
                    if self.active_shape.is_some() {
                        // Flush pending before row
                        if !self.pending_kv.is_empty() {
                            let result = Value::Object(std::mem::take(&mut self.pending_kv));
                            return Some(Ok(result));
                        }

                        match self.parse_row() {
                            Ok(Some(row)) => return Some(Ok(row)),
                            Ok(None) => continue,
                            Err(e) => return Some(Err(e)),
                        }
                    } else {
                        // Try to parse as key-value entry
                        match self.parse_kv_entry() {
                            Ok(Some((key, value))) => {
                                self.pending_kv.insert(key, value);
                            }
                            Ok(None) => {
                                self.advance();
                            }
                            Err(e) => return Some(Err(e)),
                        }
                    }
                }
            }
        }
    }

    fn advance(&mut self) {
        self.current_token = self.peek_token.take();
        self.peek_token = self.lexer.next_token();
    }

    fn current_location(&self) -> Location {
        self.current_token
            .as_ref()
            .map(|t| t.start)
            .unwrap_or(Location::new(1, 1, 0))
    }

    fn make_error(&self, msg: impl Into<String>) -> ParseError {
        let loc = self.current_location();
        ParseError::new(msg, Span::new(loc.line, loc.column))
    }

    fn make_error_at(&self, msg: impl Into<String>, loc: Location) -> ParseError {
        ParseError::new(msg, Span::new(loc.line, loc.column))
    }

    fn handle_directive(&mut self, name: &str) -> Result<(), ParseError> {
        match name {
            "def" => {
                if let Some(st) = self.current_token.clone()
                    && let Token::Ident(shape_name) = st.token
                {
                    self.advance();
                    let mut fields = Vec::new();

                    while let Some(st_curr) = &self.current_token {
                        let field_name = match &st_curr.token {
                            Token::Ident(n) => n.clone(),
                            Token::Newline | Token::Semi => break,
                            _ => break,
                        };
                        self.advance();

                        let type_def = self.parse_type_annotation()?;
                        fields.push(FieldDef {
                            name: field_name,
                            type_def,
                        });
                    }

                    self.context
                        .shapes
                        .borrow_mut()
                        .insert(shape_name.clone(), fields);
                    self.active_shape = Some(shape_name);
                }
            }
            "use" => {
                if let Some(st) = self.current_token.clone()
                    && let Token::Ident(shape_name) = st.token
                {
                    if !self.context.shapes.borrow().contains_key(&shape_name) {
                        return Err(self.make_error(format!(
                            "!use references undefined schema '{}'",
                            shape_name
                        )));
                    }
                    self.active_shape = Some(shape_name);
                    self.advance();
                }
            }
            _ => {
                // Skip unknown directives in streaming mode
                while let Some(st) = &self.current_token {
                    if matches!(st.token, Token::Newline | Token::Semi) {
                        break;
                    }
                    self.advance();
                }
            }
        }
        Ok(())
    }

    fn parse_type_annotation(&mut self) -> Result<TypeDef, ParseError> {
        if !matches!(
            self.current_token.as_ref().map(|t| &t.token),
            Some(Token::Colon)
        ) {
            return Ok(TypeDef::Scalar);
        }
        self.advance(); // Skip :

        // Check for list type [Type]
        if matches!(
            self.current_token.as_ref().map(|t| &t.token),
            Some(Token::LBracket)
        ) {
            self.advance();
            if let Some(st) = &self.current_token
                && let Token::Ident(inner) = &st.token
            {
                let t = TypeDef::List(inner.clone());
                self.advance();
                if matches!(
                    self.current_token.as_ref().map(|t| &t.token),
                    Some(Token::RBracket)
                ) {
                    self.advance();
                }
                return Ok(t);
            }
            return Err(self.make_error("Expected type name in list type"));
        }

        // Object type
        if let Some(st) = &self.current_token
            && let Token::Ident(t) = &st.token
        {
            let t_def = TypeDef::Object(t.clone());
            self.advance();
            return Ok(t_def);
        }

        Ok(TypeDef::Scalar)
    }

    fn parse_row(&mut self) -> Result<Option<Value>, ParseError> {
        let shape_name = match &self.active_shape {
            Some(n) => n.clone(),
            None => return Ok(None),
        };

        let fields = match self.context.shapes.borrow().get(&shape_name) {
            Some(f) => f.clone(),
            None => return Ok(None),
        };

        let mut obj = Map::new();
        let mut field_idx = 0;

        loop {
            if self.current_token.is_none() {
                break;
            }
            if let Some(st) = &self.current_token
                && matches!(
                    st.token,
                    Token::Newline | Token::Semi | Token::RBrace | Token::RBracket
                )
            {
                break;
            }

            if field_idx >= fields.len() {
                // Extra values - skip
                self.advance();
                continue;
            }

            let field = &fields[field_idx];
            let value = self.parse_value(&field.type_def)?;
            obj.insert(field.name.clone(), value);
            field_idx += 1;
        }

        // Skip newline/semi
        if let Some(st) = &self.current_token
            && matches!(st.token, Token::Newline | Token::Semi)
        {
            self.advance();
        }

        if obj.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Value::Object(obj)))
        }
    }

    fn parse_value(&mut self, _type_def: &TypeDef) -> Result<Value, ParseError> {
        let st = match &self.current_token {
            Some(st) => st.clone(),
            None => return Err(self.make_error("Unexpected end of input")),
        };

        let value = match &st.token {
            Token::Number(n) => {
                self.advance();
                Value::Number(
                    serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0)),
                )
            }
            Token::String(s) => {
                self.advance();
                Value::String(s.clone())
            }
            Token::Ident(s) => {
                self.advance();
                Value::String(s.clone())
            }
            Token::Bool(b) => {
                self.advance();
                Value::Bool(*b)
            }
            Token::Null => {
                self.advance();
                Value::Null
            }
            Token::LBracket => self.parse_array()?,
            Token::LBrace => self.parse_object()?,
            _ => {
                return Err(self.make_error(format!("Unexpected token: {:?}", st.token)));
            }
        };

        Ok(value)
    }

    fn parse_array(&mut self) -> Result<Value, ParseError> {
        self.advance(); // Skip [
        let mut arr = Vec::new();

        loop {
            if let Some(st) = &self.current_token {
                if matches!(st.token, Token::RBracket) {
                    self.advance();
                    break;
                }
                if matches!(st.token, Token::Newline | Token::Semi) {
                    self.advance();
                    continue;
                }
            } else {
                return Err(self.make_error("Unterminated array"));
            }

            let value = self.parse_value(&TypeDef::Scalar)?;
            arr.push(value);
        }

        Ok(Value::Array(arr))
    }

    fn parse_object(&mut self) -> Result<Value, ParseError> {
        self.advance(); // Skip {
        let mut obj = Map::new();

        loop {
            if let Some(st) = &self.current_token {
                if matches!(st.token, Token::RBrace) {
                    self.advance();
                    break;
                }
                if matches!(st.token, Token::Newline | Token::Semi) {
                    self.advance();
                    continue;
                }
            } else {
                return Err(self.make_error("Unterminated object"));
            }

            // Parse key
            let key = match &self.current_token {
                Some(st) => match &st.token {
                    Token::Ident(k) => {
                        let k = k.clone();
                        self.advance();
                        k
                    }
                    Token::String(k) => {
                        let k = k.clone();
                        self.advance();
                        k
                    }
                    _ => return Err(self.make_error("Expected key in object")),
                },
                None => return Err(self.make_error("Unexpected end in object")),
            };

            // Parse value
            let value = self.parse_value(&TypeDef::Scalar)?;
            obj.insert(key, value);
        }

        Ok(Value::Object(obj))
    }

    fn parse_kv_entry(&mut self) -> Result<Option<(String, Value)>, ParseError> {
        let key = match &self.current_token {
            Some(st) => match &st.token {
                Token::Ident(k) => {
                    let k = k.clone();
                    self.advance();
                    k
                }
                Token::String(k) => {
                    let k = k.clone();
                    self.advance();
                    k
                }
                _ => return Ok(None),
            },
            None => return Ok(None),
        };

        let value = self.parse_value(&TypeDef::Scalar)?;
        Ok(Some((key, value)))
    }
}

/// Iterator adapter for StreamingParser
impl<'a> Iterator for StreamingParser<'a> {
    type Item = Result<Value, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_record()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_basic() {
        let input = "!def User id name\n1 Alice\n2 Bob\n3 Carol";
        let parser = StreamingParser::new(input);

        let records: Vec<_> = parser.collect();
        assert_eq!(records.len(), 3);

        let first = records[0].as_ref().unwrap();
        assert_eq!(first["id"], 1.0);
        assert_eq!(first["name"], "Alice");
    }

    #[test]
    fn test_streaming_memory_efficient() {
        // Generate large dataset
        let mut input = String::from("!def Row id value\n");
        for i in 0..10000 {
            input.push_str(&format!("{} \"val{}\"\n", i, i));
        }

        let mut parser = StreamingParser::new(&input);
        let mut count = 0;

        // Process one at a time - constant memory
        while let Some(result) = parser.next_record() {
            assert!(result.is_ok());
            count += 1;
        }

        assert_eq!(count, 10000);
    }

    #[test]
    fn test_streaming_iterator() {
        let input = "!def Point x y\n10 20\n30 40";

        // Can use as iterator
        let count = StreamingParser::new(input).filter_map(|r| r.ok()).count();

        assert_eq!(count, 2);
    }
}
