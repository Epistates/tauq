# Security & Robustness

Tauq is designed for production environments where security and stability are paramount. As a replacement for JSON in critical data paths, we adhere to strict engineering standards.

## Security Model

### DoS Prevention
The Tauq parser enforces hard limits to prevent Denial of Service (DoS) attacks via memory exhaustion or deep recursion stack overflow:
- **Max Input Size:** 100 MB (`MAX_INPUT_SIZE`)
- **Max Nesting Depth:** 100 levels (`MAX_NESTING_DEPTH`)

### Safe Execution (TQQ)
Tauq Query (TQQ) supports powerful scripting capabilities, including shell execution.
- **Default Safe Mode:** `compile_tauqq_safe()` is the default entry point, which disables `!emit`, `!run`, and `!pipe` directives.
- **Explicit Opt-In:** Shell execution requires explicit `unsafe` opt-in via `compile_tauqq_unsafe()` or CLI flags.

## Continuous Fuzzing

To ensure the highest level of robustness, Tauq employs **Continuous Fuzz Testing** using `cargo-fuzz` (libFuzzer).

### Infrastructure
Our fuzzing infrastructure targets the most critical attack surfaces:
1.  **Text Parser (`fuzz_parser`)**: Generates random byte sequences to ensure the TQN parser never panics, even on malformed UTF-8 or deep nesting.
2.  **Binary Decoder (`fuzz_tbf`)**: Bombards the TBF decoder with malformed binary data to verify it safely rejects invalid varints, schemas, or corrupted columnar data without memory safety violations.

### Running Fuzz Tests
Security researchers or contributors can run the fuzz suite locally:

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Run the parser fuzzer
cargo fuzz run fuzz_parser

# Run the binary format fuzzer
cargo fuzz run fuzz_tbf
```

### CI Integration
Fuzz "smoke tests" run on every Pull Request to ensure no regressions are introduced in parser stability.

## Memory Safety
Tauq is written in **Rust**, guaranteeing memory safety (no buffer overflows, use-after-free) in safe code. `unsafe` blocks are minimized and audited, primarily used for:
- SIMD optimizations in TBF (e.g., fast varint decoding)
- Zero-copy deserialization optimizations
