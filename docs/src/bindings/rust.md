# Rust Bindings

This is the native library.

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
tauq = "0.1.0"
```

## Usage

```rust
use tauq;

fn main() {
    // Note: !def implies !use, so data rows immediately follow
    let input = r#"
!def User id name
1 Alice
2 Bob
"#;

    // 1. Parse (Returns serde_json::Value)
    let json_val = tauq::compile_tauq(input).unwrap();
    println!("{:?}", json_val);

    // 2. Format (JSON -> Tauq)
    let tqn = tauq::format_to_tauq(&json_val);
    println!("{}", tqn);

    // 3. Exec Query (with safe_mode=true to disable shell execution)
    let res = tauq::compile_tauqq("!def T x\n1\n2", true).unwrap();
    println!("{:?}", res);

    // 4. Minify
    let min = tauq::minify_tauq_str(&json_val);
    println!("{}", min);
}
```
