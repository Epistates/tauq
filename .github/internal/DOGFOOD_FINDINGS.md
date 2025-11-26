# Tauq Dogfood Testing Report

**Date:** 2025-11-24
**Version Tested:** tauq 0.1.0
**Tester:** Claude Code dogfood session

## Executive Summary

Comprehensive dogfood testing of `tqn` (Tauq Notation) and `tqq` (Tauq Query) tools. Created 25+ test files covering basic parsing, schemas, edge cases, error handling, round-trip conversions, TQQ pipelines, and real-world scenarios.

### Overall Assessment

| Category | Status | Notes |
|----------|--------|-------|
| Basic TQN Parsing | **PASS** | Key-value, nested objects, arrays all work |
| Schema/Table Data | **PARTIAL** | Basic works, complex nested schemas broken |
| Error Handling | **FAIL** | Silent failures, no error messages |
| Round-trip | **PASS** | JSON ↔ TQN preserves semantics |
| TQQ Emit/Pipe | **PASS** | Works when used correctly |
| TQQ Safe Mode | **PASS** | Correctly blocks shell execution |
| Query (Rhai) | **PASS** | Filter, map, property access work |

---

## Bugs Found

### Critical Bugs

#### 1. No Error on Unclosed Braces
**File:** `edge-cases/06_errors_syntax.tqn`
```tqn
broken {
  name test
  value 123
# missing closing brace
```
**Expected:** Parse error
**Actual:** Returns `[]` with exit code 0
**Impact:** Silent data loss - invalid syntax produces empty output

#### 2. No Error on Unclosed Strings
**File:** `edge-cases/07_errors_string.tqn`
```tqn
bad_string "this string never closes
next_line value
```
**Expected:** Parse error
**Actual:** Entire file content becomes part of string value
**Impact:** Subtle data corruption - multiline content absorbed into string

#### 3. No Error on Unclosed Arrays
**File:** `edge-cases/08_errors_array.tqn`
```tqn
bad_array [1 2 3 4
next_value 5
```
**Expected:** Parse error
**Actual:** Returns `[]` with exit code 0
**Impact:** Silent data loss

#### 4. Bareword `5g` Split Incorrectly
**File:** `real-world/02_ecommerce_catalog.tqn`
```tqn
tags [smartphone 5g flagship]
```
**Expected:** `["smartphone", "5g", "flagship"]`
**Actual:** `["smartphone", 5.0, "g", "flagship"]`
**Impact:** Data corruption - barewords starting with numbers get split

### Major Bugs

#### 5. Complex Nested Schemas Fail
**File:** `real-world/04_kubernetes_manifest.tqn`

Schemas with typed arrays of objects don't parse correctly:
```tqn
!schemas
Container name image ports:[ContainerPort]
Deployment name replicas containers:[Container]
```
**Result:** Containers become empty objects `{}`, fields get scattered incorrectly

#### 6. List Schema Type Parsing Wrong
**File:** `tqn/07_schemas_list.tqn`
```tqn
!use Project
"Website Redesign" active [{frontend blue} {urgent red}]
```
**Expected:** Array of Tag objects
**Actual:** Objects nested incorrectly with field names as keys

#### 7. Validate Command Doesn't Detect Errors
The `tauq validate` command reports "✓ Valid Tauq" for files with unclosed braces/strings/arrays.

### Minor Issues

#### 8. Schema Field Count Mismatch Silent
Rows with too few or too many fields are silently accepted:
- Too few: Missing fields omitted from output
- Too many: Extra fields ignored

Should at least warn users.

#### 9. All Integers Become Floats
```tqn
age 32
```
Becomes `"age": 32.0` in JSON output. May cause type issues downstream.

#### 10. Table Formatter Uses Generic Name
When formatting JSON arrays back to TQN tables, schema is named `T` instead of inferring from data:
```tqn
!def T email id name role   # Would be better as !def User
```

---

## Working Features

### TQN Features Verified Working

| Feature | Test File | Status |
|---------|-----------|--------|
| Key-value pairs | `01_basic_kvp.tqn` | ✅ |
| Quoted strings | `01_basic_kvp.tqn` | ✅ |
| Bareword strings | `01_basic_kvp.tqn` | ✅ |
| Numbers (int/float) | `01_basic_kvp.tqn` | ✅ |
| Booleans | `01_basic_kvp.tqn` | ✅ |
| Null values | `01_basic_kvp.tqn` | ✅ |
| Nested objects | `02_nested_objects.tqn` | ✅ |
| Deep nesting (6+ levels) | `03_deeply_nested.tqn` | ✅ |
| Simple arrays | `03_arrays.tqn` | ✅ |
| Empty arrays/objects | `05_empty_structures.tqn` | ✅ |
| String escapes | `01_string_escapes.tqn` | ✅ |
| Basic schemas (!def) | `04_schemas_basic.tqn` | ✅ |
| Schema switching (!use) | `05_schemas_multiple.tqn` | ✅ |
| Nested type schemas | `06_schemas_nested.tqn` | ✅ |
| Comments (#) | Various | ✅ |
| Minification | `minify` command | ✅ |
| Prettification | `prettify` command | ✅ |
| JSON output | `--json` flag | ✅ |

### TQQ Features Verified Working

| Feature | Test File | Status |
|---------|-----------|--------|
| !emit with echo | `13_emit_correct.tqq` | ✅ |
| !pipe with grep | `12_pipe_correct.tqq` | ✅ |
| Safe mode blocking | N/A | ✅ |
| Schema in TQQ | `12_pipe_correct.tqq` | ✅ |

### Query (Rhai) Features Verified Working

| Feature | Example | Status |
|---------|---------|--------|
| Property access | `.name` | ✅ |
| Filter | `.filter(\|x\| x.role == "admin")` | ✅ |
| Map | `.map(\|m\| m.value)` | ✅ |

---

## Test Files Created

### TQN Tests (`tqn/`)
1. `01_basic_kvp.tqn` - All primitive types
2. `02_nested_objects.tqn` - Deep nesting, multiple objects
3. `03_arrays.tqn` - Various array types
4. `04_schemas_basic.tqn` - Simple !def usage
5. `05_schemas_multiple.tqn` - Multiple schemas with !use
6. `06_schemas_nested.tqn` - Nested type references
7. `07_schemas_list.tqn` - Array type fields (broken)

### Edge Case Tests (`edge-cases/`)
1. `01_string_escapes.tqn` - Escape sequences, unicode
2. `02_numeric_edge_cases.tqn` - Large numbers, precision
3. `03_deeply_nested.tqn` - 6+ levels deep
4. `04_bareword_boundaries.tqn` - Reserved words, special chars
5. `05_empty_structures.tqn` - Empty strings/arrays/objects
6. `06_errors_syntax.tqn` - Unclosed brace (should fail)
7. `07_errors_string.tqn` - Unclosed string (should fail)
8. `08_errors_array.tqn` - Unclosed array (should fail)
9. `09_errors_schema.tqn` - Field count mismatch

### TQQ Tests (`tqq/`)
1. `01_emit_basic.tqq` - Simple emit
2. `02_emit_json.tqq` - Emit JSON
3. `03_emit_tqn.tqq` - Emit TQN
4. `04_pipe_basic.tqq` - Basic pipe (behavior test)
5. `05_pipe_grep.tqq` - Pipe with grep (fails)
6. `06_pipe_chain.tqq` - Multiple pipes (behavior test)
7. `07_pipe_jq.tqq` - Pipe with jq (fails)
8. `08_set_variable.tqq` - Variable setting
9. `09_env_variable.tqq` - Environment variables
10. `10_read_file.tqq` - File reading
11. `11_complex_pipeline.tqq` - Combined directives
12. `12_pipe_correct.tqq` - **Working** pipe usage
13. `13_emit_correct.tqq` - **Working** emit usage

### Real-World Tests (`real-world/`)
1. `01_api_config.tqn` - Full microservice config (~100 lines)
2. `02_ecommerce_catalog.tqn` - Product catalog with schemas
3. `03_log_analysis.tqq` - Log processing pipeline
4. `04_kubernetes_manifest.tqn` - K8s-like resources (broken)
5. `05_metrics_timeseries.tqn` - Time series metrics

---

## Recommendations

### High Priority (Fix Before Release)

1. **Implement proper error handling** for:
   - Unclosed braces `{`
   - Unclosed strings `"`
   - Unclosed arrays `[`
   - Return non-zero exit codes

2. **Fix bareword number splitting** - `5g` should stay as one token

3. **Fix complex nested schema parsing** - Arrays of typed objects

### Medium Priority

4. **Add warnings for**:
   - Schema field count mismatches
   - Undefined schemas referenced

5. **Fix validate command** to actually detect syntax errors

6. **Improve TQQ documentation** - The `!pipe` behavior (applies to following lines, not previous output) is non-obvious

### Low Priority

7. Consider preserving integer types instead of converting to float

8. Improve table formatter to use meaningful schema names

9. Add `--strict` mode for stricter parsing

---

## Token Efficiency Validation

Verified token efficiency claim with real-world API config:

| Format | Characters | Estimated Tokens |
|--------|------------|------------------|
| JSON equivalent | ~2,800 | ~700 |
| TQN format | ~1,100 | ~275 |
| **Savings** | **60%** | **~60%** |

The token efficiency claim is validated for configuration-style data.

---

## Conclusion

Tauq shows promise as a token-efficient format. The basic parsing, round-trip conversion, and query features work well. However, **critical error handling bugs** must be fixed before production use - silent failures on malformed input are dangerous. The complex nested schema feature needs work, and bareword parsing has an edge case bug.

Recommend:
1. Fix critical bugs (error handling, bareword parsing)
2. Add comprehensive test suite for error conditions
3. Improve documentation with more TQQ examples
4. Consider strict mode for production use cases
