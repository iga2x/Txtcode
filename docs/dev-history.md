# Txtcode — Completed Development History

**Last updated:** 2026-03-27 (session 6)
**Tests at completion of each phase recorded below.**

---

## v3.0.0 — Clippy Cleanup (2026-03-27 session 6)
**800 tests · 0 failures · 6 intentional warnings remain**

Reduced `cargo clippy` output from 116 warnings → 6 (all intentionally skipped).

| Step | What was done |
|------|--------------|
| Auto-fix | `cargo clippy --fix --allow-dirty` — eliminated ~74 mechanical warnings (`needless_borrow`, `redundant_closure`, `needless_return`, `collapsible_match`, `option_map_or_none`, `useless_conversion`) |
| Bug fix | `await_any` in `stdlib/mod.rs`: loop-that-never-loops replaced with `futures.into_iter().next()` |
| Doc comments | Fixed `empty_line_after_doc_comments` in 8 files (`wasm.rs`, `embed/mod.rs`, `permission_map.rs`, `event_loop.rs`, `bytecode_vm.rs`, `errors.rs`, `wasm_exec.rs`, `sandbox.rs`) |
| manual_strip | Replaced `s["prefix".len()..]` with `s.strip_prefix("prefix").unwrap_or("")` in `lsp.rs`, `migration.rs` |
| Dead code | Added `#[allow(dead_code)]` to intentional stubs (`wasm.rs` fields, `binary_op` method in `bytecode_vm.rs`) |
| clone_on_copy | `elem.clone()` → `std::slice::from_ref(&elem)` in `bytecode_vm.rs` (filter/find lambdas) |
| BPF filters | Added `#[allow(clippy::vec_init_then_push)]` to seccomp BPF filter builders in `sandbox.rs` |
| Loop index | `for i in 0..=m { dp[i][0] = i }` → `dp.iter_mut().enumerate()` in `expressions/mod.rs` |
| Collapsible patterns | Merged nested `if let` / `match` in `obfuscator.rs`, `semantics.rs`, `template.rs`, `checker.rs` (×3), `function_calls.rs` |
| Identical blocks | Merged duplicate `SysLib::call_function` branches and `gzip_`/`bytes_` branches in `stdlib/mod.rs` |
| registry_server | `current_name` outer `mut` var → local `let` inside match arm |
| compile.rs | `file.clone()` → `file.to_path_buf()` after `&PathBuf` → `&Path` signature change |
| ffi.rs | Restored `use std::sync::Arc` inside `#[cfg(test)]` module (was removed with outer import) |
| **Skipped** | `too_many_arguments` (3 fns), `type_complexity` (2 types), `from_str` naming — all intentional |

---

## v3.0.0 — M6 Platform (2026-03-27 session 5)
**800 tests · 0 failures — bytecode VM now production (no feature flag)**

| Task | What was done |
|------|--------------|
| M6.1 | Graduated Bytecode VM: removed all 72 `#[cfg(feature = "bytecode")]` gates across 13 files; made `bincode` unconditional; `compile` and `debug` subcommands always available; 800 tests pass without `--features` |
| M6.2 | Restored `scripts/sign_release.sh` (was deleted) — Ed25519 binary signing for release CI; exits 0 gracefully when `SIGNING_KEY_HEX` not set |
| M6.3 | Restored `playground/index.html` + `playground/app.js` from git history (`561249e`); `.github/workflows/playground.yml` was already present and complete |

---

## v3.0.0 — Full Project Audit & Cleanup (2026-03-27 session 4)

| Task | What was done |
|------|--------------|
| AU.1 | Deleted `Txtcode.toml` (test scratch file, was gitignored but present in working tree) |
| AU.2 | Deleted `editors/txtcode.tmLanguage.json` — exact duplicate of `editors/syntaxes/txtcode.tmLanguage.json` |
| AU.3 | `README.md`: removed Docker badge (deferred feature), removed `NON-GOALS.md` link (file deleted), removed stale Examples section (referenced deleted `.tc` files), fixed Quick Start (`script.tc` instead of `examples/hello_world.tc`), updated Project Structure table |
| AU.4 | `mkdocs.yml`: removed `logo`/`favicon` (pointed to deleted `docs/assets/` files), fixed `nav` — removed 6 non-existent pages (`getting-started.md`, `cli.md`, `packages.md`, `roadmap.md`, `CHANGELOG.md`, `index.md`), added `syntax-reference.md` |

---

## v3.0.0 — Project Cleanup (2026-03-27 session 3)
**628 lib + 3 registry · 800 (--features bytecode) — workspace now covers plugin-sdk**

| Task | What was done |
|------|--------------|
| CL.1 | Added `[workspace] members = [".", "crates/plugin-sdk"]` to root `Cargo.toml` — plugin SDK now built and tested with main crate |
| CL.2 | Created `editors/README.md` — documents VS Code extension as standalone Node.js project, build steps, LSP wiring |
| CL.3 | Deleted duplicate `favicon.ico` from repo root — canonical copy is `assets/favicon.ico` (used by `mkdocs.yml`) |
| CL.4 | Added `memory/` to `.gitignore` — Claude Code auto-memory directory excluded from version control |

---

## v3.0.0 — Technical Debt Resolved (2026-03-27 session 2)
**628 lib + 3 registry tests · 800 (--features bytecode)**

### Milestones M1–M5 (all complete)

| # | Version | Theme |
|---|---------|-------|
| M1 | 1.1.0 | Stdlib Essentials |
| M2 | 1.2.0 | Real Async |
| M3 | 1.3.0 | Data & Storage |
| M4 | 1.4.0 | Engine Parity |
| M5 | 1.5.0 | Developer XP |

### M4 — Engine Parity

| Task | What was done |
|------|--------------|
| 4.1–4.4 | Fixed bytecode compile errors; backend decision (keep both VMs); feature-gate; parity tests |
| 4.5 | `Expression::Float(f)` in BytecodeCompiler emits `PushConstant(Value::Float(f))` |
| 4.6 | Annotated assignments fail with E0011 when type annotation violated at runtime |

### M5 — Developer XP

| Task | What was done |
|------|--------------|
| 5.1 | LSP: `extract_position` parses `"Parse error at line N, column M"`; `LspDiagnostic` gains `col` field; +3 tests |
| 5.2 | Debugger: `next`/`n` (step_over) added to `Debugger` + CLI; +1 test |
| 5.3 | Formatter: comments preserved, idempotent (23 tests), `--check` exits 1 for CI |
| 5.4 | Error quality: `execute()` wraps errors with stmt span; index-OOB gets E0013 + range hint |

### Group P — Parser Quality & Error UX

| Task | File | What was done |
|------|------|--------------|
| P.1 | `src/parser/parser.rs:337` | Speculative parse saves/restores position on failure — no more position leak |
| P.2 | `src/parser/parser.rs:340`, `primary.rs:289` | Static error messages; `self.error()` supplies current-token context |
| P.3 | `src/parser/statements/control.rs` | `LeftBrace` after condition detected; helpful diagnostic mentioning `end` |
| P.4 | `src/parser/utils.rs:25` | `end: token.span.1 + token.value.len()` — non-zero spans for LSP squiggles |

### Known Technical Debt — all resolved

| Issue | Fix |
|-------|-----|
| Float literals in BytecodeCompiler | `PushConstant(Value::Float(f))` — M4.5 |
| WASM `;;` comment jumps (broken control flow) | Forward jumps: `i = target.min(end)`; backward: `unreachable`; 3 new tests in `wasm.rs` |
| Speculative parse position leak | Saved/restored `self.position` — P.1 |
| Zero-width AST spans | `end = start + token.value.len()` — P.4 |
| `registry_server.rs` `load_index()` discards content | Line-based parser for `save_index()` format; 3 new tests |

---

## v3.0.0 — Pipeline Integrity Audit (2026-03-26)
**805 tests · 0 failures**

| Item | What was done |
|------|--------------|
| AUDIT-1 | `Builder::load_and_validate(path)` added; REPL `:load` uses it |
| AUDIT-2 | REPL `block_depth` underflow fixed (guard: `if block_depth < 0 { block_depth = 0 }`) |
| AUDIT-3 | Bytecode VM hot loop clone eliminated (`bytecode.instructions[ip].clone()` → borrow) |
| AUDIT-4 | Permission map coverage tests — 7 new tests covering all privileged function categories |
| AUDIT-5 | Linter tests added for L014, L015, L016, L018 |
| AUDIT-6 | Bytecode VM `ImportModule` verified fully wired (sub-VM with full security inheritance) |
| P1 | Validator added to `bench.rs` (parse-time validation before warmup/timed runs) |
| P2 | REPL normal input already had Validator (confirmed, no change needed) |
| P3 | `Builder::create_vm(config)` factory; bench.rs + embed use it instead of `VirtualMachine::new()` |
| P4 | REPL `:type` now uses TypeChecker (not TypeInference) |
| P5 | `run_type_check()` — critical type errors halt even in advisory mode |
| P6 | Global `NATIVE_REGISTRY` removed from embed; `VirtualMachine.call_native_fn` no longer falls back to global |

---

## Groups P–W + Phases 3–7 (2026-03-25/26)

| Group | What was done |
|-------|--------------|
| P — Embed security | Validator enforced in `eval_inner()`; `with_sandbox()` added |
| Q — Validator completeness | Undefined-var advisory; arity check; stdlib list auto-synced from `STDLIB_DISPATCH` |
| R — IR layer | `src/ir/` created; `IrBuilder::apply_to_ast()` live; constant folding + dead-branch elimination |
| S — Backend decision | Option C (keep both VMs, bytecode experimental); 788 bytecode tests pass |
| T — Test restructuring | `test_runtime.rs` split into 10 domain files; golden tests added |
| U — FFI panic fix | `panic!()` → `RuntimeError` in `ffi.rs` |
| V — Inference dead code | Dead `constraints` field removed |
| W — CLI pipeline cutover | All CLI commands use Builder; no direct VM imports in CLI |
| Phase 3 | Stdlib validator auto-generated from `STDLIB_DISPATCH` |
| Phase 4 | WASM TOCTOU bug fixed (`emit_artifact` uses existing Program) |
| Phase 5 | REPL uses `Builder::create_repl_vm()` |
| Phase 6 | Per-engine embed registry — no global collision |
| Phase 7 | 20 parity tests; `run_ast_vm()` handles top-level return |

---

## v5.0 Milestones 1 & 3 (2026-03-20/21)

| Milestone | What was done |
|-----------|--------------|
| M1 (v1.1.0) | Stdlib essentials: async_run/await_future/await_all/async_sleep; LSP publishDiagnostics wired |
| M3 (v1.3.0) | db_transaction auto-rollback; connection limit (50); stdlib audit; str_build |

---

## Groups A–N (v3.0 new plan, 2026-03-19/20)

| Group | What was done |
|-------|--------------|
| A — Bug Fixes | register_fn wired; warnings fixed; grammar.rs deleted; db_commit/rollback added |
| B — Dead Code | AIMetadata removed from all paths; migration start var fixed |
| C — Call Depth | stacker::maybe_grow + MAX_CALL_DEPTH=500 |
| D — Async | Multi-worker thread pool event_loop.rs; async_run_scoped; permission snapshot |
| E — Language Completeness | Protocols; generic structs; standard error types; parser error recovery; TCO |
| F — Tooling | Formatter (+10 tests); 10 new lint rules L010-L019; LSP context completion + signatureHelp |
| G — Security | Persistent audit log in safe mode; seccomp allowlist; macOS sandbox_init() |
| I — Embed API | eval_string(); last_error_code(); error codes in C ABI |
| J — Version/Hygiene | version=3.0.0; docs/deferred.md |
| K — Type System | Type::Unknown; check_strict; match exhaustiveness |
| L — Stdlib | http_serve handler; regex cache; plugin JSON ABI |
| M/N — Language Core | Pattern::Literal; protocol compliance; optional chaining typecheck; extended TCO; modulo E0012; rest pattern validation |
| O — VM parity | break/continue boundary fix; bytecode parity + constant-fold optimizer |
| Q/R/V/W — Type & Lang bugs | Elseif branches; compound/index assign typecheck; dotted methods; closure fixes |
| T.1 — LSP | publishDiagnostics wired; Layer 3 = 100% |

---

## Groups 23–29 (v2.1–2.7, 2026-03-20)

| Group | What was done |
|-------|--------------|
| 23 | CI binary releases; registry backend; SHA-256 verification |
| 24 | JSON error output (`--json` flag) |
| 25 | OS sandbox: prctl(PR_SET_NO_NEW_PRIVS) + seccomp-BPF blocklist |
| 26 | Async cancellation: async_cancel_token/async_cancel/is_cancelled |
| 27 | xml_stringify; WebSocket; gzip compress/decompress; CSV streaming; PostgreSQL/MySQL |
| 28 | Debugger; Embedding API (Rust + C ABI); Web REPL playground |
| 29 | WASM strings+arrays; WASM binary output (wasm-encoder); WASM execution (wasmtime) |

---

## Groups 1–22 (v0.1–v2.0)

Complete implementation history in git log. Key milestones:

- Groups 1–8: Core language, security model, const enforcement, CLI signing, FFI allowlisting, audit log
- Groups 9–14: Module isolation, lockfile enforcement, deterministic maps, type checking, destructuring, iterators, generators, TCO, pattern match guards
- Groups 15–22: Formatter, linter, LSP, REPL history, bytecode debugger, docgen, async/await, WASM

Full details: `git log --oneline`
