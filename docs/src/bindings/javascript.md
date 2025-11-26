# JavaScript / TypeScript Bindings

Our JS bindings are powered by WebAssembly (WASM), making them incredibly fast and suitable for both Node.js and the Browser.

## Installation

```bash
npm install tauq
```

## Usage

```javascript
import * as tauq from 'tauq';

// 1. Parse
const data = tauq.parse(`
!def User id name
1 Alice
`);
console.log(data); // [{id: 1, name: "Alice"}]

// 2. Stringify (Format)
const tqn = tauq.stringify(data);
console.log(tqn);

// 3. Exec Query
// Note: In the browser, shell execution (!run) is naturally sandboxed or disabled.
const result = tauq.exec("!set A 1\nfoo $A", true); // safe_mode=true

// 4. Minify
const min = tauq.minify("!def T x; 1; 2");
```