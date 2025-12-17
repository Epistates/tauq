# Adaptive Formatter: World-Class Design

## Vision

**One elegant Tauq format** with an intelligent formatter that automatically makes optimal token decisions.

Users don't choose "modes" - they get optimal output by default, with minimal overrides when needed.

## Core Principle

**!def is an optimization feature, not a requirement.**

The formatter should:
- **Default**: Automatically use !def when it reduces tokens
- **Override**: `--no-schemas` to force inline key:value everywhere
- **Transparent**: `--explain` to show what decisions were made

## User Experience

```bash
# Default: Automatically optimal
tauq format data.json
→ Intelligently uses !def when beneficial

# Force inline everywhere
tauq format data.json --no-schemas

# See what it chose
tauq format data.json --explain
→ employees[100]: Schema saved 420 tokens
→ config[3]: Inline (schema would cost +12 tokens)
→ Total: 1,234 tokens

# Combine with formatting options
tauq format data.json --no-schemas --comma --minify
```

## Formatting Concerns (Orthogonal)

Three independent dimensions:

1. **Schema Strategy**
   - `adaptive` (default): Use !def when it saves tokens
   - `--no-schemas`: Never use !def

2. **Delimiter** (for schema rows)
   - `space` (default): `1 Alice Engineering`
   - `--comma`: `1,Alice,Engineering`

3. **Whitespace**
   - `pretty` (default): Multi-line, indented
   - `--minify`: Single-line, minimal whitespace

Users compose what they need:
```bash
tauq format data.json --no-schemas --comma --minify
```

## Formatter Architecture

### Current Structure (Problematic)

```rust
pub struct Formatter {
    minify: bool,
    indent_size: usize,
    delimiter: Delimiter,
    no_def: bool,  // ← Boolean flag
}

// Multiple confusing constructors
impl Formatter {
    pub fn new() -> Self { ... }
    pub fn simple() -> Self { ... }
    pub fn token_optimized() -> Self { ... }
    pub fn ultra_compact() -> Self { ... }
}
```

**Problems**:
- `no_def: bool` doesn't express "adaptive" vs "never"
- Multiple constructors create confusion ("which one do I use?")
- No way to represent "always use schemas" (for testing)

### Proposed Structure (Elegant)

```rust
pub struct Formatter {
    delimiter: Delimiter,
    minify: bool,
    indent_size: usize,
    schema_strategy: SchemaStrategy,
}

pub enum SchemaStrategy {
    /// Automatically use !def when it reduces tokens (default)
    Adaptive,

    /// Never use !def schemas (--no-schemas flag)
    Never,

    /// Always use !def when possible (for testing/debugging)
    Always,
}

impl Formatter {
    /// Create formatter with sensible defaults
    pub fn new() -> Self {
        Self {
            delimiter: Delimiter::Space,
            minify: false,
            indent_size: 2,
            schema_strategy: SchemaStrategy::Adaptive,  // ← Default
        }
    }

    // Builder methods (chainable)
    pub fn without_schemas(mut self) -> Self {
        self.schema_strategy = SchemaStrategy::Never;
        self
    }

    pub fn always_schemas(mut self) -> Self {
        self.schema_strategy = SchemaStrategy::Always;
        self
    }

    pub fn with_comma_delimiter(mut self) -> Self {
        self.delimiter = Delimiter::Comma;
        self
    }

    pub fn minified(mut self) -> Self {
        self.minify = true;
        self
    }

    pub fn with_indent(mut self, size: usize) -> Self {
        self.indent_size = size;
        self
    }
}

// Usage
let formatter = Formatter::new()
    .without_schemas()
    .with_comma_delimiter()
    .minified();
```

## Adaptive Algorithm (Two-Phase)

### Phase 1: Document Analysis

Scan entire JSON tree and identify potential schemas:

```rust
struct SchemaCandidate {
    signature: String,           // "id,name,email,age"
    fields: Vec<String>,         // ["id", "name", "email", "age"]
    arrays: Vec<ArrayLocation>,  // Where this schema appears
    total_objects: usize,        // Total objects across all arrays
}

fn analyze_schemas(value: &Value) -> HashMap<String, SchemaCandidate> {
    let mut candidates = HashMap::new();

    // Walk entire tree
    walk_tree(value, &mut |array_location, objects| {
        if let Some(fields) = detect_uniform_fields(objects) {
            let sig = fields.join(",");
            candidates.entry(sig)
                .or_insert_with(|| SchemaCandidate::new(fields))
                .add_location(array_location, objects.len());
        }
    });

    candidates
}
```

### Phase 2: Token-Optimal Formatting

For each array, generate both versions and pick the smaller:

```rust
struct FormattingChoice {
    with_schema: String,
    without_schema: String,
    schema_tokens: usize,
    inline_tokens: usize,
}

impl FormattingChoice {
    fn optimal(&self, strategy: &SchemaStrategy) -> &str {
        match strategy {
            SchemaStrategy::Adaptive => {
                if self.schema_tokens < self.inline_tokens {
                    &self.with_schema
                } else {
                    &self.without_schema
                }
            }
            SchemaStrategy::Never => &self.without_schema,
            SchemaStrategy::Always => &self.with_schema,
        }
    }

    fn savings(&self) -> isize {
        self.inline_tokens as isize - self.schema_tokens as isize
    }

    fn used_schema(&self, strategy: &SchemaStrategy) -> bool {
        self.optimal(strategy) == &self.with_schema
    }
}

fn format_array_with_choice(
    arr: &[Value],
    schema_candidate: Option<&SchemaCandidate>,
    formatter: &Formatter
) -> FormattingChoice {
    // Generate schema version (if applicable)
    let with_schema = if let Some(candidate) = schema_candidate {
        format_with_schema(arr, candidate, formatter)
    } else {
        String::new()  // No schema possible
    };
    let schema_tokens = estimate_tokens(&with_schema);

    // Generate inline version
    let without_schema = format_inline_objects(arr, formatter);
    let inline_tokens = estimate_tokens(&without_schema);

    FormattingChoice {
        with_schema,
        without_schema,
        schema_tokens,
        inline_tokens,
    }
}
```

### Token Estimation

Start with fast heuristic:

```rust
/// Fast token estimation (≈90% accurate)
fn estimate_tokens(text: &str) -> usize {
    text.split(|c: char| c.is_whitespace() || "[]{}():,\"".contains(c))
        .filter(|s| !s.is_empty())
        .count()
}
```

Later: Add tiktoken integration for exact counts (slower but accurate).

## CLI Design

### Current (Confusing)

```bash
tauq format data.json                # Standard
tauq format data.json --simple       # No schemas
tauq format data.json --optimized    # Comma-delimited
tauq format data.json --ultra        # Comma + minified
```

Users think these are different "modes."

### Proposed (Clean)

```bash
# Default: adaptive schemas, space-delimited, pretty
tauq format data.json

# Schema control
tauq format data.json --no-schemas

# Delimiter control
tauq format data.json --comma

# Whitespace control
tauq format data.json --minify

# Combine orthogonal options
tauq format data.json --no-schemas --comma --minify

# Explain decisions
tauq format data.json --explain
```

### CLI Flag Mapping

| Flag | Effect |
|------|--------|
| (none) | Adaptive schemas, space delimiter, pretty |
| `--no-schemas` | Force inline key:value everywhere |
| `--comma` | Use comma delimiter in schema rows |
| `--minify` | Single-line output |
| `--explain` | Show token-savings analysis |

## --explain Output

```bash
$ tauq format large-dataset.json --explain

Schema Decisions:
─────────────────────────────────────────────────────────────
Array                Objects   Choice    Tokens    Savings
─────────────────────────────────────────────────────────────
employees            100       Schema    1,234     -420
departments          3         Inline    156       +12
nested.config        1         Inline    48        +8
nested.replicas      2         Inline    92        +3
─────────────────────────────────────────────────────────────

Total: 1,530 tokens
  vs All Schemas: 1,543 tokens (+13)
  vs All Inline:  1,950 tokens (+420)

Adaptive choice saved 420 tokens vs all-inline (27% reduction)
```

## Implementation Plan

### Phase 1: Refactor to SchemaStrategy (This PR)

1. Replace `no_def: bool` with `schema_strategy: SchemaStrategy`
2. Remove confusing constructors (`simple()`, `token_optimized()`, etc.)
3. Keep clean builder pattern
4. Update CLI to use `--no-schemas` instead of `--simple`
5. Remove "simple mode" terminology everywhere

### Phase 2: Intelligent Adaptive (Next PR)

1. Implement two-phase analysis:
   - Phase 1: Scan document, find schema candidates
   - Phase 2: Generate both versions, count tokens, pick smaller
2. Add fast token estimation
3. Add `--explain` flag
4. Document the algorithm

### Phase 3: Advanced Optimization (Future)

1. Integrate tiktoken for exact token counts
2. Add schema threshold tuning (`--schema-threshold=3`)
3. Add "fork" optimization (generate incrementally, abort early if one path is clearly losing)
4. Benchmark and optimize performance

## Migration Path

**Backwards compatibility**:
- Keep `--optimized` as alias for `--comma`
- Keep `--ultra` as alias for `--comma --minify`
- Deprecate `--simple` → suggest `--no-schemas`

**Code API**:
```rust
// Old (deprecated)
Formatter::simple()
Formatter::token_optimized()
Formatter::ultra_compact()

// New (encouraged)
Formatter::new().without_schemas()
Formatter::new().with_comma_delimiter()
Formatter::new().with_comma_delimiter().minified()
```

## Testing Strategy

**Unit tests**:
```rust
#[test]
fn test_adaptive_chooses_schema_when_beneficial() {
    let data = generate_100_employees();
    let formatter = Formatter::new(); // Adaptive
    let output = formatter.format(&data);

    assert!(output.contains("!def"));  // Should use schema
}

#[test]
fn test_adaptive_chooses_inline_when_better() {
    let data = json!({"config": [{"key": "value"}]});
    let formatter = Formatter::new(); // Adaptive
    let output = formatter.format(&data);

    assert!(!output.contains("!def"));  // Should use inline
}

#[test]
fn test_no_schemas_never_uses_def() {
    let data = generate_1000_employees();
    let formatter = Formatter::new().without_schemas();
    let output = formatter.format(&data);

    assert!(!output.contains("!def"));  // Never use schema
}
```

**Benchmark tests**:
```rust
#[bench]
fn bench_adaptive_large_dataset(b: &mut Bencher) {
    let data = generate_large_dataset();
    let formatter = Formatter::new();
    b.iter(|| formatter.format(&data));
}
```

## Success Criteria

1. **User clarity**: "I just run `tauq format` and it does the right thing"
2. **Provably optimal**: Adaptive mode generates both, picks smaller
3. **Transparent**: `--explain` shows what decisions were made
4. **Override when needed**: `--no-schemas` for LLM testing
5. **Clean API**: Builder pattern, no confusing modes

## Documentation Updates

### README.md

```markdown
## Intelligent Formatting

Tauq automatically decides when to use schema definitions:

\`\`\`bash
tauq format data.json  # Adaptive: uses !def when beneficial
\`\`\`

Override when needed:
\`\`\`bash
tauq format data.json --no-schemas  # Force inline key:value
\`\`\`

See what it chose:
\`\`\`bash
tauq format data.json --explain
\`\`\`
```

### Spec (tauq_spec.md)

```markdown
## Schema Definitions (!def)

Schema definitions are **optional** and should be used when they reduce token count:

\`\`\`tauq
!def User id name email
---
users [
  !use User
  1 Alice alice@example.com
  2 Bob bob@example.com
]
\`\`\`

**When to use !def**:
- Schema appears 2+ times in document
- Token savings outweigh overhead

**When to skip !def**:
- Small arrays (< 50 objects)
- One-time schemas
- LLM applications prioritizing comprehension

Formatters SHOULD automatically determine optimal usage by comparing token costs.
```

## Conclusion

This design creates **one elegant Tauq format** where `!def` is an intelligent optimization, not a "mode" users think about.

**Default behavior**: Just works optimally.
**Override when needed**: `--no-schemas` for LLM testing.
**Transparent**: `--explain` shows decisions.

World-class DX.

---

**Next Step**: Implement Phase 1 (Refactor to SchemaStrategy)
