# Txtcode — Active Development Plan

**Version:** 3.0.0
**Updated:** 2026-03-27
**Tests:** 800 (`cargo test`) · 183 unit + 617 integration + 3 registry

Completed work history → `docs/dev-history.md`

---

## System State

| Component | Status |
|-----------|--------|
| Lexer | Working |
| Parser | Working (error recovery via `parse_with_errors`; position backtrack fixed) |
| Validator | Working — dup-fn, break/return/continue checks, undefined-var advisory |
| TypeChecker | Advisory by default; halts on `--strict-types` |
| IR layer | Active — `IrBuilder::apply_to_ast()` constant-folds AST before every execution |
| AST VM | Working — full feature set, production engine |
| Bytecode VM | **Production** — 800 tests pass (no feature flag required) |
| WASM output | Always compiled; control flow fixed (structured if/loop; no `;;` comment jumps) |
| LSP | Working (`txtcode lsp` JSON-RPC; non-zero spans for squiggles) |
| Security pipeline | Working (6-layer: permissions, capability, intent, rate-limit, audit, sandbox) |
| Registry server | Working — `load_index()` now parses saved index; search/publish functional |

---

## Architecture (Production Path)

```
source
  → Lexer → Parser
  → TypeChecker (advisory)
  → Validator (hard: dup-fn, break/return outside scope)
  → IrBuilder::apply_to_ast() (constant folding + dead-branch elimination)
  → VirtualMachine (AST-walking)
  → 6-layer security pipeline (per statement)
```

Bytecode path (`txtcode compile` / `txtcode debug`):
```
source → Lexer → Parser → BytecodeCompiler → BytecodeVM
```

---

## Milestones

| # | Version | Theme | Status |
|---|---------|-------|--------|
| M1 | 1.1.0 | Stdlib Essentials | ✅ Complete |
| M2 | 1.2.0 | Real Async | ✅ Complete |
| M3 | 1.3.0 | Data & Storage | ✅ Complete |
| M4 | 1.4.0 | Engine Parity | ✅ Complete |
| M5 | 1.5.0 | Developer XP | ✅ Complete |
| M6 | 2.0.0 | Platform | ✅ Complete |

**No open tasks — all milestones M1–M6 complete as of 2026-03-27.**

---

## Deferred (Post-M6 infrastructure)

| Item | File(s) | Why deferred |
|------|---------|--------------|
| Registry deployment | `src/bin/registry_server.rs` | Backend logic complete — needs CDN + hosting |
| Self-update | `src/security/update_verifier.rs` | Requires CDN + binary releases |
| Docker images | `.github/workflows/docker.yml` | Infrastructure; no dev need yet |
| Registry publishing | `txtcode package publish` | Contacts non-existent server |
| LLVM/Cranelift backend | — | Post-M6 |

---

## Known Technical Debt

None open. All resolved — see `docs/dev-history.md` (sessions 2–5).

---

## Repository Directory Map

| Directory / File | What it is | Build connection |
|-----------------|-----------|-----------------|
| `src/` | Language runtime source | `cargo build` always |
| `tests/` | Integration + unit test suite | `cargo test` always |
| `benches/` | Criterion benchmarks | `[[bench]]` in Cargo.toml |
| `fuzz/` | Cargo-fuzz harness (5 targets) | `cargo fuzz`, own workspace root |
| `crates/plugin-sdk/` | Plugin ABI SDK for native plugin authors | Workspace member (CL.1 ✅) |
| `editors/` | VS Code extension (Node.js, standalone) | Not in Rust build — see `editors/README.md` |
| `assets/` | Icons + logos shared by docs site and VS Code extension | Not in Rust build |
| `mkdocs.yml` | MkDocs docs-site config | Deployment only (deferred) |
| `Makefile` | Build/install shortcuts wrapping cargo | Developer convenience |
| `install.sh` / `uninstall.sh` | End-user install scripts | Deferred (no binary releases yet) |
| `memory/` | Claude Code auto-memory — gitignored (CL.4 ✅) | Not a project file |

Groups CL + AU complete — see `docs/dev-history.md` (sessions 3–4).
