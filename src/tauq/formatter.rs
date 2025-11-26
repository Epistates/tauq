// Tauq Formatter: JSON → Tauq
// Intelligently converts JSON to token-optimal Tauq syntax
//
// Schema strategy:
// - Top-level arrays → !def + implicit !use (rows follow directly)
// - Nested arrays in objects → !def at top, --- separator, !use inside arrays
// - This allows type switching within arrays
//
// Delimiter modes:
// - Space (default): Most readable, good token efficiency
// - Comma: Maximum token efficiency (matches TOON's density)

use serde_json::Value;
use std::collections::HashMap;

/// Delimiter used between values in schema rows
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Delimiter {
    /// Space-separated values (default): `1 Alice admin`
    Space,
    /// Comma-separated values (TOON-compatible density): `1,Alice,admin`
    Comma,
}

impl Default for Delimiter {
    fn default() -> Self {
        Delimiter::Space
    }
}

/// Schema information collected during formatting
#[derive(Clone, Debug)]
struct SchemaInfo {
    name: String,
    fields: Vec<String>,
}

/// Collect and deduplicate schemas, returning name for each unique field set
struct SchemaRegistry {
    /// Map from field signature to schema info
    schemas: HashMap<String, SchemaInfo>,
    /// Counter for unique naming
    name_counter: HashMap<String, usize>,
}

impl SchemaRegistry {
    fn new() -> Self {
        Self {
            schemas: HashMap::new(),
            name_counter: HashMap::new(),
        }
    }

    /// Get or create a schema for the given fields, using context for naming
    fn get_or_create(&mut self, fields: &[String], context: Option<&str>) -> String {
        // Create deterministic signature from sorted fields (for deduplication only)
        let mut sorted = fields.to_vec();
        sorted.sort();
        let sig = sorted.join(",");

        // Return existing schema if same shape
        if let Some(info) = self.schemas.get(&sig) {
            return info.name.clone();
        }

        // Generate name from context or fields
        let base = Self::derive_name(fields, context);
        let name = self.unique_name(&base);

        self.schemas.insert(
            sig,
            SchemaInfo {
                name: name.clone(),
                fields: fields.to_vec(), // Preserve original order!
            },
        );
        name
    }

    fn unique_name(&mut self, base: &str) -> String {
        let count = self.name_counter.entry(base.to_string()).or_insert(0);
        *count += 1;
        if *count == 1 {
            base.to_string()
        } else {
            format!("{}{}", base, count)
        }
    }

    fn derive_name(fields: &[String], context: Option<&str>) -> String {
        // Use context if provided (singularize + PascalCase)
        if let Some(ctx) = context {
            return Self::singularize(ctx);
        }

        // Infer from field patterns
        for f in fields {
            let lower = f.to_lowercase();
            if lower == "user_id" || lower == "userid" {
                return "User".to_string();
            }
            if lower == "product_id" || lower == "productid" {
                return "Product".to_string();
            }
        }

        let has_id = fields
            .iter()
            .any(|f| f.to_lowercase() == "id" || f.to_lowercase().ends_with("_id"));
        let has_name = fields.iter().any(|f| f.to_lowercase() == "name");

        if has_id && has_name {
            "Record".to_string()
        } else if has_id {
            "Item".to_string()
        } else if has_name {
            "Entry".to_string()
        } else {
            "Row".to_string()
        }
    }

    fn singularize(s: &str) -> String {
        let singular = if s.ends_with("ies") {
            format!("{}y", &s[..s.len() - 3])
        } else if s.ends_with('s') && !s.ends_with("ss") && s.len() > 1 {
            s[..s.len() - 1].to_string()
        } else {
            s.to_string()
        };

        // PascalCase
        let mut result = String::new();
        let mut cap_next = true;
        for c in singular.chars() {
            if c == '_' || c == '-' {
                cap_next = true;
            } else if cap_next {
                result.push(c.to_ascii_uppercase());
                cap_next = false;
            } else {
                result.push(c.to_ascii_lowercase());
            }
        }
        result
    }

    /// Get all schema definitions as !def lines
    fn definitions(&self, delimiter: Delimiter) -> Vec<String> {
        let mut defs: Vec<_> = self.schemas.values().collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name)); // Deterministic order

        let field_sep = match delimiter {
            Delimiter::Comma => ",",
            Delimiter::Space => " ",
        };

        defs.iter()
            .map(|s| format!("!def {} {}", s.name, s.fields.join(field_sep)))
            .collect()
    }

    fn is_empty(&self) -> bool {
        self.schemas.is_empty()
    }
}

pub struct Formatter {
    minify: bool,
    indent_size: usize,
    delimiter: Delimiter,
}

impl Formatter {
    pub fn new() -> Self {
        Self {
            minify: false,
            indent_size: 2,
            delimiter: Delimiter::Space,
        }
    }

    pub fn minified() -> Self {
        Self {
            minify: true,
            indent_size: 0,
            delimiter: Delimiter::Space,
        }
    }

    /// Comma-delimited mode: matches TOON/CSV visual style
    /// Note: Space-delimited (standard) is actually MORE token-efficient
    /// because cl100k_base tokenizes spaces better than commas
    pub fn token_optimized() -> Self {
        Self {
            minify: false,
            indent_size: 2,
            delimiter: Delimiter::Comma,
        }
    }

    /// Token-optimized + minified for absolute minimum tokens
    pub fn ultra_compact() -> Self {
        Self {
            minify: true,
            indent_size: 0,
            delimiter: Delimiter::Comma,
        }
    }

    pub fn with_indent(indent_size: usize) -> Self {
        Self {
            minify: false,
            indent_size,
            delimiter: Delimiter::Space,
        }
    }

    pub fn with_delimiter(mut self, delimiter: Delimiter) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Get the value separator string based on delimiter
    fn value_sep(&self) -> &'static str {
        match self.delimiter {
            Delimiter::Space => " ",
            Delimiter::Comma => ",",
        }
    }

    /// Format JSON value to Tauq syntax
    pub fn format(&self, value: &Value) -> String {
        let mut registry = SchemaRegistry::new();
        let sep = if self.minify { ";" } else { "\n" };

        // Check if this is a top-level array of uniform objects
        // Use schema syntax with implicit !use (rows follow !def directly)
        if let Value::Array(arr) = value {
            if let Some(fields) = self.detect_uniform_objects(arr) {
                let schema_name = registry.get_or_create(&fields, None);
                return self.format_top_level_table(arr, &fields, &schema_name);
            }
            // Handle heterogeneous array at top level
            return self.format_heterogeneous_array(arr, &registry, 0);
        }

        // For objects/other values: collect schemas from nested arrays first
        self.collect_schemas(value, &mut registry, None);

        // Format the body
        let body = self.format_with_schemas(value, &registry, 0, None);

        // If we have schemas, emit !def declarations, ---, then body
        if registry.is_empty() {
            body
        } else {
            let defs = registry.definitions(self.delimiter).join(sep);
            format!("{}{sep}---{sep}{body}", defs)
        }
    }

    /// Collect schemas from nested arrays (first pass)
    fn collect_schemas(&self, value: &Value, registry: &mut SchemaRegistry, context: Option<&str>) {
        match value {
            Value::Object(obj) => {
                for (key, val) in obj {
                    if let Value::Array(arr) = val {
                        if let Some(fields) = self.detect_uniform_objects(arr) {
                            registry.get_or_create(&fields, Some(key));
                        }
                        // Recurse into array elements
                        for item in arr {
                            self.collect_schemas(item, registry, Some(key));
                        }
                    } else {
                        self.collect_schemas(val, registry, Some(key));
                    }
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    self.collect_schemas(item, registry, context);
                }
            }
            _ => {}
        }
    }

    /// Format value using collected schemas (second pass)
    fn format_with_schemas(
        &self,
        value: &Value,
        registry: &SchemaRegistry,
        depth: usize,
        context: Option<&str>,
    ) -> String {
        match value {
            Value::Object(obj) => {
                let mut lines = Vec::new();
                for (key, val) in obj {
                    lines.push(self.format_field_with_schemas(key, val, registry, depth));
                }
                if self.minify {
                    lines.join(";")
                } else {
                    lines.join("\n")
                }
            }
            Value::Array(arr) if !arr.is_empty() => {
                self.format_array_with_schemas(arr, registry, depth, context)
            }
            Value::Array(_) => String::from("[]"),
            other => self.format_primitive(other),
        }
    }

    fn format_field_with_schemas(
        &self,
        key: &str,
        value: &Value,
        registry: &SchemaRegistry,
        depth: usize,
    ) -> String {
        let formatted_key = self.format_key(key);

        let formatted_value = match value {
            // Recursively apply schema logic to nested objects
            Value::Object(obj) => self.format_object_with_schemas(obj, registry, depth),
            Value::Array(arr) => self.format_array_with_schemas(arr, registry, depth + 1, Some(key)),
            other => self.format_value_standard(other, depth),
        };

        if self.minify {
            format!("{} {}", formatted_key, formatted_value)
        } else {
            let indent = " ".repeat(depth * self.indent_size);
            format!("{}{} {}", indent, formatted_key, formatted_value)
        }
    }

    /// Format nested object while preserving schema logic for arrays inside
    fn format_object_with_schemas(
        &self,
        obj: &serde_json::Map<String, Value>,
        registry: &SchemaRegistry,
        depth: usize,
    ) -> String {
        if obj.is_empty() {
            return "{}".to_string();
        }

        let mut fields = Vec::new();
        for (key, value) in obj {
            fields.push(self.format_field_with_schemas(key, value, registry, depth + 1));
        }

        if self.minify {
            format!("{{{}}}", fields.join(";"))
        } else {
            let close_indent = " ".repeat(depth * self.indent_size);
            format!("{{\n{}\n{}}}", fields.join("\n"), close_indent)
        }
    }

    fn format_array_with_schemas(
        &self,
        arr: &[Value],
        registry: &SchemaRegistry,
        depth: usize,
        context: Option<&str>,
    ) -> String {
        if arr.is_empty() {
            return "[]".to_string();
        }

        // Check if this array has uniform objects with a schema
        if let Some(fields) = self.detect_uniform_objects(arr) {
            // Find the schema for these fields
            let mut sorted = fields.clone();
            sorted.sort();
            let sig = sorted.join(",");

            if let Some(schema_info) = registry.schemas.get(&sig) {
                // Use !use inside array with schema rows
                return self.format_schema_array(arr, &schema_info.name, &schema_info.fields, depth);
            }
        }

        // No schema - check if heterogeneous objects
        if arr.iter().all(|v| v.is_object()) {
            return self.format_heterogeneous_array(arr, registry, depth);
        }

        // Regular array of primitives/mixed
        let elements: Vec<String> = arr
            .iter()
            .map(|v| self.format_with_schemas(v, registry, depth, context))
            .collect();
        format!("[{}]", elements.join(" "))
    }

    /// Format heterogeneous array (objects with different shapes)
    fn format_heterogeneous_array(
        &self,
        arr: &[Value],
        registry: &SchemaRegistry,
        depth: usize,
    ) -> String {
        if arr.is_empty() {
            return "[]".to_string();
        }

        let mut elements = Vec::new();
        for item in arr {
            match item {
                Value::Object(obj) => {
                    // Format each object inline
                    let obj_str = self.format_inline_object(obj, registry, depth + 1);
                    elements.push(obj_str);
                }
                other => {
                    elements.push(self.format_primitive(other));
                }
            }
        }

        if self.minify {
            format!("[{}]", elements.join(" "))
        } else {
            let item_indent = " ".repeat(depth * self.indent_size);
            let close_indent = " ".repeat((depth.saturating_sub(1)) * self.indent_size);
            let items = elements
                .iter()
                .map(|e| format!("{}{}", item_indent, e))
                .collect::<Vec<_>>()
                .join("\n");
            format!("[\n{}\n{}]", items, close_indent)
        }
    }

    /// Format an object for inline use in heterogeneous arrays
    fn format_inline_object(
        &self,
        obj: &serde_json::Map<String, Value>,
        _registry: &SchemaRegistry,
        _depth: usize,
    ) -> String {
        if obj.is_empty() {
            return "{}".to_string();
        }

        let fields: Vec<String> = obj
            .iter()
            .map(|(k, v)| {
                let key = self.format_key(k);
                let value = self.format_primitive(v);
                format!("{} {}", key, value)
            })
            .collect();

        format!("{{ {} }}", fields.join(" "))
    }

    /// Format array of uniform objects using !use inside array
    fn format_schema_array(
        &self,
        arr: &[Value],
        schema_name: &str,
        fields: &[String],
        depth: usize,
    ) -> String {
        let value_sep = self.value_sep();
        let mut rows = Vec::new();

        for item in arr {
            if let Some(obj) = item.as_object() {
                let values: Vec<String> = fields
                    .iter()
                    .filter_map(|key| obj.get(key))
                    .map(|v| self.format_value_for_row(v))
                    .collect();
                rows.push(values.join(value_sep));
            }
        }

        if self.minify {
            format!("[!use {};{}]", schema_name, rows.join(";"))
        } else {
            let row_indent = " ".repeat(depth * self.indent_size);
            let close_indent = " ".repeat((depth - 1) * self.indent_size);
            let rows_str = rows
                .iter()
                .map(|r| format!("{}{}", row_indent, r))
                .collect::<Vec<_>>()
                .join("\n");
            format!("[\n{}!use {}\n{}\n{}]", row_indent, schema_name, rows_str, close_indent)
        }
    }

    /// Format top-level array of uniform objects using !def (implicit !use)
    fn format_top_level_table(&self, arr: &[Value], fields: &[String], schema_name: &str) -> String {
        let sep = if self.minify { ";" } else { "\n" };
        let value_sep = self.value_sep();
        let field_sep = value_sep; // Use same separator for schema fields

        // Generate schema definition
        let def_line = format!("!def {} {}", schema_name, fields.join(field_sep));

        // Generate rows (implicit !use after !def)
        let mut rows = Vec::new();
        for item in arr {
            if let Some(obj) = item.as_object() {
                let values: Vec<String> = fields
                    .iter()
                    .filter_map(|key| obj.get(key))
                    .map(|v| self.format_value_for_row(v))
                    .collect();
                rows.push(values.join(value_sep));
            }
        }

        format!("{}{}{}", def_line, sep, rows.join(sep))
    }

    /// Format a value for use in a schema row (handles quoting based on delimiter)
    fn format_value_for_row(&self, value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => {
                if self.is_safe_bareword_for_row(s) {
                    s.clone()
                } else {
                    self.quote_string(s)
                }
            }
            Value::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|v| self.format_value_for_row(v)).collect();
                format!("[{}]", elements.join(" "))
            }
            Value::Object(obj) => {
                let fields: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        let key = self.format_key(k);
                        format!("{} {}", key, self.format_value_for_row(v))
                    })
                    .collect();
                format!("{{ {} }}", fields.join(" "))
            }
        }
    }

    fn format_value_standard(&self, value: &Value, depth: usize) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => {
                if self.is_safe_bareword(s) {
                    s.clone()
                } else {
                    self.quote_string(s)
                }
            }
            Value::Array(arr) => self.format_array_inline(arr, depth),
            Value::Object(obj) => self.format_object_inline(obj, depth),
        }
    }

    fn format_array_inline(&self, arr: &[Value], depth: usize) -> String {
        if arr.is_empty() {
            return "[]".to_string();
        }

        let elements: Vec<String> = arr
            .iter()
            .map(|v| self.format_value_standard(v, depth))
            .collect();
        format!("[{}]", elements.join(" "))
    }

    fn format_object_inline(&self, obj: &serde_json::Map<String, Value>, depth: usize) -> String {
        if obj.is_empty() {
            return "{}".to_string();
        }

        let mut fields = Vec::new();
        for (key, value) in obj {
            let formatted_key = self.format_key(key);
            let formatted_value = self.format_value_standard(value, depth + 1);
            fields.push(format!("{} {}", formatted_key, formatted_value));
        }

        if self.minify {
            format!("{{{}}}", fields.join(";"))
        } else {
            let indent = " ".repeat((depth + 1) * self.indent_size);
            let sep = format!("\n{}", indent);
            format!(
                "{{\n{}{}\n{}}}",
                indent,
                fields.join(&sep),
                " ".repeat(depth * self.indent_size)
            )
        }
    }

    /// Detect if array contains uniform objects suitable for schema
    fn detect_uniform_objects(&self, arr: &[Value]) -> Option<Vec<String>> {
        if arr.len() < 2 {
            return None; // Need at least 2 objects for schema to be beneficial
        }

        // All elements must be objects
        let objects: Vec<&serde_json::Map<String, Value>> =
            arr.iter().filter_map(|v| v.as_object()).collect();

        if objects.len() != arr.len() {
            return None; // Mixed types
        }

        // Extract keys from first object (preserve insertion order with preserve_order feature)
        let first_keys: Vec<String> = objects[0].keys().cloned().collect();

        if first_keys.is_empty() {
            return None; // Empty objects
        }

        // Check all objects have exactly the same keys (order-independent check)
        let first_keys_set: std::collections::HashSet<_> = first_keys.iter().collect();
        for obj in &objects[1..] {
            let keys_set: std::collections::HashSet<_> = obj.keys().collect();
            if keys_set != first_keys_set {
                return None; // Different shapes
            }
        }

        Some(first_keys)
    }

    /// Format primitive values (no nested structures)
    fn format_primitive(&self, value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => {
                if self.is_safe_bareword(s) {
                    s.clone()
                } else {
                    self.quote_string(s)
                }
            }
            Value::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|v| self.format_primitive(v)).collect();
                format!("[{}]", elements.join(" "))
            }
            Value::Object(obj) => {
                let fields: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        let key = self.format_key(k);
                        format!("{} {}", key, self.format_primitive(v))
                    })
                    .collect();
                if self.minify {
                    format!("{{{}}}", fields.join(";"))
                } else {
                    format!("{{ {} }}", fields.join(" "))
                }
            }
        }
    }

    /// Format a key (always more conservative quoting for keys)
    fn format_key(&self, s: &str) -> String {
        if self.is_valid_identifier(s) {
            s.to_string()
        } else {
            self.quote_string(s)
        }
    }

    /// Quote a string with proper escaping
    fn quote_string(&self, s: &str) -> String {
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        format!("\"{}\"", escaped)
    }

    /// Check if string is a valid identifier (for keys)
    fn is_valid_identifier(&self, s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        // Keywords need quoting
        if matches!(s, "true" | "false" | "null") {
            return false;
        }

        // Must start with letter or underscore
        let first = s.chars().next().unwrap();
        if !first.is_alphabetic() && first != '_' {
            return false;
        }

        // All chars must be alphanumeric, underscore, or hyphen
        s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    }

    /// Check if string can be a bareword value (more permissive than identifiers)
    fn is_safe_bareword(&self, s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        // Keywords need quoting
        if matches!(s, "true" | "false" | "null") {
            return false;
        }

        // Check if it looks like a number
        if s.parse::<f64>().is_ok() {
            return false;
        }

        // Must start with letter, underscore, or allowed special char
        let first = s.chars().next().unwrap();
        if !first.is_alphabetic() && first != '_' {
            return false;
        }

        // Allow: alphanumeric, underscore, hyphen, dot, @, /
        // These are commonly found in values like emails, paths, URLs
        // Note: space requires quoting (obvious), as do structural chars []{}:;"
        s.chars().all(|c| {
            c.is_alphanumeric()
                || c == '_'
                || c == '-'
                || c == '.'
                || c == '@'
                || c == '/'
                || c == '+'
        })
    }

    /// Check if string is safe as bareword in a row (considers delimiter)
    fn is_safe_bareword_for_row(&self, s: &str) -> bool {
        if !self.is_safe_bareword(s) {
            return false;
        }

        // If using comma delimiter, commas in values need quoting
        if self.delimiter == Delimiter::Comma && s.contains(',') {
            return false;
        }

        true
    }
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Format JSON value to Tauq (pretty, space-delimited)
pub fn json_to_tauq(value: &Value) -> String {
    Formatter::new().format(value)
}

/// Format JSON value to minified Tauq
pub fn minify_tauq(value: &Value) -> String {
    Formatter::minified().format(value)
}

/// Format JSON to token-optimized Tauq (comma-delimited for max efficiency)
pub fn json_to_tauq_optimized(value: &Value) -> String {
    Formatter::token_optimized().format(value)
}

/// Format JSON to ultra-compact Tauq (comma-delimited + minified)
pub fn json_to_tauq_ultra(value: &Value) -> String {
    Formatter::ultra_compact().format(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_simple_object() {
        let value = json!({"name": "Alice", "age": 30});
        let result = json_to_tauq(&value);

        assert!(result.contains("name Alice") || result.contains("age 30"));
    }

    #[test]
    fn test_format_preserves_field_order() {
        // With preserve_order feature, fields should maintain JSON order
        let value = json!([
            {"id": 1, "name": "Alice", "email": "alice@example.com"},
            {"id": 2, "name": "Bob", "email": "bob@example.com"}
        ]);
        let result = json_to_tauq(&value);

        // Should have id, name, email in that order (not alphabetized)
        assert!(
            result.contains("!def Record id name email") || result.contains("!def Record id,name,email"),
            "Expected field order id,name,email but got: {}",
            result
        );
    }

    #[test]
    fn test_format_with_strings() {
        let value = json!({"host": "localhost", "path": "/api/v1"});
        let result = json_to_tauq(&value);

        assert!(result.contains("host localhost"));
        // /api/v1 should not need quoting with improved bareword rules
        assert!(
            result.contains("path /api/v1") || result.contains("path \"/api/v1\""),
            "Path handling: {}", result
        );
    }

    #[test]
    fn test_format_table() {
        // Top-level array of uniform objects uses !def with implicit !use
        let value = json!([
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]);
        let result = json_to_tauq(&value);

        // Should use !def with implicit !use (no explicit !use needed)
        assert!(result.contains("!def Record"), "Expected !def, got: {}", result);
        assert!(!result.contains("!use"), "Should not have explicit !use: {}", result);
        assert!(result.contains("1 Alice"));
        assert!(result.contains("2 Bob"));
    }

    #[test]
    fn test_token_optimized_uses_commas() {
        let value = json!([
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]);
        let result = json_to_tauq_optimized(&value);

        // Should use comma-separated fields and values
        assert!(
            result.contains("!def Record id,name"),
            "Should have comma-separated fields: {}",
            result
        );
        assert!(result.contains("1,Alice"), "Should have comma-separated values: {}", result);
    }

    #[test]
    fn test_minify() {
        let value = json!({"a": 1, "b": 2});
        let result = minify_tauq(&value);

        assert!(result.contains(";"));
        assert!(!result.contains("\n"));
    }

    #[test]
    fn test_minify_table() {
        let value = json!([
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]);
        let result = minify_tauq(&value);

        // Top-level table with !def and implicit !use
        assert!(result.contains("!def Record"));
        assert!(!result.contains("!use"), "Should not have explicit !use");
        // Should be minified with semicolons
        assert!(result.contains(";"));
    }

    #[test]
    fn test_nested_array_uses_schema() {
        // Nested arrays use !def at top, --- separator, !use inside arrays
        let value = json!({
            "users": [
                {"id": 1, "name": "Alice", "role": "admin"},
                {"id": 2, "name": "Bob", "role": "user"}
            ]
        });
        let result = json_to_tauq(&value);

        // Should have !def with context-aware name (users -> User)
        assert!(result.contains("!def User"), "Should have !def User: {}", result);
        // Should have --- separator
        assert!(result.contains("---"), "Should have --- separator: {}", result);
        // Should have !use inside array
        assert!(result.contains("!use User"), "Should have !use User: {}", result);
        // Should have schema rows
        assert!(result.contains("1 Alice admin"), "Should have row data: {}", result);
    }

    #[test]
    fn test_top_level_vs_nested() {
        // Top-level array: uses schema with implicit !use
        let top_level = json!([
            {"id": 1, "name": "Alice"},
            {"id": 2, "name": "Bob"}
        ]);
        let top_result = json_to_tauq(&top_level);
        assert!(top_result.contains("!def"), "Top-level should use !def");
        // Top-level uses implicit !use (no explicit !use needed)
        assert!(!top_result.contains("!use"), "Top-level should use implicit !use");

        // Nested array: uses !def/---/!use pattern
        let nested = json!({
            "data": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ]
        });
        let nested_result = json_to_tauq(&nested);
        assert!(nested_result.contains("!def"), "Nested should use !def: {}", nested_result);
        assert!(nested_result.contains("---"), "Nested should have --- separator");
        assert!(nested_result.contains("!use"), "Nested should use explicit !use");
    }

    #[test]
    fn test_heterogeneous_array() {
        let value = json!([
            {"id": 1, "name": "Alice", "role": "admin"},
            {"id": 2, "name": "Bob", "department": "Engineering"},
            {"id": 3, "email": "carol@example.com"}
        ]);
        let result = json_to_tauq(&value);

        // Should format as individual objects, not use schema
        assert!(!result.contains("!def"), "Heterogeneous array should not use schema: {}", result);
        assert!(result.contains("{"), "Should have object notation");
        assert!(result.contains("id 1"), "Should have id field");
        assert!(result.contains("name Alice"), "Should have name field");
    }

    #[test]
    fn test_nested_array_round_trip() {
        // Test that formatter output can be parsed back
        let value = json!({
            "users": [
                {"id": 1, "name": "Alice", "role": "admin"},
                {"id": 2, "name": "Bob", "role": "user"}
            ]
        });
        let tauq_str = json_to_tauq(&value);

        // Parse it back
        let mut parser = crate::tauq::Parser::new(&tauq_str);
        let parsed = parser.parse().expect(&format!("Failed to parse: {}", tauq_str));

        // Parser returns object (possibly wrapped in array for consistency)
        let obj = if let Some(arr) = parsed.as_array() {
            // If array, get first item
            arr[0].as_object().expect("First item should be object")
        } else {
            parsed.as_object().expect("Result should be object or array")
        };

        // Verify users array
        let users = obj.get("users").expect("Should have users key");
        let users_arr = users.as_array().expect("users should be array");
        assert_eq!(users_arr.len(), 2, "Should have 2 users");

        // Check first user
        let first = users_arr[0].as_object().expect("user should be object");
        assert_eq!(first.get("name").unwrap(), "Alice");
        assert_eq!(first.get("role").unwrap(), "admin");

        // Check second user
        let second = users_arr[1].as_object().expect("user should be object");
        assert_eq!(second.get("name").unwrap(), "Bob");
        assert_eq!(second.get("role").unwrap(), "user");
    }

    #[test]
    fn test_array_value() {
        let value = json!({"tags": ["web", "api", "backend"]});
        let result = json_to_tauq(&value);

        assert!(result.contains("tags [web api backend]"));
    }

    #[test]
    fn test_nested_object() {
        let value = json!({
            "database": {
                "host": "localhost",
                "port": 5432
            }
        });
        let result = json_to_tauq(&value);

        // Should contain nested structure
        assert!(result.contains("database"));
        assert!(result.contains("host localhost"));
        assert!(result.contains("port 5432"));
    }

    #[test]
    fn test_bareword_detection() {
        let formatter = Formatter::new();

        // Valid barewords
        assert!(formatter.is_safe_bareword("hello"));
        assert!(formatter.is_safe_bareword("my_var"));
        assert!(formatter.is_safe_bareword("var123"));
        assert!(formatter.is_safe_bareword("user@example.com")); // @ allowed
        assert!(formatter.is_safe_bareword("path/to/file")); // / allowed
        assert!(formatter.is_safe_bareword("my-var")); // - allowed
        assert!(formatter.is_safe_bareword("file.txt")); // . allowed

        // Invalid barewords
        assert!(!formatter.is_safe_bareword("hello world")); // space
        assert!(!formatter.is_safe_bareword("123")); // number
        assert!(!formatter.is_safe_bareword("true")); // keyword
        assert!(!formatter.is_safe_bareword("")); // empty
        assert!(!formatter.is_safe_bareword("@user")); // starts with @
    }

    #[test]
    fn test_email_no_quote() {
        let value = json!({"email": "user@example.com"});
        let result = json_to_tauq(&value);

        // Email should not need quoting with improved bareword rules
        assert!(
            result.contains("email user@example.com"),
            "Email should be bareword: {}",
            result
        );
    }

    #[test]
    fn test_round_trip_simple() {
        // This tests that Tauq can parse what the formatter generates
        let original = json!({"name": "Test", "count": 42, "active": true});

        let tauq_str = json_to_tauq(&original);

        // Parse it back with Flux parser
        let mut parser = crate::tauq::Parser::new(&tauq_str);
        let parsed = parser.parse().unwrap();

        // Check values match (accounting for array wrapping)
        if let Some(arr) = parsed.as_array() {
            for item in arr {
                if let Some(obj) = item.as_object() {
                    if obj.contains_key("name") {
                        assert_eq!(obj["name"], "Test");
                    }
                    if obj.contains_key("count") {
                        assert_eq!(obj["count"], 42.0);
                    }
                    if obj.contains_key("active") {
                        assert_eq!(obj["active"], true);
                    }
                }
            }
        }
    }
}
