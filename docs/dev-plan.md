# TXTCode — Master Development Plan

**Version:** 3.0.0
**Updated:** 2026-03-26
**Tests:** 805 (`cargo test`: 628 integration + 177 unit) · 788+ (`cargo test --features bytecode`)
**Focus:** Language correctness and developer experience. No new features until M5 is complete.

---

## System State (Ground Truth)

| Component | Status |
|-----------|--------|
| Lexer | Working |
| Parser | Working (with error recovery via `parse_with_errors`) |
| Validator | Working — duplicate fn, return/break/continue checks + undefined-var advisory |
| TypeChecker | Advisory by default; halts on `--strict-types` |
| IR layer | **Active** — `IrBuilder::apply_to_ast()` constant-folds AST before every execution |
| AST VM | Working — full feature set, production engine |
| Builder | Single entry point for all CLI paths |
| REPL | Working — VM created via `Builder::create_repl_vm()` |
| Embed API | Working — per-engine native registry (no global fallback); Validator enforced |
| Stdlib list | Auto-generated from `STDLIB_DISPATCH` (217+ names); no validator drift |
| Bytecode VM | **Experimental** (`--features bytecode`) — 788 tests pass; float literals unimplemented |
| WASM output | Behind `bytecode` feature; IR path available; control flow partially broken |
| LSP | Working (`txtcode lsp` JSON-RPC) |
| Security pipeline | Working (6-layer: permissions, capability, intent, rate-limit, audit, sandbox) |
| Constant folding | **Active** via `IrBuilder::apply_to_ast()` — mutates AST in place before VM |
| Parity tests | 19 AST/Bytecode parity tests pass; 1 ignored (float — known bytecode gap) |

---

## Architecture

```
Production path (all CLI entry points):
  source
    → Lexer → Parser
    → TypeChecker (advisory)
    → Validator (hard: dup-fn, break/return outside scope, undefined-var advisory)
    → IrBuilder::apply_to_ast() (constant folding + dead-branch elimination)
    → VirtualMachine (AST-walking)
    → 6-layer security pipeline (per statement)

Experimental (--features bytecode):
  source → Lexer → Parser → BytecodeCompiler → BytecodeVM

Future target (M4+ complete):
  source → Lexer → Parser → Validator → IR → [ AST-VM | BytecodeVM | WasmBackend ]
  All entry points (CLI, REPL, embed) go through Builder — no bypasses.
```

---

## Milestones

| # | Version | Theme | Status |
|---|---------|-------|--------|
| M1 | 1.1.0 | Stdlib Essentials | ✅ Complete |
| M2 | 1.2.0 | Real Async | ✅ Complete |
| M3 | 1.3.0 | Data & Storage | ✅ Complete |
| M4 | 1.4.0 | Engine Parity | 🔧 Active |
| M5 | 1.5.0 | Developer XP | Pending M4 |
| M6 | 2.0.0 | Platform | Deferred — see §Deferred below |

> Every milestone answers: *"Can a real developer ship something with this?"*

---

## Milestone 4 — Engine Parity (v1.4.0) 🔧

**Goal:** Bytecode VM passes all tests AST VM passes. Predictable behavior on both engines.

| Task | Status |
|------|--------|
| 4.1 Fix bytecode VM compile errors (12× Arc\<str\>) | ✅ Done (2026-03-26) |
| 4.2 Backend decision: Option C — keep both VMs, bytecode is experimental | ✅ Decided (2026-03-26) |
| 4.3 Feature-gate `wasm_binary.rs` consistently | ✅ Done (2026-03-25) |
| 4.4 20 AST/Bytecode parity tests | ✅ Done (2026-03-26) |
| 4.5 Float literals in bytecode VM | ❌ Open — bytecode compiler emits Null for float constants |
| 4.6 Runtime type enforcement (`store → x: int → "hello"` → E0060) | ❌ Open |

**Remaining work for M4:**

#### 4.5 — Float literals in BytecodeCompiler
- File: `src/compiler/bytecode.rs`
- `Expression::Float(f)` currently not emitted as `PushConstant(Value::Float(f))`
- Completion gate: `test_parity_float_arithmetic` passes (currently `#[ignore]`)

#### 4.6 — Runtime type enforcement
- File: `src/runtime/execution/statements.rs`
- Annotated assignments (`store → x: int → "hello"`) must fail with `E0060`
- TypeChecker warning already fires; make it a hard runtime error when annotation present
- Completion gate: 5 tests pass; `--strict-types` still controls TypeChecker advisory mode

---

## Milestone 5 — Developer XP (v1.5.0)

**Blocked on M4 complete.**

#### 5.1 — LSP diagnostics push
- `textDocument/publishDiagnostics` on every `didChange` — **DONE** (Group T.1, 2026-03-25)
- Remaining: VS Code shows live error underlines for type mismatches without running the script

#### 5.2 — Debugger step-through
- `step`, `next`, `continue`, `print <expr>`, `stack` commands
- File: `src/cli/debug.rs`, `src/runtime/bytecode_vm.rs`
- Completion gate: step through a 20-line script line by line

#### 5.3 — Formatter: preserve comments + idempotent
- Preserve comments (currently stripped), idempotent formatting, `--check` flag for CI
- Completion gate: format twice = same result; 5 formatter tests pass

#### 5.4 — Error message quality
- Every `RuntimeError` includes line + column
- Suggestion text for top 10 common mistakes
- Completion gate: top 10 errors have location and hint

---

## Completed Groups (reference)

All Groups P–W are complete as of 2026-03-26. Key items:

| Group | What was done | Date |
|-------|--------------|------|
| P — Embed security | Validator enforced in `eval_inner()`; `with_sandbox()` added | 2026-03-25 |
| Q — Validator completeness | Undefined-var advisory; arity check; stdlib list synced | 2026-03-25 |
| R — IR layer | `src/ir/` created; `apply_to_ast()` mutates AST; constant folding live | 2026-03-26 |
| S — Backend decision | Option C (hybrid); bytecode 788 tests; parity suite added | 2026-03-26 |
| T — Test restructuring | `test_runtime.rs` split into 10 domain files; golden tests added | 2026-03-25 |
| U — FFI panic fix | `panic!()` → `RuntimeError` in `ffi.rs` | 2026-03-25 |
| V — Inference dead code | Dead constraints field removed; `#[allow(dead_code)]` resolved | 2026-03-25 |
| W — CLI pipeline cutover | All CLI commands use Builder; no direct VM imports in CLI | 2026-03-25 |
| Phase 3 | Stdlib validator auto-generated from `STDLIB_DISPATCH` | 2026-03-26 |
| Phase 4 | WASM TOCTOU bug fixed (`emit_artifact` uses existing Program) | 2026-03-26 |
| Phase 5 | REPL uses `Builder::create_repl_vm()` | 2026-03-26 |
| Phase 6 | Per-engine embed registry — no global collision | 2026-03-26 |
| Phase 7 | 20 parity tests; `run_ast_vm()` handles top-level return | 2026-03-26 |
| Pipeline audit | P1–P6 pipeline fixes + AUDIT-1–AUDIT-6; 805 tests; 0 failures | 2026-03-26 |

---

## Deferred (out of scope until M5 complete)

These items have code in the repo but are not blocked on for language correctness.
**Do not work on these until M4+M5 are done.**

| Item | File(s) | Why deferred | When to address |
|------|---------|--------------|----------------|
| Registry server | `src/bin/registry_server.rs` | Broken (`load_index()` discards content). No deployed backend. | Before public beta |
| Binary release CI | `.github/workflows/release.yml`, `install.sh` | Premature until language is stable | After M5 complete |
| Self-update | `src/security/update_verifier.rs` | Requires deployed CDN + binary releases | Same as binary releases |
| Docker images | — | Infrastructure concern; no user need during development | Before public launch |
| Playground deployment | `playground/`, `.github/workflows/playground.yml` | WASM backend not stable yet | After M4 WASM fixes |
| Community docs site | — | Marketing; docs should be stable first | Public beta |
| Registry publishing | `txtcode package publish` | Contacts non-existent server | After registry server fixed |
| Ed25519 release key | `src/security/auth.rs` | Placeholder key in source — **must replace before any public release** | Before first binary release |
| LLVM/Cranelift backend | `docs/llvm-backend.md` | v1.0+ work; Cranelift chosen (see archived design doc) | After M6 |
| Float literals in bytecode | `src/compiler/bytecode.rs` | Experimental path only | M4.5 |

---

## Known Technical Debt

| Issue | Location | Severity |
|-------|----------|----------|
| Float literals unimplemented in BytecodeCompiler | `src/compiler/bytecode.rs` | Medium |
| WASM jump instructions emit `;;` comments (broken control flow) | `src/compiler/wasm.rs` | Medium |

> **Resolved (2026-03-26):** Bench runner skips Validator (P1 fixed); REPL `:type` used separate TypeInference engine (P4 fixed); REPL `block_depth` underflow (AUDIT-2 fixed).

---

## Definition of Done

Txtcode is **genuinely usable** when a developer can:

- [x] Write a script that calls HTTPS API, parses JSON, stores results in SQLite — **M1+M3**
- [x] Run 5 API calls in parallel with `async_run` + `await_all` — **M2**
- [x] Embed the interpreter in a Rust/C application safely — **M3+P**
- [ ] Get red squiggles in VS Code for type errors without running the script — **M5**
- [ ] Step through a failing script in the debugger — **M5**
- [ ] `txtcode package install` a real package from the live registry — **M6**

---

## History

Past plan versions (Groups 1–29, Groups A–J, Groups P–W) are in `docs/archive/README.md` and git history.
