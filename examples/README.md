# Tauq Examples

Comprehensive examples demonstrating all features of Tauq Notation (TQN) and Tauq Query (TQQ).

## Directory Structure

```
examples/
├── 1_basics/           # TQN fundamentals
│   ├── 01_primitives   # Strings, numbers, booleans, null, comments
│   └── 02_structures   # Objects and arrays
│
├── 2_schemas/          # Schema-driven data (Tauq's superpower)
│   ├── 01_basic        # !def, !use, implicit activation
│   ├── 02_typed        # Type annotations, nested types, lists
│   ├── 03_blocks       # !schemas block syntax
│   └── 04_nested       # !def/---/!use pattern for nested objects
│
├── 3_modularity/       # File organization
│   ├── main_config     # !import directive usage
│   └── modules/        # Reusable configuration modules
│
├── 4_queries/          # TQQ pre-processor (*.tqq files)
│   ├── 01_basics       # !set, !emit, !read, !json
│   ├── 02_pipelines    # !pipe, !run for data transformation
│   └── 03_transform    # Full data pipeline example
│
├── 5_real_world/       # Production-ready examples
│   ├── k8s_deployment  # Kubernetes manifest in Tauq
│   └── api_response    # LLM-optimized API data
│
└── 6_minified/         # Compact representations
    ├── 01_compact      # Semicolon syntax
    └── k8s.min         # Minified K8s deployment
```

## Quick Reference

### TQN Syntax

| Feature | Syntax | Example |
|---------|--------|---------|
| String | Bareword or `"quoted"` | `name Alice` or `name "Alice Smith"` |
| Number | Integer or float | `count 42` or `rate 3.14` |
| Boolean | `true` / `false` | `active true` |
| Null | `null` | `value null` |
| Array | `[...]` | `tags [web api]` |
| Object | `{...}` | `server { host localhost port 8080 }` |
| Comment | `#` | `# This is a comment` |

### Schema Directives

| Directive | Purpose | Example |
|-----------|---------|---------|
| `!def` | Define & activate schema | `!def User id name email` |
| `!use` | Activate schema (in arrays too) | `!use User` |
| `---` | Clear implicit schema scope | `!def User id name` then `---` |
| `!schemas` | Begin schema block | `!schemas` ... `---` |

### TQQ Directives

| Directive | Purpose | Example |
|-----------|---------|---------|
| `!set` | Set variable | `!set ENV production` |
| `!emit` | Run command, insert output | `!emit date` |
| `!pipe` | Filter remaining content | `!pipe sort` |
| `!run` | Execute code block | `!run python3 { print("hi") }` |
| `!import` | Include another file | `!import "config.tqn"` |
| `!json` | Convert JSON file to Tauq | `!json "data.json"` |
| `!read` | Insert file as string | `!read "template.txt"` |

## Running Examples

```bash
# Parse TQN to JSON
tauq examples/1_basics/01_primitives.tqn

# For import examples, run from the example directory
cd examples/3_modularity && tauq main_config.tqn

# Execute TQQ (requires shell access)
tauq examples/4_queries/03_data_transform.tqq

# Minify a file
tauq --minify examples/5_real_world/k8s_deployment.tqn
```

## Token Savings (Verified with tiktoken)

| Format | 1000 Records | Tokens | vs JSON |
|--------|--------------|--------|---------|
| JSON (minified) | 87 KB | 24,005 | baseline |
| TOON | 45 KB | 13,765 | -43% |
| **Tauq** | **43 KB** | **10,011** | **-58%** |

*All counts verified with tiktoken cl100k_base (GPT-4/Claude tokenizer).*

Tauq achieves these savings by:
1. Eliminating repeated keys via schemas (`!def`)
2. Using space delimiters (0 tokens) instead of commas (1 token each)
3. No count prefix required (unlike TOON's `[N]`)
4. Simpler schema syntax
