# Tauq for JavaScript/TypeScript

**44% fewer tokens than JSON overall. 54% fewer for flat data. Verified with tiktoken.**

Tauq (τq) is a token-efficient data notation built for the AI era. This package provides WebAssembly bindings for Node.js and browsers.

## Installation

```bash
npm install tauq
```

## Usage

```javascript
const tauq = require('tauq');

// Parse Tauq to JavaScript object
const data = tauq.parse(`
!def User id name email
1 Alice alice@example.com
2 Bob bob@example.com
`);
console.log(data);
// [{ id: 1, name: "Alice", email: "alice@example.com" }, ...]

// Convert Tauq to JSON string
const json = tauq.to_json(`
name Alice
age 30
`);
// '{"name":"Alice","age":30}'

// Execute TauqQ (query language)
const result = tauq.exec(`
!set greeting "Hello"
!emit echo $greeting
`, false); // false = not safe mode

// Minify Tauq
const minified = tauq.minify(`
!def User id name
1 Alice
2 Bob
`);
// "!def U id name; 1 Alice; 2 Bob"
```

## Streaming Support (AI Era Integration)

Tauq provides a `TauqStream` for processing data chunk-by-chunk. This is essential for LLM applications that need to process or display data as tokens arrive from the server.

```javascript
const stream = new tauq.TauqStream();

// Simulate arriving chunks (e.g., from an LLM response)
const chunk1 = '!def U name; "Al';
const chunk2 = 'ice"; "Bo';
const chunk3 = 'b"';

console.log(stream.push(chunk1)); // []
console.log(stream.push(chunk2)); // [{ name: "Alice" }]
console.log(stream.push(chunk3)); // [{ name: "Bob" }]
console.log(stream.finish());     // []
```

## Binary Format (TBF)

For maximum size reduction, use the binary format:

```javascript
// Convert Tauq to TBF bytes (Uint8Array)
const bytes = tauq.to_tbf(`!def U name; Alice; Bob`);

// Convert TBF back to Tauq string
const tauqStr = tauq.tbf_to_tauq(bytes);
```

## API

### `parse(input: string): any`
Parse Tauq notation to a JavaScript value.

### `exec(input: string, safeMode: boolean): any`
Execute TauqQ (Tauq Query) and return the result.
- `safeMode: true` disables shell commands (`!emit`, `!pipe`, `!run`)

### `minify(input: string): string`
Compress Tauq to single-line format.

### `stringify(value: any): string`
Convert a JavaScript value to Tauq notation.

### `to_json(input: string): string`
Parse Tauq and return as JSON string.

### `to_tbf(input: string): Uint8Array`
Encode Tauq or JSON string to Tauq Binary Format.

### `tbf_to_tauq(data: Uint8Array): string`
Decode TBF bytes to Tauq notation.

### `new TauqStream()`
Class for incremental stream parsing.
- `.push(chunk: string): any[]` - Returns array of completed objects from this chunk.
- `.finish(): any[]` - Flushes remaining objects.

## TypeScript

Type definitions are included:

```typescript
import * as tauq from 'tauq';

const stream = new tauq.TauqStream();
const objects = stream.push('key val');
```

## Browser Usage

For browser usage, build with web target:

```bash
npm run build:web
```

Then import in your bundler:

```javascript
import init, { parse, stringify } from 'tauq';

await init();
const data = parse('key value');
```

## License

MIT
