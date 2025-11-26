# Tauq Basics

Tauq Notation (`.tqn`) is designed to be the most token-efficient way to represent structured data. If you know JSON, you're 90% of the way there—just subtract the syntax tax.

## The Core Philosophy: "Fields, not Keys"

JSON repeats keys for every object. Tauq defines the "shape" (schema) once and then streams the values.

### 1. Simple Key-Value Pairs
For simple configuration, it looks a lot like other formats, but cleaner.

```tqn
app_name "My Service"
version "1.0.0"
debug true
timeout 30
```

### 2. The Power of Schemas (`!def`)
This is where Tauq shines. Use `!def` to define a schema—it automatically becomes active.

```tqn
!def User id name email role

1 "Alice" "alice@example.com" "admin"
2 "Bob"   "bob@example.com"   "user"
```

**What just happened?**
1.  `!def User ...` created a reusable template and activated it immediately.
2.  The parser reads the values and automatically maps them to `id`, `name`, `email`, and `role`.
3.  Use `!use` to switch between previously defined schemas (see Schema Block below).

### 3. Arrays and Lists
Arrays use `[...]`. No commas needed, just spaces.

```tqn
tags [web api "machine learning"]
matrix [
    [1 0 0]
    [0 1 0]
    [0 0 1]
]
```

### 4. Nested Objects
You can define nested schemas for complex data.

```tqn
!def Geo lat lon
!def City name location:Geo

!use City
"New York" { 40.71 -74.00 }
"London"   { 51.50 -0.12 }
```

### 5. Schema Block Pattern

When converting JSON with nested arrays, use the `---` separator to define schemas upfront:

```tqn
!def User id name role
---
users [
  !use User
  1 Alice admin
  2 Bob user
]
settings {
  timeout 30
}
```

**Why `---`?** After `!def`, the schema is implicitly active for immediate data rows. The `---` separator clears this, letting you write structured key-value data that references schemas inside arrays with `!use`.

### 6. Minified Syntax
Want to squeeze it even smaller? Use semicolons `;` to stack records on one line.

```tqn
!def Point x y; 0 0; 10 20; 100 200
```

## Next Steps
Now that you know the notation, learn how to generate and transform it dynamically with [Tauq Query](tauq_query.md).