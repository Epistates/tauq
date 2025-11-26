use super::lexer::Lexer;
use super::token::{Location, SpannedToken, Token};
use crate::error::{ParseError, Span};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::Path;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub type_def: TypeDef,
}

#[derive(Debug, Clone)]
pub enum TypeDef {
    Scalar,
    Object(String),
    List(String),
}

#[derive(Clone)]
pub struct Context {
    pub shapes: Rc<RefCell<HashMap<String, Vec<FieldDef>>>>,
    /// Base directory for resolving relative imports
    pub base_dir: Option<std::path::PathBuf>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            shapes: Rc::new(RefCell::new(HashMap::new())),
            base_dir: None,
        }
    }

    pub fn with_base_dir(base_dir: std::path::PathBuf) -> Self {
        Self {
            shapes: Rc::new(RefCell::new(HashMap::new())),
            base_dir: Some(base_dir),
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Option<SpannedToken>,
    peek_token: Option<SpannedToken>,
    context: Context,
    active_shape: Option<String>,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        Self::new_with_context(source, Context::new())
    }

    pub fn new_with_context(source: &'a str, context: Context) -> Self {
        let mut lexer = Lexer::new(source);
        let current_token = lexer.next_token();
        let peek_token = lexer.next_token();
        Self {
            lexer,
            current_token,
            peek_token,
            context,
            active_shape: None,
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

    /// Check if current token matches the given token type
    fn check(&self, token: &Token) -> bool {
        self.current_token
            .as_ref()
            .map(|t| std::mem::discriminant(&t.token) == std::mem::discriminant(token))
            .unwrap_or(false)
    }

    pub fn parse(&mut self) -> Result<Value, ParseError> {
        let mut result = Vec::new();
        let mut pending_map = Map::new();

        while let Some(st) = &self.current_token {
            match &st.token {
                Token::Directive(d) => {
                    if !pending_map.is_empty() {
                        result.push(Value::Object(pending_map));
                        pending_map = Map::new();
                    }
                    let d_str = d.clone();
                    self.advance();
                    if d_str == "schemas" || d_str == "models" {
                        self.handle_schemas_block()?;
                    } else {
                        self.handle_directive(&d_str)?;
                    }
                }
                Token::Newline | Token::Semi => self.advance(),
                Token::TripleDash => {
                    // --- clears the active schema (ends implicit !use scope)
                    self.active_shape = None;
                    self.advance();
                }
                Token::RBrace => {
                    let loc = st.start;
                    return Err(
                        self.make_error_at("Unexpected '}' at top level - mismatched braces", loc)
                    );
                }
                Token::RBracket => {
                    let loc = st.start;
                    return Err(self
                        .make_error_at("Unexpected ']' at top level - mismatched brackets", loc));
                }
                _ => {
                    if self.active_shape.is_some() {
                        if !pending_map.is_empty() {
                            result.push(Value::Object(pending_map));
                            pending_map = Map::new();
                        }
                        if let Some(row) = self.parse_row()? {
                            result.push(row);
                        }
                    } else {
                        // Try to parse as map entry
                        if let Some(val) = self.parse_map_entry()? {
                            if let Value::Object(map) = val {
                                for (k, v) in map {
                                    pending_map.insert(k, v);
                                }
                            }
                        } else if let Some(val) = self.parse_value()? {
                            result.push(val);
                        } else {
                            let loc = self.current_location();
                            let token_desc = self
                                .current_token
                                .as_ref()
                                .map(|t| format!("{:?}", t.token))
                                .unwrap_or_else(|| "EOF".to_string());
                            return Err(self
                                .make_error_at(format!("Unexpected token: {}", token_desc), loc));
                        }
                    }
                }
            }
        }

        if !pending_map.is_empty() {
            result.push(Value::Object(pending_map));
        }

        if result.len() == 1 {
            Ok(result.remove(0))
        } else {
            Ok(Value::Array(result))
        }
    }

    fn handle_schemas_block(&mut self) -> Result<(), ParseError> {
        loop {
            match &self.current_token {
                Some(st) => match &st.token {
                    Token::TripleDash => {
                        self.advance();
                        break;
                    }
                    Token::Ident(shape_name) => {
                        let shape_name = shape_name.clone();
                        self.advance();

                        let mut fields = Vec::new();
                        // Parse fields until newline or EOF or TripleDash
                        while let Some(st2) = &self.current_token {
                            match &st2.token {
                                Token::Ident(name) => {
                                    let name = name.clone();
                                    self.advance();

                                    let type_def = self.parse_type_annotation()?;
                                    fields.push(FieldDef { name, type_def });
                                }
                                Token::Newline | Token::Semi => {
                                    self.advance();
                                    break;
                                }
                                Token::TripleDash => {
                                    break;
                                }
                                _ => {
                                    self.advance();
                                    break;
                                }
                            }
                        }
                        self.context.shapes.borrow_mut().insert(shape_name, fields);
                    }
                    Token::Newline | Token::Semi => {
                        self.advance();
                    }
                    _ => {
                        let loc = st.start;
                        return Err(self
                            .make_error_at("Expected schema name or '---' in schema block", loc));
                    }
                },
                None => {
                    return Err(self.make_error("Unterminated schema block - expected '---'"));
                }
            }
        }
        Ok(())
    }

    /// Parse optional type annotation (:Type or :[Type])
    fn parse_type_annotation(&mut self) -> Result<TypeDef, ParseError> {
        if !self.check(&Token::Colon) {
            return Ok(TypeDef::Scalar);
        }
        self.advance(); // Skip :

        // Check for list type [Type]
        if self.check(&Token::LBracket) {
            self.advance(); // Skip [
            if let Some(st) = &self.current_token
                && let Token::Ident(inner) = &st.token
            {
                let t = TypeDef::List(inner.clone());
                self.advance();
                if self.check(&Token::RBracket) {
                    self.advance();
                } else {
                    return Err(self.make_error("Expected ']' after list type"));
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

        // Type annotation like :int, :string - these are documentation only
        // We treat them as Scalar since we don't enforce types at parse time
        Ok(TypeDef::Scalar)
    }

    fn handle_directive(&mut self, name: &str) -> Result<(), ParseError> {
        match name {
            "import" => {
                if let Some(st) = self.current_token.clone() {
                    if let Token::String(path) = st.token {
                        self.advance();
                        self.handle_import(&path)?;
                    } else {
                        return Err(self.make_error("!import requires a quoted path string"));
                    }
                } else {
                    return Err(self.make_error("!import requires a path argument"));
                }
            }
            "def" => {
                // !def Name field1 field2:Type
                if let Some(st) = self.current_token.clone() {
                    if let Token::Ident(shape_name) = st.token {
                        self.advance();
                        let mut fields = Vec::new();

                        while let Some(st_curr) = &self.current_token {
                            let name = match &st_curr.token {
                                Token::Ident(n) => n.clone(),
                                Token::Newline | Token::Semi => break,
                                _ => break,
                            };
                            self.advance();

                            let type_def = self.parse_type_annotation()?;
                            fields.push(FieldDef { name, type_def });
                        }
                        self.context
                            .shapes
                            .borrow_mut()
                            .insert(shape_name.clone(), fields);
                        self.active_shape = Some(shape_name);
                    } else {
                        return Err(self.make_error("!def requires a schema name"));
                    }
                } else {
                    return Err(self.make_error("!def requires a schema name"));
                }
            }
            "use" => {
                if let Some(st) = self.current_token.clone() {
                    if let Token::Ident(shape_name) = st.token {
                        if !self.context.shapes.borrow().contains_key(&shape_name) {
                            return Err(self.make_error(format!(
                                "!use references undefined schema '{}'",
                                shape_name
                            )));
                        }
                        self.active_shape = Some(shape_name);
                        self.advance();
                    } else {
                        return Err(self.make_error("!use requires a schema name"));
                    }
                } else {
                    return Err(self.make_error("!use requires a schema name"));
                }
            }
            _ => {
                // Unknown directive - skip but warn
                // In a production system, this might be a warning or error
            }
        }
        Ok(())
    }

    fn handle_import(&mut self, path: &str) -> Result<(), ParseError> {
        // Resolve path relative to base_dir if set
        let resolved_path = if let Some(base) = &self.context.base_dir {
            base.join(path)
        } else {
            Path::new(path).to_path_buf()
        };

        // Security: Check for path traversal
        let canonical = resolved_path.canonicalize().map_err(|e| {
            self.make_error(format!("Cannot resolve import path '{}': {}", path, e))
        })?;

        if let Some(base) = &self.context.base_dir {
            let base_canonical = base
                .canonicalize()
                .map_err(|e| self.make_error(format!("Cannot resolve base directory: {}", e)))?;
            if !canonical.starts_with(&base_canonical) {
                return Err(self.make_error(format!(
                    "Import path '{}' escapes base directory (path traversal blocked)",
                    path
                )));
            }
        }

        let content = std::fs::read_to_string(&canonical).map_err(|e| {
            self.make_error(format!("Failed to read imported file '{}': {}", path, e))
        })?;

        // Parse imported file with same context
        let mut import_context = self.context.clone();
        import_context.base_dir = canonical.parent().map(|p| p.to_path_buf());

        let mut parser = Parser::new_with_context(&content, import_context);
        parser
            .parse()
            .map_err(|e| self.make_error(format!("Error in imported file '{}': {}", path, e)))?;

        // Copy shapes back to our context
        // (shapes are shared via Rc<RefCell<...>> so they're already updated)

        Ok(())
    }

    fn parse_row(&mut self) -> Result<Option<Value>, ParseError> {
        let shape_name = if let Some(n) = &self.active_shape {
            n.clone()
        } else {
            return Ok(None);
        };

        let fields = if let Some(f) = self.context.shapes.borrow().get(&shape_name) {
            f.clone()
        } else {
            return Ok(None);
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

            // Check for Named Arg: Ident + Colon
            let is_named = if let Some(st) = &self.current_token {
                if let Token::Ident(_) = st.token {
                    matches!(
                        self.peek_token.as_ref().map(|t| &t.token),
                        Some(Token::Colon)
                    )
                } else {
                    false
                }
            } else {
                false
            };

            if is_named {
                if let Some(st) = self.current_token.clone()
                    && let Token::Ident(key) = st.token
                {
                    self.advance(); // consume key
                    self.advance(); // consume colon

                    if let Some(field) = fields.iter().find(|f| f.name == key) {
                        if let Some(val) = self.parse_typed_value(&field.type_def)? {
                            obj.insert(key, val);
                        } else {
                            return Err(self
                                .make_error(format!("Expected value for named field '{}'", key)));
                        }
                    } else if let Some(val) = self.parse_value()? {
                        obj.insert(key, val);
                    } else {
                        return Err(self.make_error(format!("Expected value for field '{}'", key)));
                    }
                }
            } else if field_idx < fields.len() {
                let field = &fields[field_idx];
                if let Some(val) = self.parse_typed_value(&field.type_def)? {
                    obj.insert(field.name.clone(), val);
                    field_idx += 1;
                } else {
                    // Check if it's a directive - end of row
                    if let Some(st) = &self.current_token
                        && let Token::Directive(_) = st.token
                    {
                        break;
                    }
                    return Err(
                        self.make_error(format!("Expected value for field '{}'", field.name))
                    );
                }
            } else {
                // Extra tokens for this row - belong to next row
                break;
            }
        }

        if obj.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Value::Object(obj)))
        }
    }

    fn parse_typed_value(&mut self, type_def: &TypeDef) -> Result<Option<Value>, ParseError> {
        match type_def {
            TypeDef::Scalar => self.parse_value(),
            TypeDef::Object(type_name) => {
                if self.check(&Token::LBrace) {
                    self.advance(); // Skip {
                    let old_shape = self.active_shape.clone();
                    self.active_shape = Some(type_name.clone());

                    let res = self.parse_row()?;

                    if self.check(&Token::RBrace) {
                        self.advance();
                    } else {
                        return Err(self.make_error("Expected '}' for typed object"));
                    }

                    self.active_shape = old_shape;
                    Ok(res)
                } else {
                    Ok(None)
                }
            }
            TypeDef::List(type_name) => {
                if self.check(&Token::LBracket) {
                    self.advance(); // Skip [
                    let mut list = Vec::new();
                    let old_shape = self.active_shape.clone();
                    self.active_shape = Some(type_name.clone());

                    while let Some(st) = &self.current_token {
                        match st.token {
                            Token::RBracket => {
                                self.advance();
                                break;
                            }
                            Token::Newline | Token::Semi => {
                                self.advance();
                            }
                            Token::RBrace => {
                                return Err(
                                    self.make_error("Unexpected '}' in list - mismatched brackets")
                                );
                            }
                            _ => {
                                // Check for optional brace wrapper
                                let is_braced = self.check(&Token::LBrace);
                                if is_braced {
                                    self.advance(); // Skip {
                                    // Skip newlines after {
                                    while self.check(&Token::Newline) || self.check(&Token::Semi) {
                                        self.advance();
                                    }
                                }

                                if let Some(row) = self.parse_row()? {
                                    list.push(row);
                                } else if !is_braced {
                                    self.advance();
                                }

                                if is_braced {
                                    // Skip newlines before }
                                    while self.check(&Token::Newline) || self.check(&Token::Semi) {
                                        self.advance();
                                    }

                                    if self.check(&Token::RBrace) {
                                        self.advance();
                                    } else {
                                        return Err(
                                            self.make_error("Expected '}' for item in typed list")
                                        );
                                    }
                                }
                            }
                        }
                    }
                    self.active_shape = old_shape;
                    Ok(Some(Value::Array(list)))
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn parse_map_entry(&mut self) -> Result<Option<Value>, ParseError> {
        if let Some(st) = self.current_token.clone()
            && let Token::Ident(key) = st.token
        {
            self.advance();

            // Optional colon
            if self.check(&Token::Colon) {
                self.advance();
            }

            if let Some(val) = self.parse_value()? {
                let mut obj = Map::new();
                obj.insert(key, val);
                return Ok(Some(Value::Object(obj)));
            }
        }
        Ok(None)
    }

    fn parse_value(&mut self) -> Result<Option<Value>, ParseError> {
        let val = if let Some(st) = &self.current_token {
            match &st.token {
                Token::String(s) => Some(Value::String(s.clone())),
                Token::Number(n) => Some(Value::Number(
                    serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0)),
                )),
                Token::Bool(b) => Some(Value::Bool(*b)),
                Token::Null => Some(Value::Null),
                Token::Ident(s) => Some(Value::String(s.clone())),
                Token::LBracket => return self.parse_list(),
                Token::LBrace => return self.parse_object(),
                _ => None,
            }
        } else {
            None
        };

        if val.is_some() {
            self.advance();
        }
        Ok(val)
    }

    fn parse_list(&mut self) -> Result<Option<Value>, ParseError> {
        self.advance(); // Skip [
        let mut list = Vec::new();

        // Save the outer active_shape and start with None inside the array
        let outer_shape = self.active_shape.clone();
        let mut array_shape: Option<String> = None;

        loop {
            if let Some(st) = &self.current_token {
                match &st.token {
                    Token::RBracket => {
                        self.advance(); // Skip ]
                        self.active_shape = outer_shape; // Restore outer shape
                        return Ok(Some(Value::Array(list)));
                    }
                    Token::Newline | Token::Semi => {
                        self.advance();
                        continue;
                    }
                    Token::RBrace => {
                        return Err(self.make_error("Unexpected '}' in list - mismatched brackets"));
                    }
                    Token::Colon => {
                        self.advance();
                        continue;
                    }
                    Token::Directive(d) if d == "use" => {
                        // !use SchemaName inside array - sets schema for subsequent elements
                        self.advance(); // Skip !use
                        if let Some(st2) = &self.current_token {
                            if let Token::Ident(shape_name) = &st2.token {
                                let shape_name = shape_name.clone();
                                if !self.context.shapes.borrow().contains_key(&shape_name) {
                                    return Err(self.make_error(format!(
                                        "!use references undefined schema '{}' in array",
                                        shape_name
                                    )));
                                }
                                array_shape = Some(shape_name);
                                self.advance(); // Skip schema name
                                continue;
                            }
                        }
                        return Err(self.make_error("!use in array requires a schema name"));
                    }
                    _ => {
                        // If we have an active array schema, parse as schema row
                        if let Some(ref shape_name) = array_shape {
                            self.active_shape = Some(shape_name.clone());
                            if let Some(row) = self.parse_row()? {
                                list.push(row);
                            }
                            // Don't clear active_shape - subsequent rows use same schema
                        } else if let Some(val) = self.parse_value()? {
                            list.push(val);
                        } else {
                            return Err(self.make_error("Expected value in list or ']'"));
                        }
                    }
                }
            } else {
                return Err(self.make_error("Unclosed list: expected ']'"));
            }
        }
    }

    fn parse_object(&mut self) -> Result<Option<Value>, ParseError> {
        self.advance(); // Skip {
        let mut map = Map::new();
        loop {
            if let Some(st) = &self.current_token {
                match st.token {
                    Token::RBrace => {
                        self.advance(); // Skip }
                        return Ok(Some(Value::Object(map)));
                    }
                    Token::Newline | Token::Semi => {
                        self.advance();
                        continue;
                    }
                    _ => {
                        // Expect Key Value
                        let key = if let Token::Ident(k) = &st.token {
                            k.clone()
                        } else if let Token::String(k) = &st.token {
                            k.clone()
                        } else {
                            return Err(self.make_error(format!(
                                "Expected key in object, got {:?}",
                                st.token
                            )));
                        };

                        self.advance();

                        // Optional colon
                        if self.check(&Token::Colon) {
                            self.advance();
                        }

                        if let Some(val) = self.parse_value()? {
                            map.insert(key, val);
                        } else {
                            return Err(self.make_error("Expected value for key"));
                        }
                    }
                }
            } else {
                return Err(self.make_error("Unclosed object: expected '}'"));
            }
        }
    }
}
