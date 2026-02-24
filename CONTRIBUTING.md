# Contributing to Tauq

We welcome contributions! Tauq is a high-performance, security-critical project. Please follow these guidelines to maintain "Google Grade" quality.

## Development Setup

1.  **Install Rust:** Ensure you have the latest stable Rust toolchain.
2.  **Clone the repo:**
    ```bash
    git clone https://github.com/epistates/tauq
    cd tauq
    ```
3.  **Run Tests:**
    ```bash
    cargo test
    ```

## Development Standards

-   **Code Style:** Run `cargo fmt` before committing.
-   **Linting:** Run `cargo clippy` and fix warnings.
-   **Testing:** Add unit tests for every new feature or bug fix.

## Security & Fuzzing

Tauq employs continuous fuzzing to ensure parser and decoder stability. **If you modify the parser (`src/tauq/`) or binary decoder (`src/tbf/`), you MUST run the fuzz tests.**

### Running Fuzz Tests

1.  **Install cargo-fuzz:**
    ```bash
    cargo install cargo-fuzz
    ```
2.  **Run the parser fuzzer:**
    ```bash
    # Run for a few minutes to catch obvious regressions
    cargo fuzz run fuzz_parser
    ```
3.  **Run the binary decoder fuzzer:**
    ```bash
    cargo fuzz run fuzz_tbf
    ```

### Adding New Fuzz Targets

If you add a new critical component, add a new target in `fuzz/fuzz_targets/`.

## Benchmarks

If you claim performance improvements, verify them:

```bash
# Run standard benchmarks
cargo bench

# Run token comparison
python3 benchmarks/token_benchmark.py
```

## Pull Request Process

1.  Fork the repository.
2.  Create a feature branch.
3.  Ensure `cargo test` and `cargo fmt -- --check` pass.
4.  Submit a PR with a clear description of changes.
