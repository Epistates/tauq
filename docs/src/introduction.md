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

Tauq reduces token usage by **44-54%** (verified with tiktoken) for structured data by eliminating repetitive keys and syntax noise.

```tqn
!def User id name email role active
1 Alice alice@example.com admin true
2 Bob bob@example.com user true
```
**~110 tokens** per 100 records.

## Benchmark Results

| Format | 1000 Records | Tokens | vs JSON |
|--------|--------------|--------|---------|
| JSON (minified) | 87 KB | 24,005 | baseline |
| TOON | 45 KB | 12,002 | -50.0% |
| **Tauq** | **43 KB** | **11,012** | **-54.1%** |

*All counts verified with tiktoken cl100k_base (GPT-4/Claude tokenizer).*

**Overall (10 datasets, 55,647 tokens):** Tauq saves 44.2% vs JSON, 10.8% vs TOON.

## Key Features

- **Token-Optimal:** 44-54% fewer tokens than JSON, 11% more efficient than TOON.
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
