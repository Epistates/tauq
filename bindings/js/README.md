# Tauq for JavaScript/TypeScript

**58% fewer tokens than JSON. Verified with tiktoken.**

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

// Convert JS object to Tauq string
const obj = { users: [{ id: 1, name: "Alice" }] };
const formatted = tauq.stringify(obj);
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

## TypeScript

Type definitions are included:

```typescript
import * as tauq from 'tauq';

interface User {
  id: number;
  name: string;
}

const users = tauq.parse(`
!def User id name
1 Alice
2 Bob
`) as User[];
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

## Why Tauq?

| Format | 1000 Records | Tokens |
|--------|--------------|--------|
| JSON | 87 KB | 24,005 |
| **Tauq** | **43 KB** | **10,011** |

58% fewer tokens = 58% lower API costs for LLM applications.

## License

MIT
