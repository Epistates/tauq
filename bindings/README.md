# Tauq Language Bindings

Tauq is designed to be ubiquitous. We provide first-class bindings for major languages, ensuring that you can parse, format, query, and minify Tauq data regardless of your tech stack.

All bindings expose the same core capabilities:
1.  **Parse**: `tauq_str` -> `json_str` (or native object)
2.  **Format**: `json_str` -> `tauq_str`
3.  **Exec**: `tqq_str` -> `json_str` (Tauq Query with scripting support)
4.  **Minify**: `tauq_str` -> `minified_tauq_str`

---

## Supported Languages

### 🐍 Python (`bindings/python`)
Built using [Maturin](https://github.com/PyO3/maturin).

**Installation:**
```bash
pip install tauq
```

**Build from source:**
```bash
cd bindings/python
maturin develop
```

### 🌐 JavaScript / WebAssembly (`bindings/js`)
Built using [wasm-pack](https://github.com/rustwasm/wasm-pack). Works in Node.js and Browsers.

**Installation:**
```bash
npm install tauq
```

**Build from source:**
```bash
cd bindings/js
wasm-pack build --target nodejs
```

### 🐹 Go (`bindings/go`)
Uses CGO to link against the Rust core. Idiomatic `Marshal`/`Unmarshal`.

**Installation:**
```bash
go get github.com/epistates/tauq
```

**Build Notes:**
Requires `libtauq` (compiled via `cargo build --release`) to be available in your library path.

### ☕ Java (`bindings/java`)
JNI bindings with zero external dependencies.

**Build:**
```bash
cd bindings/java
./gradlew build
```

### 🔷 C# / .NET (`bindings/csharp`)
P/Invoke wrapper for cross-platform .NET Core / Framework support.

### 🐦 Swift (`bindings/swift`)
Swift Package Manager (SPM) integration with safe C interop.

### 🦀 Rust (`src/lib.rs`)
The native reference implementation.

**Installation:**
Add to `Cargo.toml`:
```toml
[dependencies]
tauq = "0.1.0"
```

---

## C API (FFI)

For other languages, the core library exposes a stable C ABI.

**Header:** [`include/tauq.h`](../include/tauq.h)

**Functions:**
- `char* tauq_to_json(const char* input)` - Parse Tauq to JSON string
- `char* tauq_exec_query(const char* input, bool safe_mode)` - Execute TauqQ with optional safe mode
- `char* tauq_minify(const char* input)` - Minify Tauq to single line
- `char* json_to_tauq_c(const char* input)` - Convert JSON string to Tauq
- `size_t tauq_get_last_error(char* buffer, size_t size)` - Get last error message
- `void tauq_free_string(char* s)` - Free strings returned by tauq functions

**Note:** All functions returning `char*` return `NULL` on error. Use `tauq_get_last_error` to retrieve error details. Caller must free non-NULL results with `tauq_free_string`.