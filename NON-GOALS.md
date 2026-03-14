# Txt-code Non-Goals

This document lists things that Txt-code is **intentionally not** designed to do.
Understanding non-goals is as important as understanding goals — it defines the scope
and prevents scope creep.

---

## Not a General-Purpose Language

Txt-code is a **domain-specific language** for secure orchestration and automation.
It is not intended to replace Python, Rust, Go, or other general-purpose languages
for all use cases.

Use Txt-code for: policy-controlled automation, security tooling, AI-assisted execution.
Use other languages for: GUI applications, game development, high-performance computing.

---

## Not a Shell Replacement

Txt-code is not a shell scripting language like Bash, Zsh, or Fish.
It does not aim to replace shell scripts for general system administration.

---

## Not an Exploit Framework

Txt-code does not:
- Provide built-in exploit payloads or shellcode generators
- Include CVE databases or auto-exploitation modules
- Automate unauthorized access to systems

Its security features are for **defensive automation, auditing, and controlled testing**
within authorized environments only.

---

## Not an Obfuscation Tool for Malware

The built-in obfuscation and encryption features exist to:
- Protect legitimate intellectual property in compiled programs
- Prevent casual reverse engineering of authorized tools

> **Note:** AST identifier obfuscation is a planned no-op stub in v0.4 — `Obfuscator::obfuscate()` returns the program unchanged and provides no IP protection. See [docs/security-features.md](docs/security-features.md) for current status.

They are **not** intended to help evade antivirus detection or hide malicious code.

---

## Not a Compiled Systems Language

Txt-code is not designed to replace C, C++, or Rust for:
- Operating system kernels
- Device drivers
- Hard real-time systems requiring nanosecond-level latency guarantees

---

## Not a Sandboxed Browser Language

Txt-code does not target browser environments or WebAssembly runtimes as a primary
deployment target (WASM output is experimental and not a core goal).

---

## Not a Package Registry

Txt-code includes a minimal package management system (`Txtcode.toml`) for dependency
declaration. It does not operate or host a public package registry comparable to
crates.io, npm, or PyPI.

---

## No Stability Guarantees Before v1.0

Until version 1.0, Txt-code syntax and APIs may change between releases.
Backwards compatibility is a goal post-v1.0, not before.
