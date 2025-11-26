# Security Audit Report - Tauq
**Date:** November 24, 2025
**Auditor:** Google Security Engineering Team (Simulated)
**Target:** Tauq Core (`src/tauq/tauqq.rs`, bindings, and parser)

## Executive Summary
A security audit was performed on the `tauq` codebase, focusing on the query engine (`tauqq`) and FFI bindings. 

**Result:** 🔴 **FAIL**
**Primary Reason:** The "Safe Mode" feature, designed to sandbox execution for untrusted input, fails to prevent arbitrary file access. A user can read sensitive files (e.g., `/etc/passwd`, SSH keys) using `!read` even when `safe_mode` is enabled.

---

## 1. Critical Vulnerabilities

### [CRITICAL-01] Safe Mode Bypass: Arbitrary File Read
**Location:** `src/tauq/tauqq.rs`
**Description:** 
The `!read`, `!import`, and `!json` directives accept a file path and read its content using `std::fs::read_to_string`. While `!emit`, `!pipe`, and `!run` are correctly disabled when `safe_mode` is true, the file access directives are **not**.
**Impact:** 
An attacker submitting a malicious `.tqq` file to a service running `tauq` (e.g., a preview tool or data pipeline) can exfiltrate sensitive files from the server.
**Reproduction:**
```rust
// This succeeds even with safe_mode = true
let input = "!read \"/etc/passwd\"";
let result = tauqq::process(input, &mut vars, true);
```
**Remediation:** 
Disable `!read`, `!import`, and `!json` in `safe_mode`, or implement a strict allow-list/chroot mechanism for file paths.

---

## 2. High Severity Issues

### [HIGH-01] Denial of Service via Recursive Imports
**Location:** `src/tauq/tauqq.rs` -> `process` (recursive call)
**Description:** 
The `!import` directive calls `process` recursively without checking for import cycles or recursion depth.
**Impact:** 
A malicious file `bomb.tqn` containing `!import "bomb.tqn"` will cause a stack overflow, crashing the entire process (DoS).
**Remediation:** 
Implement a recursion depth limit (e.g., max 100 nested imports) and/or a cycle detection mechanism using a set of visited paths.

---

## 3. Medium Severity Issues

### [MED-01] Unsafe FFI Memory Management
**Location:** `src/c_bindings.rs` -> `tauq_free_string`
**Description:** 
The C API exposes `tauq_free_string`, which takes a `*mut c_char` and reconstructs a `CString` to drop it.
```rust
unsafe { CString::from_raw(s) };
```
If a caller passes a pointer that was not allocated by Rust (or was already freed), this causes undefined behavior (double free, heap corruption).
**Remediation:** 
This is inherent to C APIs, but documentation must strictly warn users. Consider using an opaque handle or a more robust memory model if possible.

---

## 4. Architectural Observations

### Shell Execution Model
The project uses `std::process::Command` with arguments split by a custom `split_args` function. It does **not** use `sh -c` by default.
*   **Pros:** Reduces the risk of accidental shell injection (e.g., chaining commands with `;` or `&&` inside an argument).
*   **Cons:** Users expecting shell features (globbing `*`, pipes `|`) inside `!emit` might be confused, as `!pipe` is the only directive that supports piping.
*   **Risk:** `!run` writes code to a temporary file and executes an interpreter. This is "Remote Code Execution as a Feature". It is critical that `safe_mode` is never accidentally disabled for untrusted input.

---

## Recommendations

1.  **Immediate Fix:** Patch `src/tauq/tauqq.rs` to check `safe_mode` before executing `!read`, `!import`, and `!json`.
2.  **Hardening:** Add a recursion depth counter to the `process` function signature.
3.  **Sanitization:** If file access is required in safe mode, implement `Path::canonicalize` and ensure the path starts with a permitted root directory.
