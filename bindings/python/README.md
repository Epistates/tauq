# Tauq for Python

**44% fewer tokens than JSON overall. 54% fewer for flat data. Verified with tiktoken.**

Tauq (τq) is a token-efficient data notation built for the AI era. This package provides high-performance Python bindings powered by Rust.

## Installation

```bash
pip install tauq
```

## Usage

```python
import tauq

# Parse Tauq to Python dict/list
data = tauq.loads("""
!def User id name email
1 Alice alice@example.com
2 Bob bob@example.com
""")
print(data)
# [{"id": 1, "name": "Alice", "email": "alice@example.com"}, ...]

# From file
data = tauq.load("config.tqn")

# Convert Python object to Tauq string
tauq_str = tauq.dumps({"app": "test", "port": 8080})
# 'app test port 8080'

# Execute TauqQ (query language)
result = tauq.exec_tauqq("!emit echo Hello", safe_mode=False)

# Minify Tauq
minified = tauq.minify("""
!def User id name
1 Alice
2 Bob
""")
```

## Streaming Support (AI Era Integration)

Tauq provides a `TauqStream` for processing data chunk-by-chunk. This is essential for LLM applications that need to process or display data as tokens arrive from the server.

```python
stream = tauq.TauqStream()

# Simulate arriving chunks (e.g., from an LLM response)
chunk1 = '!def U name; "Al'
chunk2 = 'ice"; "Bo'
chunk3 = 'b"'

print(stream.push(chunk1)) # []
print(stream.push(chunk2)) # [{"name": "Alice"}]
print(stream.push(chunk3)) # [{"name": "Bob"}]
print(stream.finish())     # []
```

## Binary Format (TBF)

For maximum size reduction, use the binary format:

```python
# Convert Tauq or JSON string to TBF bytes
bytes_data = tauq.to_tbf('!def U name; Alice; Bob')

# From Python object directly to TBF bytes
bytes_data = tauq.tbf_dumps([{"name": "Alice"}, {"name": "Bob"}])

# From TBF bytes to Python object
data = tauq.tbf_loads(bytes_data)

# File I/O for TBF
tauq.tbf_dump(data, "output.tbf")
loaded_data = tauq.tbf_load("output.tbf")
```

## API

### `loads(source: str) -> Any`
Parse Tauq notation string to a Python value.

### `load(path: Union[str, os.PathLike]) -> Any`
Load and parse a `.tqn` file. Supports `pathlib.Path`.

### `dumps(obj: Any) -> str`
Convert a Python object to Tauq notation.

### `dump(obj: Any, path: Union[str, os.PathLike]) -> None`
Serialize Python object to a Tauq file.

### `minify(source: str) -> str`
Compress Tauq to single-line format.

### `exec_tauqq(source: str, safe_mode: bool = False) -> Any`
Execute TauqQ (Tauq Query) and return the result.

### `tbf_dumps(obj: Any) -> bytes`
Serialize Python object to Tauq Binary Format (TBF) bytes.

### `tbf_loads(data: bytes) -> Any`
Deserialize TBF bytes to a Python object.

### `tbf_dump(obj: Any, path: Union[str, os.PathLike]) -> None`
Serialize Python object to a TBF file.

### `tbf_load(path: Union[str, os.PathLike]) -> Any`
Deserialize TBF file to a Python object.

### `class TauqStream`
Class for incremental stream parsing.
- `.push(chunk: str) -> List[Any]` - Returns completions from this chunk.
- `.finish() -> List[Any]` - Flushes remaining objects.

## License

MIT
