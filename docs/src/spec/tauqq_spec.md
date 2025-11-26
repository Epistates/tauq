# Tauq Query (TQQ) Specification

Tauq Query (`.tqq`) is a programmable **pre-processor** for Tauq Notation.

## Execution Model

1.  **Pre-processing**: The TQQ engine reads the source file, executing directives and resolving imports.
2.  **Generation**: The output of the pre-processing phase is a pure stream of Tauq Notation (`.tqn`).
3.  **Parsing**: The generated TQN is parsed by the Tauq parser to produce the final data structure (e.g., JSON).

## Directives

All directives start with `!`. They are processed strictly in order.

### `!pipe <command>`
**Applies to: All subsequent lines.**

Pipes the remainder of the generated document (from the point of the directive to the end) into the standard input of `<command>`. The command's standard output replaces the original content.

*   **Behavior**: It acts as a "filter" for the rest of the stream.
*   **Termination**: Processing of the current scope stops after a `!pipe` because the rest of the content is consumed by the pipe.

```tqn
!pipe sort
C
A
B
```
*Result:* `A\nB\nC`

### `!emit <command>`
Executes `<command>` and inserts its standard output into the stream at the current position.

### `!run <interpreter> { ... }`
Executes the enclosed code block using the specified interpreter. The block's stdout is inserted into the stream.

### `!set <key> <value>`
Sets a variable in the processing context. These are passed as environment variables to child processes started by `!emit`, `!pipe`, or `!run`.

### `!import <file>`
Recursively processes and inserts the content of `<file>`.

### `!json <file>`
Reads a JSON file, parses it, converts it to Tauq Notation, and inserts it into the stream.

### `!read <file>`
Reads a file and emits its content as a quoted JSON string. Useful for embedding raw text.

```
