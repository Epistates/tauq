# Tauq Language Bindings

Tauq is designed to be ubiquitous. We provide first-class bindings for major languages, ensuring that you can parse, format, query, and minify Tauq data regardless of your tech stack.

All bindings expose the same core capabilities:
1.  **Parse**: `tauq_str` -> `json_str` (or native object)
2.  **Format**: `json_str` -> `tauq_str`
3.  **Exec**: `tqq_str` -> `json_str` (Tauq Query with scripting support)
4.  **Minify**: `tauq_str` -> `minified_tauq_str`
5.  **TBF (Binary)**: Full support for converting to and from the high-performance **Tauq Binary Format**.
6.  **Streaming**: Native support for processing incomplete data streams (available in JS and Python).

---

## Supported Languages

### 🐍 Python (`bindings/python`)
Built using [Maturin](https://github.com/PyO3/maturin). Features include full `TauqStream` for processing LLM token streams.

**Installation:**
```bash
pip install tauq
```

### 🌐 JavaScript / WebAssembly (`bindings/js`)
Built using [wasm-pack](https://github.com/rustwasm/wasm-pack). Works in Node.js and Browsers. Includes `TauqStream` support.

**Installation:**
```bash
npm install tauq
```

### 🐹 Go (`bindings/go`)
Uses CGO to link against the Rust core. Idiomatic `Marshal`/`Unmarshal` and TBF support.

**Installation:**
```bash
go get github.com/epistates/tauq/bindings/go
```

### ☕ Java (`bindings/java`)
JNI bindings with zero external dependencies. Supports TBF and standard parsing.

### 🔷 C# / .NET (`bindings/csharp`)
P/Invoke wrapper for cross-platform .NET Core 8 support. Full TBF and error reporting.

### 🐦 Swift (`bindings/swift`)
Swift Package Manager (SPM) integration with safe C interop. Supports `Data` for TBF.

### 🦀 Rust (`src/lib.rs`)
The native reference implementation. Use `tbf_derive` for compile-time schema generation.

---

## C API (FFI)

For other languages, the core library exposes a stable C ABI.

**Header:** [`include/tauq.h`](../include/tauq.h)

**Functions:**
- `char* tauq_to_json(const char* input)` - Parse Tauq to JSON string
- `char* tauq_exec_query(const char* input, bool safe_mode)` - Execute TauqQ with optional safe mode
- `char* tauq_minify(const char* input)` - Minify Tauq to single line
- `char* json_to_tauq_c(const char* input)` - Convert JSON string to Tauq
- `unsigned char* tauq_to_tbf(const char* input, size_t* out_len)` - Encode to TBF (binary)
- `char* tauq_tbf_to_json(const unsigned char* data, size_t len)` - Decode TBF to JSON string
- `size_t tauq_get_last_error(char* buffer, size_t size)` - Get last error message (thread-local)
- `void tauq_free_string(char* s)` - Free strings returned by tauq functions
- `void tauq_free_buffer(unsigned char* ptr, size_t len)` - Free binary buffers

**Note:** Functions returning pointers return `NULL` on error. Use `tauq_get_last_error` to retrieve error details.
