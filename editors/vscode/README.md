# Tauq for Visual Studio Code

Official VS Code extension for **Tauq** (τq) - the token-efficient data notation built for the AI era.

## Features

- **Syntax Highlighting**: Full syntax highlighting for `.tqn` and `.tqq` files
- **Diagnostics**: Real-time error detection and reporting
- **Auto-completion**: Intelligent completion for directives (`!def`, `!use`, `!import`, etc.)
- **Hover Information**: Quick documentation on hover
- **Formatting**: Document formatting support

## Requirements

This extension requires the `tauq-lsp` language server to be installed and available in your PATH.

### Installing the Language Server

```bash
# From crates.io (when published)
cargo install tauq

# From source
git clone https://github.com/epistates/tauq
cd tauq
cargo install --path .
```

The installation includes both the `tauq` CLI and `tauq-lsp` language server.

## Extension Settings

This extension contributes the following settings:

- `tauq.lsp.path`: Path to the Tauq Language Server executable (default: `tauq-lsp`)
- `tauq.lsp.enabled`: Enable/disable the language server (default: `true`)
- `tauq.format.indentSize`: Number of spaces for indentation (default: `2`)

## Quick Start

1. Install the extension
2. Install `tauq-lsp` (see above)
3. Open a `.tqn` or `.tqq` file
4. Start writing Tauq!

## Example

```tqn
# Define a schema
!def User id name email

# Data rows (44-54% fewer tokens than JSON!)
1 Alice alice@example.com
2 Bob bob@example.com
3 Carol carol@example.com
```

## Commands

- **Tauq: Convert to JSON** - Convert current Tauq file to JSON
- **Tauq: Convert from JSON** - Convert JSON to Tauq format
- **Tauq: Minify** - Compress Tauq to single line

## Learn More

- [Tauq Documentation](https://github.com/epistates/tauq)
- [Language Specification](https://github.com/epistates/tauq/blob/main/docs/src/spec/tauq_spec.md)
- [TauqQ Query Language](https://github.com/epistates/tauq/blob/main/docs/src/spec/tauqq_spec.md)

## License

MIT
