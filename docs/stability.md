# Stability Guarantees and Semver Policy

Txtcode v3.0.0 establishes clear stability tiers and a semantic versioning policy for the language, standard library, CLI, and internal APIs.

---

## Stability Tiers

### Stable (guaranteed for v3.x)

The following surfaces are **stable** from v3.0.0 and will not have breaking changes until a v4.0 release:

| Surface | Notes |
|---------|-------|
| **Language syntax** | All syntax described in `language-spec.md`. Additive changes only in minor releases. |
| **Standard library API** | All function names, argument order, and return types documented in stdlib. New functions may be added in minor releases. |
| **CLI flags** | All flags in `txtcode --help`. Flags will not be removed or renamed in patch/minor releases. |
| **Module format** | `Txtcode.toml`, `import` statements, module search paths. |
| **Lockfile format** | `Txtcode.lock` — lockfiles are forward-compatible within v3.x. |
| **Package registry protocol** | `POST /api/v1/packages` and `GET` endpoints. |
| **Script signature format** | `.sig` files produced by `txtcode sign` are stable. |
| **Audit log JSON format** | Fields in `--audit-log` output. |

### Unstable (may change in v3.x minor releases with deprecation notice)

| Surface | Notes |
|---------|-------|
| **Bytecode format** | The compiled `.tbc` bytecode is internal. Do not distribute compiled bytecode; always distribute source `.tc` files. |
| **Internal Rust API** | `src/runtime/`, `src/parser/`, `src/lexer/` Rust types and traits are not public API. |
| **LSP protocol extensions** | Custom LSP capabilities beyond the standard protocol. |
| **VM internals** | `VirtualMachine` struct fields, `Value` enum variants (for Rust embedding). |

### Experimental (may be removed or significantly changed)

| Surface | Notes |
|---------|-------|
| **WASM target** | `txtcode compile --target wasm` output format may change. |
| **Native compilation** | Not yet production-ready. |
| **Registry server** | The self-hosted registry server binary is experimental. |

---

## Semantic Versioning Policy

Txtcode follows [Semantic Versioning 2.0.0](https://semver.org/).

### Patch releases (3.0.x)

- Bug fixes
- Security patches
- Performance improvements
- Documentation corrections
- **No breaking changes to any Stable surface**

### Minor releases (3.x.0)

- New stdlib functions (backwards compatible)
- New language features (backwards compatible)
- New CLI flags
- New LSP capabilities
- Deprecation of Unstable surfaces (with one minor release notice)
- **No breaking changes to any Stable surface**

### Major releases (4.0)

- Breaking changes to language syntax or stdlib API are permitted
- Must be accompanied by a migration guide
- `txtcode migrate` will be updated to handle all breaking changes
- Breaking changes will be announced at least one minor release in advance

---

## Deprecation Process

1. A feature is marked `[DEPRECATED]` in docs and emits a runtime warning
2. Deprecated features remain functional for **at least one minor release** after deprecation
3. Removed in the next major release

---

## Long-Term Support

- **v3.0.x** receives security patches for **24 months** from release date (until March 2028)
- **v3.x** receives bug fixes for **12 months** after the next minor release

---

## Reporting Stability Issues

If a patch or minor release breaks your code in a way that violates this policy, please [file an issue](https://github.com/iga2x/txtcode/issues) with the label `stability-regression`.
