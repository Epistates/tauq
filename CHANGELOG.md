# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-10

### Security

- **Rhai engine resource limits**: Added operation count, call depth, string size, array size, map size, and expression depth limits to the query engine. Disabled `eval` to prevent code injection.
- **Python bindings safe-mode default**: `exec_tauqq` now runs in safe mode by default (shell execution disabled). Added explicit `exec_tauqq_unsafe` for trusted contexts.
- **Allocation amplification protection**: Added caps to string dictionary (1M entries), schema fields (10K), and all batch decode functions to prevent crafted inputs from triggering unbounded memory allocation.
- **Environment variable sanitization**: TauqQ shell execution now calls `.env_clear()` before injecting the safe variable allowlist, preventing unintended environment inheritance.
- **Bloom filter hash unpredictability**: Replaced `DefaultHasher` with `ahash::AHasher` and added per-instance random seeds to prevent hash prediction attacks on bloom filters.
- **Import cycle prevention**: Added visited-file tracking and a `MAX_TOTAL_IMPORTS = 100` limit to prevent unbounded recursive `!import` directives.

### Fixed

- **StringDictionary hash collision corruption**: Replaced FNV-1a hash-keyed lookup map with direct string-keyed `HashMap<String, u32>`, eliminating silent data corruption on hash collisions.
- **UltraBuffer use-after-free on panic**: Rewrote `reserve_slow` using `ManuallyDrop` to prevent double-free if `Vec::reserve` panics. Removed unsound `unsafe impl Send/Sync`.
- **StreamingParser triple-dash handling**: `---` separator now correctly clears `active_shape` in the streaming parser, matching batch parser behavior.
- **Benchmark roundtrip fidelity**: `bench_roundtrip` now performs a true JSON -> Tauq -> JSON roundtrip via `compile_tauq`.
- **Float comparison masking**: Test assertions now use `.as_i64()` / `Some(N)` instead of `assert_eq!(val, N.0)` to detect integer-vs-float type bugs.
- **Unterminated string detection**: Lexer now records an error for unterminated string literals, surfaced by the parser at the end of `parse()`.

### Changed

- **Public API surface reduction**: Reduced TBF module exports from 60+ wildcard re-exports to focused, organized sections with clear documentation grouping.
- **Removed global `#![allow(dead_code)]`**: Replaced with targeted `#[allow(dead_code)]` on specific modules containing intentionally unused advanced/experimental code.
- **Removed unused dependencies**: Dropped `regex` from `[dependencies]`.
- **Removed dead code**: Deleted unused `Token::Eof` variant.

### Added

- **Lexer unit tests**: 52 tests covering all token types, escape sequences, UTF-8, comments, spans, and edge cases.
- **Decoder error-path tests**: 12 tests covering empty input, truncated data, wrong magic, bad version, invalid type tags, and position tracking.
- **Production edge case tests**: 17 tests replacing prior assertion-free stubs, covering null columns, deep nesting, malformed input resilience, Unicode roundtrips, TBF corruption handling, and decimal precision.

## [0.1.0] - 2025-01-01

### Added

- Initial release of Tauq: TQN text notation, TBF binary format, TQQ query language.
- Serde integration for direct serialization/deserialization.
- Schema-driven parsing with `!def`/`!use` directives.
- TBF binary format with varint encoding, string dictionary, columnar storage.
- Adaptive codec selection (RLE, delta, dictionary encoding).
- Bloom filters and predicate pushdown for columnar queries.
- CLI tool with build, format, query, exec, minify, prettify, and validate commands.
- FFI bindings: C, Python (PyO3), Java (JNI), plus community bindings for Go, C#, Swift, JavaScript/WASM.
- Apache Iceberg integration (feature-gated).
- LSP server for editor support.
- Derive macros (`tbf_derive`) for compile-time schema generation.
