# Introduction

**Tauq** (τq) is a token-efficient data notation built for the AI era where every token counts.

## The Problem

JSON is verbose. In the age of Large Language Models (LLMs), verbosity equals cost and latency.

```json
[
  {"id": 1, "name": "Alice", "email": "alice@example.com", "role": "admin", "active": true},
  {"id": 2, "name": "Bob", "email": "bob@example.com", "role": "user", "active": true}
]
```
**~242 tokens** (minified) per 100 records.

## The Solution

Tauq reduces token usage by **58%** (verified with tiktoken) for structured data by eliminating repetitive keys and syntax noise.

```tqn
!def User id name email role active
1 Alice alice@example.com admin true
2 Bob bob@example.com user true
```
**~108 tokens** per 100 records.

## Benchmark Results

| Format | 1000 Records | Tokens | vs JSON |
|--------|--------------|--------|---------|
| JSON (minified) | 87 KB | 24,005 | baseline |
| TOON | 45 KB | 13,765 | -43% |
| **Tauq** | **43 KB** | **10,011** | **-58%** |

*All counts verified with tiktoken cl100k_base (GPT-4/Claude tokenizer).*

## Key Features

- **Token-Optimal:** 58% fewer tokens than JSON, 27% fewer than TOON.
- **Schema-Driven:** Define shapes with `!def` and switch with `!use`.
- **True Streaming:** `StreamingParser` yields records one at a time. No count required.
- **Programmable:** Use **Tauq Query (TQQ)** for data transformations.
- **Polyglot:** Bindings for Python, JavaScript, Go, Java, C#, Swift, and Rust.
- **Secure:** Path traversal protection and safe mode for TauqQ.

## Why Tauq Beats TOON

| Feature | TOON | Tauq |
|---------|------|------|
| Count required | Yes `[N]` | **No** |
| Delimiter | Comma (1 token) | **Space (0 tokens)** |
| Streaming | Block parse | **Iterator API** |
| Query language | No | **Yes (TQQ)** |
| Comments | No | **Yes** |
