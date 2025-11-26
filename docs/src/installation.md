# Installation

Tauq provides a standalone CLI tool and bindings for many major languages.

## CLI Tool

The `tauq` CLI is the core tool for parsing, formatting, and querying Tauq data.

### Via Cargo (Rust)

If you have Rust installed:

```bash
cargo install tauq
```

### From Source

```bash
git clone https://github.com/epistates/tauq.git
cd tauq
cargo build --release
# Binary is at target/release/tauq
```

## Language Libraries

### Python
```bash
pip install tauq
```

### JavaScript / Node.js
```bash
npm install tauq
```

### Go
```bash
go get github.com/epistates/tauq
```

### Rust
Add this to your `Cargo.toml`:
```toml
[dependencies]
tauq = "0.1.0"
```

### Java / Kotlin
Add the dependency to your `pom.xml` or `build.gradle` (Artifact details TBD).

### C# / .NET
NuGet package coming soon.

### Swift
Add the package via Swift Package Manager:
```swift
.package(url: "https://github.com/epistates/tauq.git", from: "0.1.0")
```
