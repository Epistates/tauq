# Tauq Notation (TQN) Specification

Tauq Notation (`.tqn`) is a token-efficient data serialization format designed for the AI era. It prioritizes compactness, readability, and streamability.

## Basic Syntax

### Values
*   **Strings**: Double-quoted `"hello world"`. Barewords (unquoted strings) are allowed if they don't contain whitespace or special characters.
*   **Numbers**: Integers `42`, Floats `3.14`.
*   **Booleans**: `true`, `false`.
*   **Null**: `null`.

```tqn
name "Alice"
role admin
count 42
active true
```

### Arrays
Arrays are delimited by square brackets `[...]`. Elements are whitespace-separated. commas are not used.

```tqn
ids [1 2 3]
tags [web api "machine learning"]
```

### Objects / Structs
Tauq favors a "field-value" pair sequence for top-level data, or schema-defined rows.

## Tabular Data & Schemas
The core feature of Tauq is schema-driven data, which drastically reduces token count by removing repeated keys.

### `!def`
Defines a schema for a type.

```tqn
!def User id name email
```

### `!use`
Sets the active schema. Subsequent lines are interpreted as rows of values matching the schema fields.

```tqn
!use User
1 Alice "alice@example.com"
2 Bob "bob@example.com"
```

### Implicit Schema Usage
The `!def` directive both defines AND activates a schema. Data rows immediately following `!def` are parsed using that schema. Use `!use` explicitly only when switching to a different previously-defined schema.

### Schema Block with `---` Separator

When you need to define schemas upfront but use them in structured data (like arrays inside objects), use the `---` separator to terminate the implicit schema scope:

```tqn
!def User id name role
---
users [
  !use User
  1 Alice admin
  2 Bob user
]
config {
  timeout 30
  debug true
}
```

**How it works:**
1. `!def User ...` defines the schema
2. `---` clears the implicit schema activation (without it, `users` would be parsed as a schema field value)
3. After `---`, normal key-value parsing resumes
4. `!use User` inside the array activates the schema for array elements

### `!use` Inside Arrays

Arrays can contain schema-driven rows by using `!use` inside the array:

```tqn
!def Product sku name price
---
inventory [
  !use Product
  SKU001 "Widget" 9.99
  SKU002 "Gadget" 19.99
]
```

### Type Switching in Arrays

Multiple `!use` directives in the same array allow heterogeneous but structured data:

```tqn
!def Admin id name perms
!def Guest id name
---
team [
  !use Admin
  1 Alice [read write admin]
  2 Bob [read write]
  !use Guest
  3 Carol
  4 Dave
]
```

This produces an array with Admin objects followed by Guest objects, each with their respective schema fields.

## Nested Types
Types can be nested using `fieldname:TypeName` syntax in `!def`.

```tqn
!def Address street city
!def User id name addr:Address

1 Alice { "123 Main" "NYC" }
```

Nested objects are delimited by curly braces `{ ... }` and contain values corresponding to the nested type's schema.

## Lists of Objects
Lists can contain objects defined by a schema.

```tqn
!def Employee name role
!def Department name employees:[Employee]

Engineering [
    Alice "Dev"
    Bob "Ops"
]
```

## Deeply Nested Schemas

Schemas work at any nesting depth. The `!def/---/!use` pattern applies to arrays anywhere in the document structure:

```tqn
!def Item name price quantity sku
---
order {
  customer {
    name "Jane Doe"
    address {
      city "San Francisco"
      state CA
    }
  }
  items [
    !use Item
    "ThinkPad X1" 1299.99 1 "LAPTOP-001"
    "Wireless Mouse" 29.99 2 "MOUSE-042"
  ]
}
```

Multiple schemas can be defined and used at different locations:

```tqn
!def Department id name budget
!def Employee id name role
---
company {
  departments [
    !use Department
    1 Engineering 1000000
    2 Sales 500000
  ]
  employees [
    !use Employee
    101 Alice dev
    102 Bob mgr
  ]
}
```

## JSON to Tauq Conversion

The `tauq format` command intelligently converts JSON to Tauq:

```bash
tauq format data.json -o data.tqn
```

The formatter:
- Detects uniform arrays of objects and creates schemas automatically
- Uses context-aware naming (e.g., `users` array → `User` schema)
- Applies schemas at any nesting depth
- Deduplicates schemas with identical field signatures

## Minified Syntax
Semicolons `;` can be used as record separators to put multiple records on one line.

```tqn
!def U id name; 1 A; 2 B; 3 C
```