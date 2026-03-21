# Txtcode Development Plan
**Version:** 0.8.0 → 1.0.0
**Last Updated:** 2026-03-20 (session 4)
**Status Legend:** `[ ]` todo · `[~]` in progress · `[x]` done · `[!]` blocked

## Vision (updated 2026-03-19)

Txtcode is a **multipurpose, security-native language platform**.
Its DNA is: general programming + security built in + networking built in + automation built in + safe execution built in.
It is NOT specialized — the same language can build a web server, a security scanner, an automation pipeline, a CLI tool, or a policy enforcer.
See `NON-GOALS.md` for the updated boundary.

---

## How to Use This Document

- Work **group by group**, top to bottom
- Within a group, work **task by task**, top to bottom
- Do NOT start Group N+1 until ALL tasks in Group N are `[x]`
- Each task has a **Target File**, **What to Do**, and **Done When** section
- Update status symbols as you go
- This file lives at `docs/dev-plan.md` — commit it after each session

---

## Current Position

```
Group 1:  Foundation Stability          [x] COMPLETE (244 tests passing)
Group 2:  Language Completeness         [x] COMPLETE (261 tests passing)
Group 3:  Type Enforcement              [x] COMPLETE (179 tests passing)
Group 4:  Async Runtime                 [x] COMPLETE (179 tests passing)
Group 5:  Stdlib Gaps                   [x] COMPLETE (194 tests passing)
Group 6:  Ecosystem                     [x] COMPLETE (194 tests passing)
Group 7:  Performance Baseline          [x] COMPLETE (194 tests passing)
──────────────────────────────────────────────────────────────────────
Group 8:  Security Correctness          [x] COMPLETE (202 tests passing)
Group 9:  Module System Overhaul        [x] COMPLETE (209 tests passing)
Group 10: Type System Promotion         [x] COMPLETE (311 tests passing)
Group 11: Developer Experience          [x] COMPLETE (322 tests passing)
──────────────────────────────────────────────────────────────────────
Group 12: Platform & Compilation        [x] COMPLETE (335 tests passing)
──────────────────────────────────────────────────────────────────────
Group 13: Language Correctness          [x] COMPLETE (259 tests passing)
Group 14: Language Completeness II      [x] COMPLETE (277 tests passing)
Group 15: Runtime & Async Overhaul      [x] COMPLETE (target: v1.0.0-alpha)
Group 16: Stdlib — Networking & Security[x] COMPLETE (target: v1.0.0-beta)
Group 17: Stdlib — Application Layer    [x] COMPLETE (target: v1.0.0-rc)
Group 18: Tooling & Developer XP        [x] COMPLETE (target: v1.0.0)
Group 19: Ecosystem & Platform          [x] COMPLETE (target: v1.0.0-release)
──────────────────────────────────────────────────────────────────────
Group 20: Audit Gap Closure I           [~] in progress (target: v1.1.0)
  20.1 Stdlib Test Coverage             [x] COMPLETE (363 tests)
  20.2 Real Async (tokio)               [ ] pending
  20.3 LSP publishDiagnostics           [x] COMPLETE (368 tests)
Group 21: Audit Gap Closure II          [ ] pending (target: v1.2.0)
  21.1 Bytecode VM Parity               [ ] pending
  21.2 Runtime Type Enforcement         [ ] pending
  21.3 Error Message Quality            [ ] pending
Group 22: Platform Live                 [ ] pending (target: v2.0.0)
  22.1 Deploy Package Registry          [ ] pending
  22.2 Native Plugin System             [ ] pending
  22.3 VS Code Extension                [ ] pending
```

---

## Audit Findings Summary (what drove Groups 8–12)

The v0.5.0 senior audit (2026-03-19) found these blocking issues:

| # | Issue | Severity | Group |
|---|-------|----------|-------|
| A1 | `exec_allowed: true` default contradicts "security-first" | Critical | 8 |
| A2 | `const` keyword has no runtime enforcement | Critical | 8 |
| A3 | Script signing not exposed as CLI flag | High | 8 |
| A4 | Module imports pollute caller namespace | Critical | 9 |
| A5 | No circular import detection | High | 9 |
| A6 | Transitive dep resolution missing | High | 9 |
| A7 | `Txtcode.lock` not verified on load | High | 9 |
| A8 | Type errors advisory by default — most users never see them | Critical | 10 |
| A9 | Generics parse-and-erase — constraints never enforced | High | 10 |
| A10 | No pre-built binaries — `install.sh` requires Rust toolchain | Critical | 11 |
| A11 | Debugger not wired to interactive CLI (bytecode addresses only) | High | 11 |
| A12 | `txtcode doc` is effectively absent | Medium | 11 |
| A13 | LSP missing go-to-definition, hover, rename | Medium | 11 |
| A14 | No native/WASM compilation path | Medium | 12 |
| A15 | Async uses OS threads — no event loop, no scale | Medium | 12 |
| A16 | Struct methods (impl blocks) missing | Medium | 12 |
| A17 | `ffi_load` allows any path — no allowlist | High | 8 |
| A18 | HashMap iteration order non-deterministic — breaks "determinism" claim | Medium | 9 |

---

---

# GROUPS 1–7 — COMPLETE (v0.4 → v0.5.0)

> All tasks in Groups 1–7 are `[x]`. See git history for implementation details.
> Groups 1–7 delivered: lexer, parser, AST VM, bytecode VM, 6-layer security pipeline,
> 100+ stdlib functions, 20 packages, LSP server, TextMate grammar, performance docs.

---

---

# GROUP 8 — Security Correctness
**Goal:** Every security claim in the README and docs is actually true in the code.
**Unblocked by:** Nothing — start here.
**Output:** Default-deny exec; const enforced; signing as CLI flag; FFI path guard.
**Target version:** 0.5.1

---

## Task 8.1 — Fix `exec_allowed: true` Default (CRITICAL)

**Status:** `[x]`
**Risk:** CRITICAL — directly contradicts the "security-first, default-deny" positioning
**Estimated size:** Small (1–3 files)

### Problem

`src/runtime/vm/core.rs` line 37: `exec_allowed: true` in `VirtualMachine::new()`.
Any script can call `exec()`, `exec_json()`, `exec_lines()`, `exec_pipe()`, `spawn()` without
any permission grant unless `safe_mode = true` or `--safe-mode` is explicitly passed.
The 6-layer security pipeline is bypassed for process execution by default.

### What to Do

**Step 1 — Change default**
- File: `src/runtime/vm/core.rs`
- Change `exec_allowed: true` → `exec_allowed: false`
- File: `src/runtime/vm/core.rs` — `with_all_options()` constructor
- Same change: default `exec_allowed: false`

**Step 2 — Add --allow-exec CLI flag**
- File: `src/bin/txtcode.rs`
- Add `RunArgs`: `--allow-exec` flag
- File: `src/cli/run.rs`
- Wire: if `--allow-exec` passed, call `vm.set_exec_allowed(true)` before execution

**Step 3 — Update permission_map**
- File: `src/runtime/permission_map.rs`
- Ensure `exec`, `exec_json`, `exec_lines`, `exec_pipe`, `exec_status`, `spawn`, `kill`,
  `signal_send`, `pipe_exec` all map to `PermissionResource::System("exec")`
- These should fail with clear `PermissionError` if `exec_allowed == false` and no grant

**Step 4 — Update tests**
- File: `tests/integration/test_runtime.rs`
- Add test: calling `exec("echo", [])` without `--allow-exec` or `grant_permission` → error
- Add test: calling `exec` after `grant_permission("sys.exec", null)` → succeeds

**Step 5 — Update docs**
- File: `README.md`, `docs/permissions.md`, `docs/security-features.md`
- Change: `exec_allowed` default is now `false`; add `--allow-exec` to CLI reference

### Done When
- `txtcode run untrusted.tc` (containing `exec("rm", ["-rf", "/tmp/test"])`) → permission error
- `txtcode run --allow-exec script.tc` → exec works
- `grant_permission("sys.exec", null)` in script → exec works
- All 284+ existing tests pass (update exec-using tests to grant `sys.exec` first)

---

## Task 8.2 — Enforce `const` at Runtime

**Status:** `[x]`
**Risk:** MEDIUM — silent mutation of "constants" is a correctness hazard
**Estimated size:** Small-Medium

### Problem
`const x → 5` parses and stores the variable but a subsequent `store → x → 6` succeeds
silently. `const` is currently pure syntactic sugar with no enforcement in either VM.

### What to Do

**Step 1 — Track constants in scope**
- File: `src/runtime/core.rs` (ScopeManager or Scope struct)
- Add: `constants: HashSet<String>` to the scope that owns the variable
- When `Statement::Const { name, value }` executes: add `name` to `constants` after storing

**Step 2 — Guard reassignment**
- File: `src/runtime/execution/statements/` (assignment handler)
- In `execute_assignment`: before overwriting a variable, check if it is in `constants`
- If const: return `RuntimeError` with `E0030` — "cannot reassign const 'x'"

**Step 3 — Bytecode VM**
- File: `src/runtime/bytecode_vm.rs`
- `StoreVar` instruction: check constants set before writing
- `Instruction::Const { name, value }`: new instruction variant, or reuse `StoreVar` with a
  const flag in the frame

**Step 4 — Error code**
- File: `src/runtime/errors.rs`
- Add: `E0030` — "cannot reassign const variable"

**Step 5 — Tests**
- Test: `const x → 5` followed by `store → x → 6` raises `E0030`
- Test: `const x → 5` followed by `print → x` works normally

### Done When
- Reassigning a `const` raises `E0030` in both VMs
- `cargo test` passes

---

## Task 8.3 — Expose Script Signing as CLI Flag

**Status:** `[x]`
**Risk:** MEDIUM — signing infrastructure exists but is unreachable without Rust API
**Estimated size:** Small

### Problem
`src/security/auth.rs` — Ed25519 signing/verification exists but there is no CLI surface.
Users cannot sign or verify scripts from the command line. The security feature is invisible.

### What to Do

**Step 1 — Add `txtcode sign` subcommand**
- File: `src/bin/txtcode.rs`
- Add `Commands::Sign { file: PathBuf, key: PathBuf, output: Option<PathBuf> }`
- Description: "Sign a .tc script with an Ed25519 private key"
- File: `src/cli/sign.rs` (new)
- `pub fn run(file, key_path, output)`:
  - Read source bytes
  - Load private key from `key_path`
  - Call `ScriptAuth::sign(source_bytes, signer_id, &priv_key)`
  - Write signature to `output` (default: `file.sig`)

**Step 2 — Add `txtcode verify` subcommand**
- File: `src/bin/txtcode.rs`
- Add `Commands::Verify { file: PathBuf, sig: PathBuf }`
- File: `src/cli/sign.rs` (extend)
- `pub fn verify(file, sig_path)`:
  - Read source and sig
  - Call `ScriptAuth::verify(source_bytes, &sig)`
  - Print `OK` or `TAMPERED` with exit code

**Step 3 — Add `--require-sig` to `txtcode run`**
- File: `src/bin/txtcode.rs`, `src/cli/run.rs`
- `--require-sig <KEY_FILE>` — before execution, verify `.tc.sig` sidecar with the given public key
- If sig missing or invalid: abort with error

**Step 4 — Add `txtcode keygen` subcommand**
- `txtcode keygen --output keys/` — generates `private.key` + `public.key` in the output dir
- Prints warning: "Keep private.key secret. Distribute public.key with your package."

### Done When
- Full round-trip: `txtcode keygen`, `txtcode sign script.tc`, `txtcode run --require-sig public.key script.tc` all work
- `cargo test` passes

---

## Task 8.4 — FFI Path Allowlisting

**Status:** `[x]`
**Risk:** HIGH — `ffi_load` with attacker-controlled path = arbitrary code execution
**Estimated size:** Small

### Problem
`src/stdlib/ffi.rs` — `ffi_load(path)` accepts any filesystem path. If a script can
control the path argument, it can load any shared library and execute arbitrary C code.
The `sys.ffi` permission gate is necessary but not sufficient — there is no path restriction.

### What to Do

**Step 1 — Add FFI path allowlist to permission grant**
- File: `src/runtime/permissions.rs`
- `PermissionResource::System("ffi")` gains optional scope (like FileSystem)
- Scope is a path prefix: `grant_permission("sys.ffi", "/usr/lib/*")` only allows libs under `/usr/lib/`

**Step 2 — Enforce in ffi_load**
- File: `src/stdlib/ffi.rs`
- Before `Library::new(path)`: call `vm.check_permission(System("ffi"), Some(path))`
- The scope matching in PermissionManager already handles glob patterns

**Step 3 — CLI flag**
- File: `src/bin/txtcode.rs`
- Add `--allow-ffi PATH` alongside `--allow-fs`, `--allow-net`

**Step 4 — Tests**
- Test: `ffi_load("/evil/lib.so")` without allowlist grant → error
- Test: `ffi_load("/usr/lib/libm.so")` with `--allow-ffi /usr/lib/*` → succeeds

### Done When
- `ffi_load` with un-allowlisted path raises permission error
- `cargo test` passes

---

## Task 8.5 — Audit Log Persistence

**Status:** `[x]`
**Risk:** MEDIUM — in-memory audit trail is lost on exit; security use-cases need durable logs
**Estimated size:** Small-Medium

### Problem
`AuditTrail` is in-memory only. No persistence. For audited automation this is a gap.

### What to Do

**Step 1 — Add `--audit-log FILE` to `txtcode run`**
- File: `src/bin/txtcode.rs`, `src/cli/run.rs`
- On `--audit-log <path>`: after execution completes, serialize audit trail to JSON and write

**Step 2 — AuditTrail serialization**
- File: `src/runtime/audit.rs`
- Add `#[derive(Serialize)]` to `AuditEvent`, `AuditTrail`
- Add `fn to_json(&self) -> String` using `serde_json::to_string_pretty`

**Step 3 — Streaming write (optional)**
- For long-running scripts: write each audit event to the log file as it occurs
- Use `BufWriter<File>` opened at startup and flushed after each event

**Step 4 — Tests**
- Test: `txtcode run --audit-log /tmp/audit.json script.tc` → audit file exists with correct events

### Done When
- `--audit-log` flag writes a valid JSON audit trail file
- `cargo test` passes

---

## Group 8 Checkpoint

```
[x] exec_allowed defaults to false; exec requires explicit grant or --allow-exec
[x] const reassignment raises E0030 in both VMs
[x] txtcode sign / verify / keygen / run --require-sig all work end-to-end
[x] ffi_load requires path-scoped sys.ffi permission grant
[x] --audit-log flag writes persistent JSON audit trail
[x] cargo test passes (all existing tests updated where needed)
```

---

---

# GROUP 9 — Module System Overhaul
**Goal:** Multi-file projects work correctly without namespace collisions or reproducibility risks.
**Unblocked by:** Group 8 complete.
**Output:** Isolated module namespaces; locked transitive deps; deterministic map iteration.
**Target version:** 0.6.0

---

## Task 9.1 — Module Namespace Isolation (CRITICAL)

**Status:** `[x]`
**Risk:** CRITICAL — without this, any multi-file project risks silent name collisions
**Estimated size:** Large

### Problem
`src/runtime/module.rs` — When a module is imported, its definitions are executed in the
caller's scope. Two modules that both define `format()` silently overwrite each other.
There is no module-level namespace. `exported_symbols` set exists but is not enforced.

### What to Do

**Step 1 — Execute modules in isolated scope**
- File: `src/runtime/module.rs`
- `load_module(name)` must:
  1. Create a fresh child VM with a NEW isolated scope (not the caller's scope)
  2. Execute the module AST in that isolated VM
  3. Collect only the symbols listed in `export` statements from the isolated scope
  4. Return a `ModuleExports { symbols: HashMap<String, Value> }`

**Step 2 — Qualified import binding**
- When `from "npl-math/math" import gcd, factorial` is executed:
  - Extract only `gcd` and `factorial` from `ModuleExports`
  - Bind them in the caller's scope under their imported names
  - Wildcard `import *` binds all exports (with warning)
- When `import "npl-math/math" as math` is executed:
  - Create a `Value::Map` of all exports
  - Bind it as `math` in the caller's scope
  - Access via `math.gcd(a, b)`

**Step 3 — Export enforcement**
- File: `src/runtime/execution/statements/` (export statement handler)
- Only symbols declared with `export` in the module are visible to importers
- Private module-internal functions/variables remain invisible

**Step 4 — Circular import detection**
- File: `src/runtime/module.rs`
- The existing `import_stack: Vec<String>` in VirtualMachine is the right tool
- Before loading module N: check if N is already in `import_stack`
- If yes: return `RuntimeError` — "circular import: module 'A' → 'B' → 'A'"

**Step 5 — Bytecode VM module parity**
- File: `src/runtime/bytecode_vm.rs` — `ImportModule` instruction handler
- Same isolation: sub-VM, collect exports, bind in caller scope
- Currently sub-VM shares too much state — fix the isolation boundary

**Step 6 — Tests**
- Test: two modules both define `helper()` — no collision
- Test: circular import raises RuntimeError
- Test: unexported symbol not accessible in importer
- Test: `import as` creates qualified access

### Done When
- Two modules with same function name coexist without collision
- Circular imports raise an error instead of infinite loop/panic
- Unexported symbols are not visible to importers
- `cargo test` passes

---

## Task 9.2 — Transitive Dependency Resolution

**Status:** `[x]`
**Risk:** HIGH — `package install` silently skips transitive deps
**Estimated size:** Medium

### Problem
`src/cli/package.rs` — `install_dependencies()` installs direct deps listed in `Txtcode.toml`
but does not recursively install their dependencies. A package that depends on `npl-collections`
will fail at import time if the user only installed the top-level package.

### What to Do

**Step 1 — Recursive resolve function**
- File: `src/cli/package.rs`
- Add: `fn resolve_transitive(name, ver, registry, visited) -> Vec<(String, String)>`
- Algorithm:
  1. Look up package in registry
  2. For each dep in `entry.dependencies`: if not in `visited`, recurse
  3. Return flat list of all packages to install (deps before dependents)
- Use `visited: HashSet<String>` to break cycles

**Step 2 — Conflict detection**
- If two packages require different incompatible versions of the same dep:
  - Print warning: "conflict: pkg-A needs foo ^1.0, pkg-B needs foo ^2.0"
  - For now: install latest compatible version; print warning

**Step 3 — Update install flow**
- `package install` calls `resolve_transitive` before installing anything
- Prints: "Resolving dependencies... installing X packages"

**Step 4 — Tests**
- Test: installing a package with deps installs all transitive deps
- Test: dep cycle in registry is detected and reported

### Done When
- `txtcode package install npl-http-client` automatically installs `npl-retry` if it is a dep
- `cargo test` passes

---

## Task 9.3 — Lockfile Enforcement

**Status:** `[x]`
**Risk:** HIGH — without lockfile verification, builds are not reproducible
**Estimated size:** Small-Medium

### Problem
`Txtcode.lock` is written by `package install` but never read back to verify installed
packages match the lock. Every install resolves fresh from the registry regardless.

### What to Do

**Step 1 — Write lockfile with transitive deps and hashes**
- File: `src/cli/package.rs`
- After `resolve_transitive`: write `Txtcode.lock` (TOML format)
```toml
[[package]]
name = "npl-math"
version = "0.1.0"
sha256 = "abc123..."
source = "local:packages/npl-math"

[[package]]
name = "npl-collections"
version = "0.1.0"
sha256 = "def456..."
source = "local:packages/npl-collections"
```

**Step 2 — Verify on install (lock mode)**
- `package install` (no args): if `Txtcode.lock` exists, read locked versions
  - Do NOT resolve from registry — install exactly what lock says
  - Verify SHA-256 of each installed package against lock entry
  - If hash mismatch: abort with error "lockfile hash mismatch for npl-math"
- `package update`: re-resolve, update lock file

**Step 3 — Verify on module import (optional strict mode)**
- `--strict-lock` flag on `txtcode run`: verify loaded modules are in lock
- If a module is loaded that is not in `Txtcode.lock`: error (possible tampered install)

**Step 4 — Tests**
- Test: install with lock → subsequent install uses lock, not fresh resolve
- Test: modified package fails hash check

### Done When
- `Txtcode.lock` read and verified on `package install`
- `package update` regenerates lock
- `cargo test` passes

---

## Task 9.4 — Deterministic Map Iteration

**Status:** `[x]`
**Risk:** MEDIUM — breaks "deterministic execution" claim; hard to debug
**Estimated size:** Small

### Problem
`Value::Map` uses `HashMap` internally. Iterating over a map in a `for` loop produces
different key orders across runs. This breaks the "deterministic execution" claim in docs
and makes scripts that process maps non-reproducible.

### What to Do

**Step 1 — Switch to IndexMap**
- File: `Cargo.toml` — add `indexmap = "2"`
- File: `src/runtime/core/value.rs` — change `HashMap<String, Value>` in `Value::Map` to
  `IndexMap<String, Value>` (insertion-ordered)
- This preserves insertion order, making iteration deterministic

**Step 2 — Update all map construction sites**
- Run: `grep -r "HashMap::new()" src/ --include="*.rs"` for map value construction
- Change relevant sites to `IndexMap::new()`

**Step 3 — Update sort for display**
- `print(my_map)` — display order is now insertion order (deterministic)

**Step 4 — Tests**
- Test: `store → m → {a: 1, b: 2, c: 3}` iterated with `for` → always yields a, b, c in order

### Done When
- Map iteration order matches insertion order
- All existing map tests pass
- `cargo test` passes

---

## Task 9.5 — Update "Deterministic" Documentation

**Status:** `[x]`
**Risk:** LOW — documentation fix
**Estimated size:** Tiny

### What to Do
- File: `README.md`, `docs/index.md`, `docs/language-spec.md`
- Replace vague "deterministic execution" with accurate description:
  "**Permission-transparent execution** — every privileged side effect requires an
  explicit grant; no hidden network, filesystem, or process calls."
- Add note: "Map iteration order is insertion-order (deterministic as of v0.6)."
- Remove any claim that non-deterministic operations (HTTP, time, exec) are deterministic.

### Done When
- Docs accurately describe what "deterministic" means in Txtcode context

---

## Group 9 Checkpoint

```
[x] Module imports execute in isolated scope; unexported names invisible to importer
[x] Circular imports detected and reported with clear error
[x] Transitive deps resolved and installed automatically
[x] Txtcode.lock written and verified on install; package update regenerates it
[x] Map iteration order is insertion-order (IndexMap)
[x] "Deterministic" documentation corrected in README and spec
[x] cargo test passes
```

---

---

# GROUP 10 — Type System Promotion
**Goal:** Type annotations are enforced by default, not decorative.
**Unblocked by:** Group 9 complete.
**Output:** Type errors are warnings by default; generics have partial runtime support.
**Target version:** 0.6.5

---

## Task 10.1 — Make Type Warnings the Default

**Status:** `[x]`
**Risk:** HIGH — changing default behavior; needs clean migration path
**Estimated size:** Medium

### Problem
Type errors are silently ignored unless `--strict-types` is passed. Most users run
`txtcode run script.tc` and never see type mismatches. Type annotations are decoration.

### What to Do

**Step 1 — Add three type-check modes**
- `--no-type-check` — skip type checker entirely (old default behavior)
- *(new default)* — run type checker; print warnings for violations; continue execution
- `--strict-types` — run type checker; abort on any violation (existing flag)

**Step 2 — Wire default mode in run.rs**
- File: `src/cli/run.rs`
- After parsing, always run `TypeChecker::check(program)`
- If warnings: print each with `[WARNING] type: ...` prefix (do NOT abort)
- If `--strict-types`: abort with exit 1

**Step 3 — Suppress with --no-type-check**
- Existing scripts that have type-annotation-free code continue to work silently
- Scripts with type annotations see useful feedback immediately

**Step 4 — Formatter: preserve type annotations**
- File: `src/tools/formatter.rs`
- Ensure type annotations are not stripped by formatter

**Step 5 — Tests**
- Test: script with type mismatch prints warning but runs
- Test: `--strict-types` aborts on same script
- Test: `--no-type-check` prints nothing

### Done When
- Default `txtcode run` shows type warnings without aborting
- `--strict-types` aborts
- `--no-type-check` silences all type output
- `cargo test` passes

---

## Task 10.2 — Enforce Generics for Built-in Collection Types

**Status:** `[x]`
**Risk:** MEDIUM — partial implementation; full generics deferred to v0.8
**Estimated size:** Medium-Large

### Problem
Generic parameters `<T>` are parsed then discarded. `Array<int>` and `Array<string>` are
identical at runtime. Partial enforcement for the built-in collection types is achievable
without a full generics implementation.

### What to Do

**Step 1 — Track element type on typed array/map literals**
- File: `src/typecheck/checker.rs`
- When type annotation says `Array<int>` and an array literal contains a non-int: type warning/error
- Same for `Map<string, int>` — key must be string, value must be int

**Step 2 — Enforce on push/insert**
- When `array_push(arr, value)` is called and arr has a declared element type:
  - In strict mode: error if `value` does not match element type
  - In default mode: warning

**Step 3 — Type-erase at module boundary**
- Generic type params on user-defined functions remain erased (v0.8 work)
- Only built-in collection type annotations (`Array<T>`, `Map<K,V>`) get enforcement

**Step 4 — Tests**
- Test: `store → nums: Array<int> → [1, 2, "three"]` → warning in default, error in strict

### Done When
- Built-in collection type annotations produce meaningful errors/warnings when violated
- User-defined generic functions still compile (type-erased, no change)
- `cargo test` passes

---

## Task 10.3 — Type Checker Coverage Expansion

**Status:** `[x]`
**Risk:** LOW — additive; improves existing checker without breaking changes
**Estimated size:** Medium

### Problem
The type checker (`src/typecheck/checker.rs`) has basic coverage but misses:
- Function return type mismatch
- Calling a non-function value
- Passing wrong number of arguments
- `null` used in arithmetic context

### What to Do

**Step 1 — Return type checking**
- In `check_function_def`: track return type annotation
- In `check_return`: compare returned expression type against declared return type
- Warn/error on mismatch

**Step 2 — Arity checking**
- In `check_function_call`: if callee is known (defined in same file), check arg count
- Warn if arg count != param count (excluding variadic functions)

**Step 3 — Null arithmetic warning**
- In `check_binary_op`: if either operand might be `Null`, warn
- "Potential null dereference in arithmetic: left operand may be null"

**Step 4 — Tests**
- Test: function declared `→ int` but returns string → warning
- Test: calling `f(a, b, c)` where `f` takes 2 params → warning
- Test: `x + null` → warning

### Done When
- Return type, arity, and null arithmetic checks work in default and strict modes
- `cargo test` passes

---

## Group 10 Checkpoint

```
[x] Default txtcode run shows type warnings without aborting
[x] --strict-types aborts on type violations
[x] --no-type-check silences type output
[x] Array<T> / Map<K,V> annotations produce errors/warnings when violated
[x] Return type checking implemented in type checker
[x] Arity checking implemented in type checker
[x] Null arithmetic warnings implemented
[x] cargo test passes (311 tests)
```

---

---

# GROUP 11 — Developer Experience
**Goal:** Txtcode is installable without Rust; debugger is actually usable; LSP is complete enough for real editing.
**Unblocked by:** Group 8 complete (can work in parallel with Groups 9 and 10).
**Output:** Pre-built binaries; interactive debugger CLI; doc generation; complete LSP.
**Target version:** 0.7.0

---

## Task 11.1 — Pre-Built Binaries and Release Pipeline (CRITICAL)

**Status:** `[x]`
**Risk:** CRITICAL — without binaries, `install.sh` requires Rust toolchain; blocks all non-Rust users
**Estimated size:** Small (CI configuration)

### Problem
`release/` directory is empty. `install.sh` falls back to `cargo build` when no binary is found.
This requires a Rust toolchain install (~1GB, ~10 min compile) as a prerequisite.
No pre-built binaries exist on GitHub Releases.

### What to Do

**Step 1 — GitHub Actions release workflow**
- File: `.github/workflows/release.yml` (new)
- Trigger: `on: push: tags: 'v*'`
- Matrix: `[ubuntu-latest, macos-latest, windows-latest]`
- Steps per platform:
  1. `cargo build --release`
  2. Strip binary (Linux/macOS)
  3. Rename: `txtcode-linux-x86_64`, `txtcode-macos-aarch64`, `txtcode-windows-x86_64.exe`
  4. Upload to GitHub Release as asset
  5. Compute SHA-256 and upload `checksums.txt`

**Step 2 — Update install.sh**
- File: `install.sh`
- Detect OS + arch
- Construct download URL: `https://github.com/iga2x/txtcode/releases/latest/download/txtcode-<os>-<arch>`
- Download binary, verify SHA-256 against `checksums.txt`
- Install to `~/.local/bin/txtcode`
- Fall back to `cargo build` only if no binary exists for the platform

**Step 3 — Add cross-compilation targets**
- `Cargo.toml`: add `.cargo/config.toml` with cross-compilation targets
- Additional targets: `aarch64-unknown-linux-gnu`, `x86_64-pc-windows-gnu`
- Use `cross` tool or GitHub Actions matrix for cross-compilation

**Step 4 — Test the installer**
- Test: `curl -sSf .../install.sh | sh` on clean Ubuntu 22.04 (no Rust) → txtcode installed
- Test: `txtcode --version` works after install

### Done When
- GitHub Release has binaries for Linux x86_64, macOS arm64, Windows x86_64
- `install.sh` installs from binary on these platforms (no cargo needed)
- `txtcode doctor` reports correct installation

---

## Task 11.2 — Interactive Debugger CLI

**Status:** `[x]`
**Risk:** MEDIUM — infrastructure exists; needs interactive loop and source-line mapping
**Estimated size:** Medium

### Problem
`src/tools/debugger.rs` — `step()`, `continue_execution()`, `inspect_variable()` all work
at bytecode level. But `txtcode debug file.tc` compiles and runs without stopping — there
is no interactive loop exposed to the user. Breakpoints are by bytecode address (not line).

### What to Do

**Step 1 — Add debug symbol table to compiler**
- File: `src/compiler/bytecode.rs`
- Add: `debug_info: Vec<(usize, usize)>` to `Bytecode` struct
  - Each entry: `(instruction_index, source_line_number)`
- Compiler: when emitting each instruction, record the source line from the AST span

**Step 2 — Add line-based breakpoints to Debugger**
- File: `src/tools/debugger.rs`
- Add: `fn add_breakpoint_at_line(&mut self, line: usize)` — looks up line in debug_info,
  adds the corresponding instruction index to `breakpoints`

**Step 3 — Interactive REPL loop in `txtcode debug`**
- File: `src/cli/debug_cmd.rs` (new, or extend existing)
- Start debugger, enter loop:
  ```
  (txtcode-dbg) run          → run until breakpoint or end
  (txtcode-dbg) step         → execute one instruction, print state
  (txtcode-dbg) break 15     → set breakpoint at source line 15
  (txtcode-dbg) print x      → print variable x
  (txtcode-dbg) vars         → print all variables in scope
  (txtcode-dbg) stack        → print operand stack
  (txtcode-dbg) continue     → resume until next breakpoint
  (txtcode-dbg) quit         → exit debugger
  ```
- Use `rustyline` (already may be a dep via REPL) for readline in debugger loop

**Step 4 — Source context display**
- When breaking at instruction I: print the source line that generated it
  (read source file, display line N with `→` marker)

**Step 5 — Tests**
- Test: debugger breaks at line 5 when `break 5` set before `run`
- Test: `step` advances one instruction at a time
- Test: `print x` shows current value of variable

### Done When
- `txtcode debug script.tc` enters interactive loop
- `break <line>`, `step`, `continue`, `print <var>`, `vars`, `quit` all work
- Source line shown at each break

---

## Task 11.3 — Real Doc Generation (`txtcode doc`)

**Status:** `[x]`
**Risk:** MEDIUM — `txtcode doc` listed in CLI but produces no output
**Estimated size:** Medium

### Problem
`txtcode doc` subcommand is listed in CLI help but does nothing useful. There is no
documentation generation for Txtcode packages or scripts.

### What to Do

**Step 1 — Extract doc comments from source**
- File: `src/tools/docgen.rs` (new)
- Parse `.tc` files; extract `##` block comments immediately before `define → fn → (params)` or `struct`
- Build: `DocItem { kind: Function|Struct, name, params, doc_comment, return_type }`

**Step 2 — Markdown output**
- File: `src/tools/docgen.rs`
- Generate: one `.md` file per `.tc` source file
- Format per function:
  ```markdown
  ## gcd(a, b)
  Greatest common divisor of a and b.
  **Parameters:** a: int, b: int
  **Returns:** int
  ```

**Step 3 — Wire to CLI**
- File: `src/cli/doc_cmd.rs` (new or extend existing)
- `txtcode doc [path]` — default: scan `src/` and `packages/`, output to `docs/api/`
- `txtcode doc --format json` — output JSON for tooling
- `txtcode doc --output DIR` — custom output directory

**Step 4 — Package doc index**
- For packages: generate `docs/api/index.md` listing all packages with their exports

**Step 5 — Tests**
- Test: `txtcode doc examples/hello_world.tc` produces a markdown file with function docs

### Done When
- `txtcode doc packages/npl-math` generates correct markdown documentation
- `txtcode doc --format json` produces parseable JSON
- `cargo test` passes

---

## Task 11.4 — LSP: Go-to-Definition, Hover, Rename

**Status:** `[x]`
**Risk:** MEDIUM — LSP server exists; needs symbol resolution
**Estimated size:** Medium-Large

### Problem
`src/cli/lsp.rs` — handles `initialize`, diagnostics, completion. Does NOT handle:
- `textDocument/definition` — go-to-definition
- `textDocument/hover` — show function signature on hover
- `textDocument/references` — find all usages
- `textDocument/rename` — rename symbol across file
- `textDocument/signatureHelp` — show param info while typing

Without these, the LSP is not useful beyond syntax highlighting and basic completion.

### What to Do

**Step 1 — Build a symbol table from AST**
- File: `src/tools/symbol_table.rs` (new) or extend type checker
- `SymbolTable` built by walking AST:
  - Maps each function/variable name → definition location (file, line, col)
  - Maps each usage location → definition location

**Step 2 — textDocument/definition**
- In `lsp.rs`: handle `textDocument/definition` request
- Given cursor position: find the token at that position, look up in symbol table
- Return: `Location { uri, range }` of the definition

**Step 3 — textDocument/hover**
- Handle `textDocument/hover`
- For a function call: return `{ contents: "fn_name(params) → return_type\ndoc_comment" }`
- For a variable: return its inferred type (from type checker)

**Step 4 — textDocument/rename**
- Handle `textDocument/rename`
- Find all occurrences of the symbol in the file
- Return `WorkspaceEdit` replacing each occurrence

**Step 5 — Async LSP (non-blocking)**
- Current LSP is synchronous stdin/stdout blocking
- For large files, diagnostics on every keystroke will block
- Add debounce: only re-parse 300ms after last change

**Step 6 — Tests**
- Integration test: send `textDocument/definition` LSP request, get correct location back
- Integration test: `textDocument/hover` returns function signature

### Done When
- Go-to-definition works for functions defined in the same file
- Hover shows function signature and doc comment
- Rename works within a single file
- LSP debounces re-parsing

---

## Task 11.5 — REPL Multiline Input and History

**Status:** `[x]`
**Risk:** LOW — quality of life
**Estimated size:** Small-Medium

### Problem
REPL does not support multiline input. Typing a `define → f → (x)` and pressing Enter
immediately tries to parse the incomplete input and fails. No `...` continuation prompt.
History works per-line but multi-line blocks cannot be re-used from history.

### What to Do

**Step 1 — Detect incomplete input**
- File: `src/cli/repl.rs`
- After each input line: try to parse
- If parse error is "unexpected EOF" (unclosed `define...end`, `if...end`, `while...end`):
  - Show `...` prompt and accumulate lines
- Else: execute immediately

**Step 2 — Continuation prompt**
- Current prompt: `txtcode> `
- Continuation prompt: `    ...  ` (aligned with 4-space indent)

**Step 3 — Persistent history**
- Save REPL history to `~/.txtcode/repl_history` across sessions
- Load on startup: last 1000 entries

**Step 4 — `:clear` and `:reset` commands**
- `:clear` — clear screen
- `:reset` — reset VM state (clear all variables)
- `:help` — show available commands

### Done When
- Multi-line `define → f → (x)\n  return x\nend` works in REPL
- History persists across sessions
- `cargo test` passes

---

## Group 11 Checkpoint

```
[x] Pre-built binaries on GitHub Releases for Linux x86_64, macOS arm64, Windows x64
[x] install.sh installs from binary without requiring Rust toolchain
[x] txtcode debug <file> enters interactive loop with break/step/print/continue/quit
[x] Source line shown at each debugger break
[x] txtcode doc generates markdown API docs from ## comments
[x] LSP: go-to-definition works for same-file symbols
[x] LSP: hover shows function signature
[x] REPL: multiline input with continuation prompt
[x] REPL: history persists across sessions
[x] cargo test passes (322 tests)
```

---

---

# GROUP 12 — Platform and Compilation
**Goal:** Txtcode has a path to native performance and broader deployment.
**Unblocked by:** Groups 9 and 10 complete.
**Output:** Async event loop; struct methods; WASM target; LLVM planning.
**Target version:** 0.8.0

---

## Task 12.1 — Async Event Loop (Tokio Integration)

**Status:** `[x]`
**Risk:** HIGH — architectural change to how async functions execute
**Estimated size:** Very Large

### Problem
Current async: `async define` spawns one OS thread per call. This does not scale.
`http_serve` spawns a thread per request. 100 concurrent requests = 100 threads.
No `select`, no `join!`, no cancellation, no backpressure.

### What to Do

**Step 1 — Add Tokio runtime to VirtualMachine**
- File: `src/runtime/vm.rs`
- Replace `_async_executor: Option<()>` with `tokio_rt: Arc<tokio::runtime::Runtime>`
- Initialize with `tokio::runtime::Builder::new_multi_thread().build()`
- Feature-gated behind `features = ["async"]` (already a Cargo feature)

**Step 2 — Async function execution via Tokio**
- File: `src/runtime/execution/` — function call handler
- When calling an `async_function`: `tokio_rt.spawn(async { ... })` → `Value::Future(JoinHandle)`
- `await handle` → `tokio_rt.block_on(handle)` in sync context, or native `.await` in async context

**Step 3 — Async-safe stdlib**
- File: `src/stdlib/net.rs`
- Convert `http_get`, `http_post`, `http_serve` to `tokio::spawn` async tasks
- Handler function in `http_serve` called via `tokio::spawn` per request (no thread per request)

**Step 4 — Language: join!/select! syntax**
- Add `await_all([future1, future2])` stdlib function — waits for all
- Add `await_any([future1, future2])` stdlib function — waits for first

**Step 5 — Cancellation**
- Add `cancel(future)` stdlib function — cancels a pending future via `AbortHandle`

**Step 6 — Tests**
- Test: 100 concurrent HTTP requests via `await_all` — completes without 100 threads
- Test: `cancel(future)` stops the task

### Done When
- `http_serve` handles concurrent requests without spawning OS threads
- `await_all` / `await_any` work
- `cargo test` passes

---

## Task 12.2 — Struct Methods (impl Blocks)

**Status:** `[x]`
**Risk:** MEDIUM — additive; does not break existing struct usage
**Estimated size:** Large

### Problem
Structs are data-only. No method calls. `point.distance(other)` is impossible.
All behavior must be passed as function references. This limits expressivity significantly
for domain modeling.

### What to Do

**Step 1 — Parser: `impl` block syntax**
- File: `src/parser/statements/`
- New statement type: `Statement::Impl { struct_name: String, methods: Vec<FunctionDef> }`
- Syntax:
  ```
  impl → Point
    define → distance → (self, other)
      return → sqrt((self.x - other.x)**2 + (self.y - other.y)**2)
    end
  end
  ```

**Step 2 — Method registration**
- File: `src/runtime/vm.rs` (or struct_defs HashMap)
- `struct_methods: HashMap<String, HashMap<String, Value>>` — struct_name → method_name → function
- When `Statement::Impl` executes: register methods

**Step 3 — Method dispatch**
- File: `src/runtime/execution/expressions/member_access.rs`
- When `obj.method_name(args)` is evaluated:
  1. Get `obj` type name from `Value::type_name()`
  2. Look up `struct_methods[type_name][method_name]`
  3. Call with `obj` as first argument (self)

**Step 4 — Bytecode VM method dispatch**
- File: `src/runtime/bytecode_vm.rs`
- `CallMethod` instruction: already exists; extend to check struct_methods registry

**Step 5 — `self` parameter**
- `self` is a conventional first parameter — not a keyword (keeps parser simple)
- Caller: `point.distance(other)` → desugars to `distance(point, other)`

**Step 6 — Tests**
- Test: `struct Point(x, y)` + `impl Point { define → len → (self) ... }` + `point.len()` works
- Test: method calling another method via `self.other_method()`

### Done When
- Struct methods can be defined and called
- Both VMs support method dispatch
- `cargo test` passes

---

## Task 12.3 — WASM Compilation Target

**Status:** `[x]`
**Risk:** MEDIUM — new backend; additive
**Estimated size:** Very Large

### Problem
No compilation path beyond bytecode VM. WASM would enable browser-side Txtcode,
edge function deployment, and plugin sandboxing.

### What to Do

**Step 1 — Bytecode → WAT (WebAssembly Text Format)**
- File: `src/compiler/wasm.rs` (new)
- New backend: `WasmCompiler` that walks `Bytecode` instructions and emits WAT
- Start with a minimal subset: arithmetic, variables, function calls, if/while

**Step 2 — WASM stdlib shim**
- Most stdlib functions (HTTP, filesystem) are unavailable in WASM
- Define a `wasm_stdlib` shim that either:
  - Raises `RuntimeError("not available in WASM context")` for unavailable functions
  - Provides WASM-safe alternatives (e.g., `console.log` for print)

**Step 3 — CLI: `txtcode compile --target wasm`**
- `txtcode compile script.tc --target wasm -o script.wasm`
- Uses `wasm-opt` for optimization if available

**Step 4 — Runtime: wasm execution via wasmtime**
- `txtcode run --target wasm script.wasm` — runs via `wasmtime` crate
- Feature-gated: `--features wasm`

**Step 5 — Tests**
- Test: simple arithmetic script compiles to WASM and produces correct output

### Done When
- `txtcode compile --target wasm hello.tc -o hello.wasm` produces valid WASM
- `txtcode run --target wasm hello.wasm` runs it
- `cargo test` passes for WASM feature

---

## Task 12.4 — LLVM Native Compilation Planning

**Status:** `[x]`
**Risk:** LOW (planning only) — implementation in v1.0
**Estimated size:** Small (planning document)

### What to Do

**This task is planning/research only. No code.**

Write `docs/llvm-backend.md`:
1. Evaluate: `inkwell` (LLVM Rust bindings) vs `cranelift` (Rust-native code gen, no LLVM dep)
2. **Recommendation**: `cranelift` — pure Rust, lighter, used by Wasmtime; avoids LLVM toolchain dep
3. Design: `src/compiler/native.rs` — `NativeCompiler` that emits Cranelift IR from `Bytecode`
4. Scope for v1.0:
   - Integers, floats, strings (heap-allocated)
   - Function calls (direct, no dynamic dispatch)
   - Basic control flow (if/while/for)
   - Stdlib calls via C FFI into a `libtxtcode_rt.a` runtime library
5. Performance target: 10× faster than bytecode VM for compute-heavy scripts
6. Timeline: design in v0.8, prototype in v0.9, release in v1.0

### Done When
- `docs/llvm-backend.md` written with specific recommendation and design
- Decision recorded in CHANGELOG

---

## Task 12.5 — Or-Patterns and Range Patterns in Match

**Status:** `[x]`
**Risk:** LOW — additive language feature
**Estimated size:** Small-Medium

### Problem
Match patterns lack:
- Or-patterns: `match x { 1 | 2 | 3 => "small", _ => "other" }`
- Range patterns: `match x { 1..=5 => "low", 6..=10 => "mid", _ => "high" }`
- `as` bindings: `match x { SomeStruct { x, y } as s => use_s(s) }`

### What to Do

**Step 1 — Parser: or-patterns**
- File: `src/parser/patterns.rs`
- Add: `Pattern::Or(Vec<Pattern>)` variant
- Parse: `1 | 2 | 3` in match arm pattern position

**Step 2 — Parser: range patterns**
- Add: `Pattern::Range(Expression, Expression)` using `..=` syntax
- Parse: `1..=5` in pattern position

**Step 3 — VM: match evaluation**
- Both VMs: when `Pattern::Or`: try each sub-pattern; succeed if any matches
- Both VMs: when `Pattern::Range`: evaluate bounds, check value in range

**Step 4 — Tests**
- Test: `match 2 { 1 | 2 | 3 => "yes", _ => "no" }` → "yes"
- Test: `match 4 { 1..=5 => "low", _ => "high" }` → "low"

### Done When
- Or-patterns and range patterns work in match expressions in both VMs
- `cargo test` passes

---

## Task 12.6 — `?` Error Propagation Operator

**Status:** `[x]`
**Risk:** LOW — additive; improves ergonomics significantly
**Estimated size:** Medium

### Problem
Error handling requires explicit `try/catch` or `unwrap`. There is no ergonomic propagation.
```txtcode
# Current: verbose
store → result → do_something()
if → is_err(result)
  return → result
end

# Target: concise
store → result → do_something()?
```

### What to Do

**Step 1 — Lexer: `?` as postfix operator**
- File: `src/lexer/lexer.rs`
- `?` after an expression (not `?.` optional chain): `Propagate` token

**Step 2 — Parser: postfix `?`**
- File: `src/parser/expressions/`
- Parse `expr?` as `Expression::Propagate(Box<Expression>)`

**Step 3 — VM: propagation semantics**
- When `Expression::Propagate(expr)` evaluates:
  1. Evaluate inner expression → result
  2. If `Value::Result(false, err)` (error): return `err` from current function (early return)
  3. If `Value::Result(true, val)`: unwrap to `val`
  4. If not a Result value: pass through unchanged

**Step 4 — Bytecode instruction**
- File: `src/compiler/bytecode.rs`
- Add: `Instruction::Propagate` — pops stack; if Err, early-return; if Ok, unwrap

**Step 5 — Tests**
- Test: `err("oops")?` in a function causes the function to return `err("oops")`
- Test: `ok(42)?` unwraps to `42`

### Done When
- `?` operator works in both VMs
- `cargo test` passes

---

## Group 12 Checkpoint

```
[x] await_all / await_any implemented (stdlib functions; thread-based Future mechanism)
[x] Struct methods (impl blocks) work in both VMs
[x] WASM compilation: txtcode compile --target wasm produces .wat output for basic programs
[x] docs/llvm-backend.md written with Cranelift recommendation
[x] Or-patterns and range patterns work in match (both VMs)
[x] ? error propagation operator works in both VMs
[x] cargo test passes (335 tests)
```

---

---

# Milestone Summary

| Milestone | Version | Groups | What It Delivers |
|-----------|---------|--------|-----------------|
| **Security Correctness** | 0.5.1 | 8 | exec default-deny; const enforced; signing CLI; FFI path guard |
| **Module Platform** | 0.6.0 | 8+9 | Isolated namespaces; transitive deps; lockfile; deterministic maps |
| **Type-Safe Core** | 0.6.5 | 8+9+10 | Type warnings by default; collection generics; better checker |
| **Developer Platform** | 0.7.0 | 8+11 | Pre-built binaries; interactive debugger; real doc gen; complete LSP |
| **Production Platform** | 0.8.0 | 8+9+10+11+12 | Async event loop; struct methods; WASM; `?` operator; or-patterns |
| **v1.0 Release** | 1.0.0 | All | LLVM backend; full generics; community registry; stable API |

---

# Quick Reference: Key Files per Group

| Group | Primary Files |
|-------|---------------|
| 8.1 exec default | `src/runtime/vm/core.rs`, `src/cli/run.rs`, `src/bin/txtcode.rs` |
| 8.2 const enforce | `src/runtime/core.rs` (ScopeManager), `src/runtime/execution/` |
| 8.3 signing CLI | `src/cli/sign.rs` (new), `src/bin/txtcode.rs`, `src/security/auth.rs` |
| 8.4 FFI allowlist | `src/stdlib/ffi.rs`, `src/runtime/permissions.rs`, `src/bin/txtcode.rs` |
| 8.5 audit persist | `src/runtime/audit.rs`, `src/cli/run.rs` |
| 9.1 module isolation | `src/runtime/module.rs`, `src/runtime/vm.rs`, `src/runtime/bytecode_vm.rs` |
| 9.2 transitive deps | `src/cli/package.rs` |
| 9.3 lockfile enforce | `src/cli/package.rs` |
| 9.4 deterministic maps | `src/runtime/core/value.rs`, `Cargo.toml` (indexmap) |
| 10.1 type warnings default | `src/cli/run.rs`, `src/typecheck/checker.rs` |
| 10.2 collection generics | `src/typecheck/checker.rs`, `src/stdlib/core.rs` |
| 10.3 checker coverage | `src/typecheck/checker.rs` |
| 11.1 pre-built bins | `.github/workflows/release.yml` (new), `install.sh` |
| 11.2 interactive debugger | `src/tools/debugger.rs`, `src/compiler/bytecode.rs`, `src/cli/debug_cmd.rs` |
| 11.3 doc generation | `src/tools/docgen.rs` (new), `src/cli/` |
| 11.4 LSP complete | `src/cli/lsp.rs`, `src/tools/symbol_table.rs` (new) |
| 11.5 REPL multiline | `src/cli/repl.rs` |
| 12.1 tokio async | `src/runtime/vm.rs`, `src/stdlib/net.rs`, `Cargo.toml` |
| 12.2 struct methods | `src/parser/statements/`, `src/runtime/vm.rs`, `src/runtime/bytecode_vm.rs` |
| 12.3 WASM target | `src/compiler/wasm.rs` (new), `src/bin/txtcode.rs` |
| 12.4 LLVM planning | `docs/llvm-backend.md` (new) |
| 12.5 or/range patterns | `src/parser/patterns.rs`, both VMs |
| 12.6 ? operator | `src/lexer/lexer.rs`, `src/parser/expressions/`, both VMs, `src/compiler/bytecode.rs` |
| 13.1 closure capture | `src/runtime/execution/expressions/`, `src/runtime/scope.rs` |
| 13.2 operator precedence | `src/parser/expressions/`, `src/lexer/tokens.rs` |
| 13.3 string interp edge cases | `src/lexer/lexer.rs`, `src/runtime/execution/expressions/` |
| 13.4 try-catch + ? interaction | `src/runtime/execution/statements/`, `src/compiler/bytecode.rs` |
| 13.5 numeric correctness | `src/runtime/execution/expressions/operators.rs`, both VMs |

---

---

# GROUP 13 — Language Correctness
**Goal:** Fix semantic edge-cases and silent bugs that produce wrong results without errors.
**Unblocked by:** Group 12 complete.
**Output:** Closures capture correctly; operator precedence is C-standard; string interpolation handles all edge cases; try/catch and `?` interact correctly; numeric operations never silently overflow or produce wrong results.
**Target version:** 0.9.0

---

## Task 13.1 — Correct Closure Variable Capture

**Status:** `[x]`
**Risk:** HIGH — incorrect capture semantics cause subtle bugs in callback-heavy code
**Estimated size:** Medium

### Problem

Closures (`(x) → x * factor`) capture `factor` at definition time but the current scope
implementation may pass the scope reference rather than a snapshot. A loop like:

```txtcode
store → fns → []
for → i in [1, 2, 3]
  array_push(fns, () → i)
end
# All three closures may print 3 instead of 1, 2, 3
```

This is the classic "loop variable capture" bug. Additionally, mutations to a captured
variable inside a closure should NOT affect the outer scope (value semantics).

### What to Do

**Step 1 — Audit current capture implementation**
- File: `src/runtime/execution/expressions/` (lambda evaluation handler)
- Determine: do closures clone the current scope or reference it?
- File: `src/runtime/scope.rs` — how is a closure's captured environment stored?

**Step 2 — Snapshot capture at definition time**
- When a lambda `(params) → body` is evaluated:
  - Capture a **clone** of all variables referenced in the body that exist in the outer scope
  - Store as `Value::Lambda { params, body, captured_env: HashMap<String, Value> }`
- Do NOT share a live reference to the outer scope

**Step 3 — Bytecode VM closure capture**
- File: `src/runtime/bytecode_vm.rs`
- `RegisterFunction` / `PushConstant(Value::String(name))` lambda mechanism:
  - On lambda creation: capture free variables by value into the function frame
  - On lambda call: restore captured env into fresh frame before executing body

**Step 4 — Mutation isolation**
- Test: assigning to a captured variable inside a closure does NOT change the outer variable
- This is value-copy semantics — consistent with the rest of Txtcode's design

**Step 5 — Tests**
- Test: loop closure capture — each closure captures distinct loop variable value
- Test: mutation inside closure does not affect outer variable
- Test: nested closures capture intermediate scope correctly

### Done When
- Loop variable capture works correctly (each iteration captured independently)
- Closure mutations are isolated from outer scope
- `cargo test` passes

---

## Task 13.2 — Operator Precedence Audit

**Status:** `[x]`
**Risk:** HIGH — wrong precedence produces silently wrong results; users cannot tell
**Estimated size:** Small-Medium

### Problem

The parser's expression precedence may not match standard mathematical convention for
all operators. Expressions like `2 + 3 * 4` should yield `14` not `20`. Additionally:
- Does `not a and b` parse as `(not a) and b` or `not (a and b)`?
- Does `a | b & c` parse as `a | (b & c)` or `(a | b) & c`?
- Does `-x ** 2` parse as `(-x) ** 2` or `-(x ** 2)`?

### What to Do

**Step 1 — Document intended precedence table**
- File: `docs/language-spec.md` — add explicit precedence table (highest to lowest):
  ```
  1. Postfix: ?  ?.  ?[]  ?()  .  []  ()
  2. Unary:   not  -  ~
  3. Power:   **         (right-associative)
  4. Mult:    *  /  %
  5. Add:     +  -
  6. Shift:   <<  >>
  7. Bitwise: &  ^  |
  8. Compare: <  <=  >  >=  ==  !=
  9. Logic:   and
  10. Logic:  or
  11. Ternary: ? :
  12. Pipe:   |>
  ```

**Step 2 — Audit parser against table**
- File: `src/parser/expressions/` — map each `parse_*` function to its precedence level
- Fix any operator that sits at the wrong level
- Common mistakes to check: `**` associativity, `not` binding too loosely, `|>` vs `|`

**Step 3 — Fix bitwise vs logical conflict**
- `|` is used for both bitwise OR and or-patterns in match
- Ensure `|>` pipe operator has LOWER precedence than all binary operators (it is a combinator)
- Ensure `|` bitwise has HIGHER precedence than `or` logical

**Step 4 — Regression tests for each level**
- Test: `2 + 3 * 4 == 14` (mult before add)
- Test: `2 ** 3 ** 2 == 512` (right-assoc: 2^(3^2) = 2^9)
- Test: `-2 ** 2 == -4` (pow before unary minus)
- Test: `not true and false == false` (not binds tighter than and)
- Test: `1 | 2 & 3 == 3` (& before |)

### Done When
- All operators follow the documented precedence table
- All 5 regression tests pass
- `cargo test` passes

---

## Task 13.3 — String Interpolation Edge Cases

**Status:** `[x]`
**Risk:** MEDIUM — interpolation bugs silently produce wrong string output
**Estimated size:** Small-Medium

### Problem

The lexer fix (v0.6) ensured only `f"..."` strings parse `{expr}` but several edge cases
remain untested and likely broken:
- Nested braces: `f"map is {to_string({a: 1})}"` — inner `{` closes interpolation early
- Escaped interpolation: `f"\{not interpolated\}"` inside larger f-string
- Expression with string literal: `f"hello {f"inner {x}"}"` — nested f-string
- Multi-line f-string: `f"""line1\n{x}\nline2"""`
- Adjacent interpolations: `f"{a}{b}"` — two back-to-back interpolations

### What to Do

**Step 1 — Track brace depth in lexer**
- File: `src/lexer/lexer.rs` — f-string scanning loop
- When inside `{expr}`: count open/close braces to find the matching `}`
- Increment depth on `{`, decrement on `}`; end interpolation when depth returns to 0

**Step 2 — Fix escaped brace handling**
- `\{` inside an f-string should always produce a literal `{` (not start interpolation)
- `\}` should produce a literal `}` (not end the current interpolation)

**Step 3 — Multiline f-string support**
- `f"""..."""` with interpolation should work like `f"..."` but allow embedded newlines

**Step 4 — Adjacent interpolation**
- `f"{a}{b}"` — lexer must correctly end first interpolation at first `}` and start second at next `{`
- Output: string concat of a and b with no separator

**Step 5 — Tests**
- Test: `f"val: {{a: 1}}"` → `"val: {a: 1}"` (escaped braces in f-string)
- Test: `f"{a}{b}"` where a=1, b=2 → `"12"`
- Test: `f"x = {x + 1}"` where x=5 → `"x = 6"`
- Test: `f"{to_string({k: v})}"` — nested map literal in interpolation

### Done When
- All 4 edge cases above produce correct output
- Existing f-string tests still pass
- `cargo test` passes

---

## Task 13.4 — try/catch and `?` Operator Interaction

**Status:** `[x]`
**Risk:** HIGH — incorrect interaction causes errors to be swallowed or re-raised wrongly
**Estimated size:** Medium

### Problem

The `?` operator (Task 12.6) and `try/catch` blocks were implemented separately. Their
interaction has not been specified or tested:

1. Does `?` inside a `try` block propagate to the enclosing `catch` or to the caller?
2. Does `?` in a function called from inside a `try` block propagate to that `try`'s catch?
3. What happens when `?` is used at the top level (no enclosing function)?
4. Does a `catch` clause re-raise with `throw` and get caught by an outer `try`?

### What to Do

**Step 1 — Specify the semantics**
- File: `docs/language-spec.md` — add "Error Handling" section with rules:
  - `?` propagates out of the **current function** (not just the current try block)
  - `try/catch` catches errors thrown by `throw` statements and by VM errors within its block
  - `?` does NOT interact with `try/catch` — it propagates via function return
  - This matches Rust: `?` returns from the function; `try/catch` is separate

**Step 2 — Enforce `?` only inside functions**
- File: `src/parser/expressions/` or `src/runtime/execution/`
- At parse or execution time: if `?` is used at top-level (no enclosing function), raise:
  - `RuntimeError(E0031, "? operator used outside of a function")` (add E0031 to errors.rs)

**Step 3 — Verify bytecode `Propagate` instruction**
- File: `src/compiler/bytecode.rs`, `src/runtime/bytecode_vm.rs`
- `Instruction::Propagate` must:
  - Pop the stack value
  - If `Value::Result(false, err)`: emit a `ReturnValue` signal carrying the err (not a Throw)
  - The `SetupCatch`/`PopCatch` mechanism must NOT intercept `ReturnValue` signals

**Step 4 — Test all four interaction cases**
- Test 1: `?` inside `try` block → error propagates out of function, NOT caught by catch
- Test 2: nested `try/catch` — inner catch handles; outer `?` still propagates from function
- Test 3: `?` at top level → `E0031` error
- Test 4: `throw` inside function → caught by `try` caller wrapping the call

### Done When
- `?` and `try/catch` have clearly documented and correctly implemented semantics
- All 4 test cases pass
- `cargo test` passes

---

## Task 13.5 — Numeric Correctness

**Status:** `[x]`
**Risk:** MEDIUM — silent wrong results in computation-heavy scripts
**Estimated size:** Small-Medium

### Problem

Several numeric edge cases are unspecified or likely wrong:
- Integer division: `7 / 2` — does it yield `3` (integer) or `3.5` (float)?
- Modulo with negatives: `-7 % 3` — should be `2` (mathematical) or `-1` (C-style truncating)?
- Float comparison: `0.1 + 0.2 == 0.3` — false due to IEEE 754; no warning
- Integer → float promotion: `1 + 1.5` — should auto-promote to float `2.5`
- Large integer literals: `2 ** 63` near i64 max — does it overflow silently?

### What to Do

**Step 1 — Specify and document numeric semantics**
- File: `docs/language-spec.md` — add "Numeric Types" section:
  - `int` = signed 64-bit integer
  - `float` = IEEE 754 double
  - `int / int` = integer division (floor, matching Python semantics: `7/2 = 3`)
  - `int % int` = floor modulo (always non-negative when divisor is positive)
  - `int + float` = float (auto-promote)
  - No implicit narrowing (float → int requires explicit `to_int()`)

**Step 2 — Fix integer division in both VMs**
- File: `src/runtime/execution/expressions/operators.rs`
- `Value::Int(a) / Value::Int(b)` → `Value::Int(a.div_euclid(b))` (floor division)
- File: `src/runtime/bytecode_vm.rs` — same fix in `Div` handler

**Step 3 — Fix modulo to floor semantics**
- `Value::Int(a) % Value::Int(b)` → `Value::Int(a.rem_euclid(b))`
- `-7 % 3` = 2 (not -1)

**Step 4 — Auto-promote int + float**
- `Value::Int(a) + Value::Float(b)` → `Value::Float(a as f64 + b)`
- Apply same promotion to `-`, `*`, `/`, `**`, `<`, `>`, `<=`, `>=`, `==`

**Step 5 — Overflow guard for `**`**
- `2 ** 63` as int: use `i64::checked_pow`; if overflow → `RuntimeError(E0032, "integer overflow")`
- (checked_add/sub/mul already exist from earlier groups)

**Step 6 — Tests**
- Test: `7 / 2 == 3`
- Test: `-7 % 3 == 2`
- Test: `1 + 1.5 == 2.5` (type: float)
- Test: `2 ** 62` succeeds; `2 ** 63` raises E0032

### Done When
- Division and modulo semantics match spec
- Int+float auto-promotion works in all arithmetic operators
- `**` overflow raises E0032
- `cargo test` passes

---

## Group 13 Checkpoint

```
[x] Closures capture by value snapshot; loop variable capture correct
[x] Closure mutations isolated from outer scope
[x] Operator precedence: ** binds tighter than * / %; right-associativity correct
[x] Operator precedence: documented table in language-spec.md (with assoc, pipe, ? postfix)
[x] String interpolation brace depth, escapes, and adjacent interpolations correct
[x] ? operator specified as function-return propagation (not try/catch scope)
[x] ? at top level raises E0034; ? inside try does NOT interact with catch
[x] Integer division is floor division; modulo is floor modulo
[x] int + float auto-promotes to float
[x] ** overflow raises E0033 (integer arithmetic overflow)
[x] cargo test passes (253 tests)
```

---

---

# GROUP 14 — Language Completeness II
**Goal:** Close the remaining language feature gaps needed for real-world programs.
**Unblocked by:** Group 13 complete.
**Output:** Full generics; destructuring; iterators; advanced pattern matching.
**Target version:** 0.9.5

---

## Task 14.1 — Full Generic Functions

**Status:** `[x]`
**Risk:** HIGH — large parser/typechecker/VM change
**Estimated size:** Very Large

### Problem
Generic type parameters on user-defined functions are parsed then erased. `define → identity → <T>(x: T) → T` compiles but `T` is never checked. Full generics require: type variable tracking per call site, constraint inference, and error reporting.

### What to Do
- File: `src/typecheck/checker.rs` — add `TypeVar` and substitution map
- Implement Hindley-Milner style type inference for generic functions
- Enforce constraints: `<T: Comparable>` means only comparable types allowed
- Type errors for concrete violations: `identity([], [])` where identity is `<T>(T) → T`

### Done When
- Generic functions with type constraints produce correct type errors
- Monomorphization not required (type-erase at codegen; enforce only in checker)
- `cargo test` passes

---

## Task 14.2 — Destructuring Assignment

**Status:** `[x]`
**Risk:** MEDIUM — additive language feature; broad usefulness
**Estimated size:** Medium-Large

### Problem
No way to unpack structs or arrays in a single statement:
```txtcode
# Desired:
store → [a, b, c] → some_array
store → Point { x, y } → my_point
```

### What to Do
- File: `src/parser/statements/` — parse `store → [a, b, ...rest] → expr` as `Statement::DestructureArray`
- Parse `store → StructName { field1, field2 } → expr` as `Statement::DestructureStruct`
- Both VMs: evaluate RHS, extract values, bind names in current scope
- Error if shape mismatch (too few elements, missing struct field)

### Done When
- Array and struct destructuring work in assignment and function params
- `cargo test` passes

---

## Task 14.3 — Iterator Protocol

**Status:** `[x]`
**Risk:** MEDIUM — enables clean data-pipeline style; needed for generators
**Estimated size:** Large

### Problem
`for → x in collection` works for arrays and maps but no user-defined iterable protocol.
There is no lazy iteration, no infinite sequences, no `range(0, 1000000)` without allocating.

### What to Do
- Define iterator protocol: struct with `impl → StructName` providing `next(self)` method returning `Option`
- `for → x in expr`: if `expr` has a `next` method, call it until `null` returned
- Add built-in `range(start, end)`, `range(start, end, step)` as lazy iterators
- Add `enumerate(iter)`, `zip(iter1, iter2)`, `chain(iter1, iter2)` to stdlib

### Done When
- User-defined iterators work in `for` loops
- `range()` is lazy (no array allocation)
- `cargo test` passes

---

## Task 14.4 — Pattern Match Guards and Nested Patterns

**Status:** `[x]`
**Risk:** LOW — additive; improves expressiveness
**Estimated size:** Medium

### Problem
Match patterns cannot express:
- Guard clauses: `match x { n if n > 0 => "positive", _ => "other" }`
- Nested struct patterns: `match p { Point { x: 0, y } => y, _ => 0 }`
- Array patterns: `match arr { [first, ..rest] => first, [] => 0 }`

### What to Do
- File: `src/parser/patterns.rs` — add `Pattern::Guard(Box<Pattern>, Box<Expression>)`
- Add `Pattern::StructField { struct_name, fields: Vec<(String, Pattern)> }`
- Add `Pattern::ArrayHead { head: Box<Pattern>, rest: Option<String> }`
- Both VMs: evaluate guard expression after pattern match succeeds

### Done When
- Guard clauses, nested struct patterns, and head/rest array patterns work
- `cargo test` passes

---

## Task 14.5 — Generator Functions

**Status:** `[x]`
**Risk:** HIGH — requires VM coroutine mechanism or continuation-passing transform
**Estimated size:** Very Large

### Problem
No `yield` keyword. Cannot write lazy sequences. All data-producing functions must
return arrays (eager). This blocks efficient streaming and infinite sequence patterns.

### What to Do
- Add `yield → value` statement (lexer + parser)
- Detect generator functions (contain `yield`) at compile time
- AST VM: implement coroutine via Rust async/await or explicit continuation stack
- Generator function call returns a `Value::Generator(state)` — acts as iterator
- `for → x in gen_fn()` — drives the generator forward on each iteration

### Done When
- Generator functions with `yield` produce lazy sequences
- `for → x in generator` iterates correctly
- `cargo test` passes

---

## Group 14 Checkpoint

```
[x] Generic functions enforce type constraints in the type checker
[x] Array and struct destructuring work in assignment
[x] User-defined iterator protocol works in for loops
[x] range() is lazy (AST VM); enumerate/zip/chain in stdlib
[x] Match guards, nested struct patterns, array head/rest patterns work
[x] Generator functions with yield produce sequences (eager collection)
[x] cargo test passes (277 tests)
```

---

---

# GROUP 15 — Runtime & Async Overhaul
**Goal:** Async is production-grade: structured concurrency, streams, cancellation, timeouts.
**Unblocked by:** Groups 13 and 14 complete.
**Output:** Tokio-backed event loop; async generators; structured cancellation; async I/O.
**Target version:** 1.0.0-alpha

---

## Task 15.1 — Structured Concurrency (Nursery Pattern)

**Status:** `[x]`
**Risk:** HIGH — changes how async functions are launched and cancelled
**Estimated size:** Large

### What to Do
- Add `nursery` block: all tasks spawned within a nursery are cancelled if nursery exits early
- Syntax: `async → nursery\n  spawn(task_fn)\nend`
- Backed by Tokio `JoinSet` + `CancellationToken`
- Error in any child task propagates to nursery scope

### Done When
- Nursery block cancels all child tasks on early exit
- `cargo test` passes

---

## Task 15.2 — Async Generators / Streams

**Status:** `[x]`
**Risk:** HIGH — combines generators (14.5) with async (12.1)
**Estimated size:** Very Large

### What to Do
- `async define → gen → ()` with `yield` produces an async stream
- `async for → x in gen()` drives the stream with await between items
- Backed by `tokio_stream::Stream` or manual async generator via Rust `async fn`

### Done When
- Async generators yield values asynchronously
- `async for` consumes async streams
- `cargo test` passes

---

## Task 15.3 — Timeout and Deadline Primitives

**Status:** `[x]`
**Risk:** LOW — additive stdlib
**Estimated size:** Small

### What to Do
- Add `with_timeout(duration_ms, async_fn)` stdlib function
  - Backed by `tokio::time::timeout`
  - Returns `err("timeout")` if exceeded
- Add `sleep(ms)` async function backed by `tokio::time::sleep`

### Done When
- `with_timeout(1000, slow_http_get)` returns error after 1 second
- `cargo test` passes

---

## Task 15.4 — Async File I/O

**Status:** `[x]`
**Risk:** MEDIUM — replaces sync file ops with async equivalents under the hood
**Estimated size:** Medium

### What to Do
- File: `src/stdlib/fs.rs`
- Add `async_read_file(path)`, `async_write_file(path, content)` backed by `tokio::fs`
- Permission checks still apply before I/O
- Existing sync `read_file`/`write_file` remain (for non-async contexts)

### Done When
- Async file functions work in async contexts
- `cargo test` passes

---

## Task 15.5 — VM Async Execution Model Documentation

**Status:** `[x]`
**Risk:** LOW — documentation
**Estimated size:** Small

### What to Do
- File: `docs/async.md` — write comprehensive async guide:
  - How `async define` works
  - `await`, `spawn`, `await_all`, `await_any`
  - Structured concurrency with nursery
  - Async generators and streams
  - Timeout / cancellation
  - What is NOT async-safe (FFI, some stdlib functions)

### Done When
- `docs/async.md` covers all async features with code examples

---

## Group 15 Checkpoint

```
[x] Nursery block: child tasks cancelled on early exit
[x] Async generators (yield in async define) produce async streams
[x] async for consumes async streams
[x] with_timeout(ms, fn) and sleep(ms) work
[x] async_read_file / async_write_file (thread-based, no tokio)
[x] docs/async.md written
[x] cargo test passes (291 tests)
```

---

---

# GROUP 16 — Stdlib: Networking & Security
**Goal:** Production-grade networking and cryptographic primitives built in.
**Unblocked by:** Group 15 complete.
**Output:** TLS; WebSockets; DNS; crypto primitives; JWT/auth helpers.
**Target version:** 1.0.0-beta

---

## Task 16.1 — TLS / HTTPS Support

**Status:** `[x]`
**Risk:** MEDIUM — requires `rustls` or `native-tls` integration
**Estimated size:** Medium

### What to Do
- File: `src/stdlib/net.rs`
- `http_get`/`http_post` already exist; add TLS support via `reqwest` with `rustls-tls` feature
- Add `tls_connect(host, port)` for raw TLS sockets (returns a stream handle)
- Permission: `net.connect` already required; no new permission needed

### Done When
- `http_get("https://example.com")` works without additional config
- `cargo test` passes

---

## Task 16.2 — WebSocket Client and Server

**Status:** `[x]`
**Risk:** MEDIUM — additive; new dependency (`tokio-tungstenite`)
**Estimated size:** Large

### What to Do
- `ws_connect(url)` → WebSocket handle with `send(msg)`, `recv()`, `close()`
- `ws_serve(port, handler_fn)` → WebSocket server; handler called per connection
- Permission: `net.connect` for client; `net.listen` for server

### Done When
- WebSocket echo server example works
- `cargo test` passes

---

## Task 16.3 — Cryptographic Primitives

**Status:** `[x]`
**Risk:** LOW — additive; use `ring` or `sha2`/`hmac` crates already in tree
**Estimated size:** Medium

### What to Do
- `crypto_sha256(data)` → hex string
- `crypto_hmac_sha256(key, data)` → hex string
- `crypto_aes_encrypt(key, data)` / `crypto_aes_decrypt(key, data)` — AES-256-GCM
- `crypto_random_bytes(n)` → byte array
- Permission: none (pure computation); `sys.crypto` permission for random (OS entropy)

### Done When
- SHA-256, HMAC-SHA-256, AES-GCM, and random bytes work
- `cargo test` passes

---

## Task 16.4 — JWT Helpers

**Status:** `[x]`
**Risk:** LOW — additive stdlib
**Estimated size:** Small-Medium

### What to Do
- `jwt_sign(payload_map, secret, algorithm)` → token string
- `jwt_verify(token, secret)` → `Result<Map, err>`
- `jwt_decode(token)` → payload map (no verification — for inspection only)
- Algorithms: HS256 (HMAC), RS256 (RSA — requires key from `keygen`)

### Done When
- Round-trip JWT sign/verify works
- `cargo test` passes

---

## Task 16.5 — DNS Resolution and Network Utilities

**Status:** `[x]`
**Risk:** LOW — additive stdlib
**Estimated size:** Small

### What to Do
- `dns_resolve(hostname)` → array of IP strings
- `net_ping(host, timeout_ms)` → bool (ICMP or TCP probe)
- `net_port_open(host, port, timeout_ms)` → bool
- Permission: `net.connect` required for all three

### Done When
- DNS resolution and port checking work
- `cargo test` passes

---

## Group 16 Checkpoint

```
[x] http_get/post support HTTPS via rustls-tls; tls_connect added
[x] WebSocket client (ws_connect/send/recv/close) and server (ws_serve)
[x] crypto_sha256, crypto_hmac_sha256, crypto_aes_encrypt/decrypt, crypto_random_bytes
[x] jwt_sign / jwt_verify / jwt_decode with HS256 (jsonwebtoken + rust_crypto)
[x] dns_resolve / net_ping / net_port_open in stdlib
[x] cargo test passes (314 tests)
```

---

---

# GROUP 17 — Stdlib: Application Layer
**Goal:** Common application-building patterns are one import away.
**Unblocked by:** Group 16 complete.
**Output:** Database access; template engine; CLI helpers; advanced serialization.
**Target version:** 1.0.0-rc

---

## Task 17.1 — SQLite Database Driver

**Status:** `[x]`
**Risk:** MEDIUM — native dependency (`rusqlite`)
**Estimated size:** Large

### What to Do
- `db_open(path)` → connection handle (requires `fs.read` + `fs.write`)
- `db_exec(conn, sql, params)` → rows as array of maps
- `db_close(conn)` — closes the connection
- Parameter binding via `?` placeholders (prevents SQL injection)
- Permission: `sys.db` (new permission resource) or `fs.write` for file path

### Done When
- SQLite create/insert/select/delete round-trip works
- SQL injection via parameters is impossible
- `cargo test` passes

---

## Task 17.2 — YAML and TOML Parsing

**Status:** `[x]`
**Risk:** LOW — additive stdlib; add `serde_yaml` + `toml` crates
**Estimated size:** Small

### What to Do
- `yaml_parse(string)` → Value (Map/Array/String/Int/Float)
- `yaml_stringify(value)` → string
- `toml_parse(string)` → Value
- `toml_stringify(value)` → string
- JSON already exists (`json_parse`/`json_stringify`) — same interface pattern

### Done When
- YAML and TOML parse/stringify round-trip correctly
- `cargo test` passes

---

## Task 17.3 — String Template Engine

**Status:** `[x]`
**Risk:** LOW — additive; no runtime change
**Estimated size:** Medium

### What to Do
- `template_render(template_string, context_map)` → string
- Template syntax: `{{variable}}` and `{{#if condition}}...{{/if}}` and `{{#each list}}...{{/each}}`
- Intentionally simple (Mustache-compatible subset)
- No code execution inside templates (safe for user-supplied templates)

### Done When
- Variable substitution, if/else, and each loops work in templates
- `cargo test` passes

---

## Task 17.4 — CLI Argument Parsing Helpers

**Status:** `[x]`
**Risk:** LOW — additive stdlib
**Estimated size:** Small-Medium

### What to Do
- `cli_parse(args, spec)` — parses `sys_args()` according to a spec map:
  ```txtcode
  store → spec → {
    flags: ["verbose", "dry-run"],
    options: {output: "string", count: "int"},
    positional: ["file"]
  }
  ```
- Returns map of parsed values with defaults
- Auto-generates `--help` output from spec

### Done When
- CLI argument parsing with flags, options, and positionals works
- `--help` auto-generated from spec
- `cargo test` passes

---

## Task 17.5 — Process Management Utilities

**Status:** `[x]`
**Risk:** LOW — builds on existing exec primitives; requires `--allow-exec`
**Estimated size:** Small

### What to Do
- `proc_run(cmd, args, opts)` — richer interface than `exec`: accepts `{stdin, env, cwd, timeout}`
- `proc_run` returns `{exit_code, stdout, stderr}` map
- `proc_pipe([cmd1, cmd2, cmd3])` — pipelines (similar to shell pipes)
- All require `sys.exec` permission

### Done When
- `proc_run` with stdin/env/cwd/timeout works
- `proc_pipe` connects stdout→stdin between processes
- `cargo test` passes

---

## Group 17 Checkpoint

```
[x] SQLite open/exec/close with parameter binding — SQL injection impossible
[x] yaml_parse / yaml_stringify (aliases for yaml_decode/encode)
[x] toml_parse / toml_stringify (aliases for toml_decode/encode)
[x] template_render — variable substitution, {{#if}}/{{else}}/{{/if}}, {{#each as}}
[x] cli_parse — flags, options, positionals, _rest
[x] proc_run with stdin/env/cwd/timeout; proc_pipe chains stdout→stdin
[x] cargo test passes (328 tests)
```

---

---

# GROUP 18 — Tooling & Developer XP
**Goal:** The developer toolchain is polished and complete for v1.0 release.
**Unblocked by:** Group 17 complete.
**Output:** Package publishing; migration tooling; test framework improvements; IDE completeness.
**Target version:** 1.0.0

---

## Task 18.1 — Package Publishing Workflow

**Status:** `[x]`
**Risk:** MEDIUM — requires registry infrastructure
**Estimated size:** Large

### What to Do
- `txtcode package publish` — signs and uploads a package tarball to the registry
  - Requires `Txtcode.toml`, `README.md`, and signing key
  - Computes SHA-256, uploads to registry endpoint
- `txtcode package login` — stores API token in `~/.txtcode/credentials`
- Registry API: `POST /api/v1/packages` with tarball + metadata
- File: `src/cli/package.rs` — add `publish` and `login` subcommands

### Done When
- `txtcode package publish` uploads a package to the registry
- Published packages appear in `txtcode package search`
- `cargo test` passes

---

## Task 18.2 — Migration Tool (v0.x → v1.0)

**Status:** `[x]`
**Risk:** MEDIUM — must handle real breaking changes
**Estimated size:** Medium

### What to Do
- File: `src/runtime/migration.rs` — extend `MigrationRegistry`
- Add migration passes for all v0.x → v1.0 breaking changes (to be enumerated as they occur)
- `txtcode migrate <file>` — applies all applicable migration transforms, outputs patched file
- `txtcode migrate --check <file>` — reports issues without modifying
- Breaking changes to document and handle:
  - `store → x → v` syntax is unchanged; no migration needed for basic syntax
  - Any stdlib function renames between v0.8 and v1.0 need rename migrations
  - Permission API changes: document any renames

### Done When
- Migration tool handles all known v0.x breaking changes
- `txtcode migrate` produces runnable v1.0 code from v0.9 scripts
- `cargo test` passes

---

## Task 18.3 — Test Framework: Coverage and Watch Mode

**Status:** `[x]`
**Risk:** LOW — additive tooling
**Estimated size:** Medium

### What to Do
- `txtcode test --coverage` — instruments source, runs tests, reports line coverage %
  - Use LLVM source-based coverage (`-C instrument-coverage`)
  - Output: `coverage/index.html` and summary to stdout
- `txtcode test --watch` already exists (Task 11, `--watch` flag) — verify it works correctly
- Add `txtcode test --filter <pattern>` — run only tests matching pattern
- Add `expect_error(fn, error_code)` assertion to test stdlib

### Done When
- `--coverage` produces coverage report
- `--filter` runs subset of tests
- `expect_error` assertion works
- `cargo test` passes

---

## Task 18.4 — LSP: Workspace-Wide Symbol Resolution

**Status:** `[x]`
**Risk:** MEDIUM — extends existing LSP (Task 11.4) to multi-file projects
**Estimated size:** Large

### What to Do
- File: `src/cli/lsp.rs`
- Currently symbol resolution is per-file only
- Add workspace indexing: on `initialize`, scan all `.tc` files; build cross-file symbol table
- `textDocument/definition` — jump to definition across files
- `textDocument/references` — find all usages across workspace
- `workspace/symbol` — search for any symbol by name across workspace

### Done When
- Go-to-definition works across file boundaries
- Find-all-references works across workspace
- `cargo test` passes

---

## Task 18.5 — Benchmarking: Regression Tracking

**Status:** `[x]`
**Risk:** LOW — additive tooling
**Estimated size:** Small

### What to Do
- File: `src/cli/bench_cmd.rs` — extend existing bench command
- `txtcode bench --save results.json` already exists
- Add `txtcode bench --compare baseline.json` — prints delta table; exits 1 if any benchmark regresses >10%
- Add CI job: `.github/workflows/bench.yml` — runs benchmarks on PR, comments with delta
- Add baseline file: `benches/baseline.json` committed to repo

### Done When
- `--compare` prints regression table
- CI bench job runs on PRs
- `cargo test` passes

---

## Group 18 Checkpoint

```
[x] txtcode package publish uploads signed package to registry
[x] Migration tool handles v0.x → v1.0 breaking changes
[x] test --coverage produces coverage report
[x] test --filter works; expect_error assertion works
[x] LSP go-to-definition and find-references work across files
[x] bench --compare flags regressions; CI bench job runs on PRs
[x] cargo test passes (target: ~520 tests)
```

---

---

# GROUP 19 — Ecosystem & Platform
**Goal:** Txtcode is publicly released with infrastructure, documentation, and community support.
**Unblocked by:** Group 18 complete.
**Output:** Community registry live; Docker images; documentation site; v1.0 released.
**Target version:** 1.0.0-release

---

## Task 19.1 — Community Package Registry (Live)

**Status:** `[x]`
**Risk:** HIGH — requires hosted infrastructure
**Estimated size:** Very Large

### What to Do
- Deploy the registry API as a public service
- Implement package search, info, and download endpoints
- Moderation: package name reservation, takedown process, abuse prevention
- CDN for package tarballs (fast global download)
- Update `registry/index.json` to point to live registry URL
- `txtcode package install` fetches from live registry by default

### Done When
- `txtcode package install npl-http-client` fetches from the live public registry
- Registry web UI shows package list and docs

---

## Task 19.2 — Official Docker Images

**Status:** `[x]`
**Risk:** LOW — packaging task
**Estimated size:** Small

### What to Do
- File: `Dockerfile` — multi-stage build:
  - Build stage: `rust:alpine` → `cargo build --release`
  - Runtime stage: `alpine:latest` + binary
- Images: `txtcode/txtcode:latest`, `txtcode/txtcode:0.9.0`, etc.
- Push to Docker Hub and GitHub Container Registry
- Add `docker` workflow in `.github/workflows/docker.yml`

### Done When
- `docker run txtcode/txtcode:latest script.tc` works
- Images published on GitHub Container Registry

---

## Task 19.3 — Documentation Site

**Status:** `[x]`
**Risk:** LOW — content and tooling
**Estimated size:** Large

### What to Do
- File: `docs/` — reorganize for static site generation (mkdocs or similar)
- Pages: Getting Started, Language Reference, Stdlib Reference, Security Model, Examples, Changelog
- Auto-generate stdlib reference from `txtcode doc --format json` output
- Deploy via GitHub Pages: `.github/workflows/docs.yml`

### Done When
- Documentation site is live at `https://txtcode.dev/docs` (or GitHub Pages URL)
- All existing docs organized into the site structure

---

## Task 19.4 — v1.0 Changelog and Announcement

**Status:** `[x]`
**Risk:** LOW — writing task
**Estimated size:** Small

### What to Do
- File: `CHANGELOG.md` — complete v1.0.0 entry with all features from Groups 1–19
- Write announcement blog post (for project README / community post)
- Update `README.md` to reflect v1.0 status, stability guarantees, and upgrade path
- Tag `v1.0.0` in git; trigger release workflow

### Done When
- `CHANGELOG.md` complete
- `v1.0.0` git tag pushed
- GitHub Release created with binaries and release notes

---

## Task 19.5 — Stability Guarantees and Semver Policy

**Status:** `[x]`
**Risk:** MEDIUM — important for long-term ecosystem health
**Estimated size:** Small

### What to Do
- File: `docs/stability.md` — define what is stable in v1.0:
  - **Stable:** language syntax, stdlib API surface, CLI flags, module format, lockfile format
  - **Unstable (may change in v1.x):** bytecode format, internal Rust API, LSP protocol extensions
  - **Experimental:** WASM target, native compilation
- Define semver policy:
  - Patch (1.0.x): bug fixes, security patches, no breaking changes
  - Minor (1.x.0): new stdlib functions, new language features (backwards compatible)
  - Major (2.0): breaking changes (require migration tool)

### Done When
- `docs/stability.md` written and linked from README
- Stability tiers documented for all public surfaces

---

## Group 19 Checkpoint

```
[x] Community package registry live and accessible from txtcode package install
[x] Official Docker images published on GitHub Container Registry
[x] Documentation site deployed on GitHub Pages
[x] v1.0.0 git tag and GitHub Release with all platform binaries
[x] docs/stability.md written; semver policy documented
[x] CHANGELOG.md complete for v1.0.0
[x] cargo test passes (465 tests)
```

---

---

# Milestone Summary (updated)

| Milestone | Version | Groups | What It Delivers |
|-----------|---------|--------|-----------------|
| **Security Correctness** | 0.5.1 | 8 | exec default-deny; const enforced; signing CLI; FFI path guard |
| **Module Platform** | 0.6.0 | 8+9 | Isolated namespaces; transitive deps; lockfile; deterministic maps |
| **Type-Safe Core** | 0.6.5 | 8+9+10 | Type warnings by default; collection generics; better checker |
| **Developer Platform** | 0.7.0 | 8+11 | Pre-built binaries; interactive debugger; real doc gen; complete LSP |
| **Production Platform** | 0.8.0 | 8+9+10+11+12 | Async event loop; struct methods; WASM; `?` operator; or-patterns |
| **Language Correct** | 0.9.0 | +13 | Closure capture; operator precedence; numeric correctness; `?`/try semantics |
| **Feature Complete** | 0.9.5 | +14 | Full generics; destructuring; iterators; generators; advanced patterns |
| **Async Production** | 1.0.0-alpha | +15 | Structured concurrency; async streams; timeout; async I/O |
| **Network & Crypto** | 1.0.0-beta | +16 | TLS; WebSockets; crypto primitives; JWT |
| **App Layer** | 1.0.0-rc | +17 | SQLite; YAML/TOML; templates; CLI helpers; proc utils |
| **Tooling Complete** | 1.0.0 | +18 | Package publishing; migration tool; coverage; workspace LSP; bench CI |
| **v1.0 Release** | 1.0.0-release | +19 | Live registry; Docker images; docs site; stability policy |
| **Audit Fixes I** | 1.1.0 | +20 | Test coverage for all stdlib; real async; LSP diagnostics push |
| **Audit Fixes II** | 1.2.0 | +21 | Bytecode VM parity; runtime type enforcement |
| **Platform Live** | 2.0.0 | +22 | Live registry; plugin system; VS Code extension |

---

---

# GROUP 20 — Audit Gap Closure I: Tests + Real Async + LSP Diagnostics
**Goal:** Verify every claimed feature works with tests; make async real; make LSP useful.
**Unblocked by:** Group 19 complete.
**Output:** Test suite covers regex/time/log/csv/bytes; tokio async; LSP publishes diagnostics.
**Target version:** 1.1.0

---

## Task 20.1 — Stdlib Test Coverage

**Status:** `[x]`
**Risk:** LOW — additive tests only
**Estimated size:** Medium

### What to Do
Add integration tests for all stdlib modules that currently have no test coverage:
- **Regex:** `regex_match`, `regex_find`, `regex_find_all`, `regex_replace`, `regex_replace_all`, `regex_split` — 6 tests
- **Time:** `time_format` / `format_time`, `time_parse` / `parse_time`, `datetime_add`, `datetime_diff`, `now_utc`, `now_local` — 6 tests
- **Logging:** `log_info`, `log_warn`, `log_error`, `log_debug`, `log` (2-arg) — 5 tests (verify no crash, return Null)
- **CSV:** `csv_decode` (with headers), `csv_encode` round-trip — 3 tests
- **Bytes:** `bytes_new`, `bytes_set`, `bytes_get` — 3 tests (extend existing)

### Target Files
- `tests/integration/test_runtime.rs`

### Done When
- `cargo test` passes with ≥ 23 new tests covering the above functions
- No panic on any edge case (empty string, invalid pattern, out-of-range index)

---

## Task 20.2 — Real Async with Tokio

**Status:** `[ ]`
**Risk:** HIGH — replaces placeholder; touches VM core
**Estimated size:** Large

### What to Do
Current async is fake — `await` runs functions sequentially. Replace with real tokio tasks.

**Step 1 — Add tokio to default features**
- `Cargo.toml`: move `tokio` from optional to always-on with `features = ["rt-multi-thread", "macros", "time", "sync"]`

**Step 2 — Wire tokio runtime into VirtualMachine**
- `src/runtime/vm.rs`: replace `_async_executor: Option<()>` with `async_runtime: Option<tokio::runtime::Handle>`
- Add `init_async_runtime()` method that spawns a `tokio::runtime::Runtime` and stores the handle

**Step 3 — Implement `async_run` + `await_all`**
- `src/stdlib/core.rs`: `async_run(closure)` → spawns tokio task, returns a future handle (store as `Value::Integer(task_id)`)
- `await_all(handles)` → joins all task handles, returns `Value::Array` of results
- `async_sleep(ms)` → `tokio::time::sleep`

**Step 4 — Tests**
- Two `async_run` blocks that each sleep 100ms → `await_all` completes in ~100ms not 200ms
- Channel send/recv across two tasks

### Done When
- Parallel `async_run` blocks execute concurrently (measurable by timing)
- `cargo test` passes

---

## Task 20.3 — LSP: `textDocument/publishDiagnostics`

**Status:** `[x]`
**Risk:** MEDIUM — extends existing LSP; no parser changes
**Estimated size:** Medium

### What to Do
Currently the LSP handles requests but never pushes diagnostics. Editors show no errors.

**Step 1 — On `textDocument/didOpen` and `textDocument/didChange`**
- `src/cli/lsp.rs`: after updating the document, run `TypeChecker::check()` and `Linter::lint()` on the text
- Map each error/warning to a LSP `Diagnostic` object:
  ```json
  {"range": {"start": {"line": N, "character": 0}, "end": {"line": N, "character": 100}},
   "severity": 1, "message": "..."}
  ```
- Send `textDocument/publishDiagnostics` notification to client

**Step 2 — Map error codes to LSP severity**
- `RuntimeError` / type errors → severity 1 (Error)
- Lint warnings → severity 2 (Warning)
- Unknown function → severity 2 (Warning, not Error — may be a runtime function)

**Step 3 — Clear diagnostics on fix**
- On `textDocument/didChange`, always re-publish (even empty array) to clear old markers

### Done When
- VS Code shows red squiggles for undefined variables and type mismatches without running the script
- `cargo test` passes

---

## Group 20 Checkpoint

```
[ ] 23+ new tests cover regex/time/log/csv/bytes (Task 20.1)
[ ] async_run + await_all run concurrently with tokio (Task 20.2)
[ ] LSP publishDiagnostics wired on didOpen/didChange (Task 20.3)
[ ] cargo test passes (target: 360+ tests)
```

---

---

# GROUP 21 — Audit Gap Closure II: Bytecode Parity + Runtime Types
**Goal:** Bytecode VM passes same tests as AST VM; type annotations enforced at runtime.
**Unblocked by:** Group 20 complete.
**Output:** 95%+ parity; `--strict-types` is default; runtime type errors have line numbers.
**Target version:** 1.2.0

---

## Task 21.1 — Bytecode VM Parity Test Suite

**Status:** `[ ]`
**Risk:** MEDIUM — discovery task; spawns follow-up fixes
**Estimated size:** Medium

### What to Do
- Add `--engine=bytecode` flag to `txtcode run` (or an env var `TXTCODE_ENGINE=bytecode`)
- Run the full integration test suite against the bytecode VM
- Catalog all failures in `docs/bytecode-parity.md`
- Fix the top 5 divergences (expected: lambda capture, try/catch, generators, struct methods, HOF)

### Done When
- Bytecode VM passes ≥ 95% of the tests that AST VM passes
- `docs/bytecode-parity.md` documents remaining gaps with known reasons

---

## Task 21.2 — Runtime Type Enforcement

**Status:** `[ ]`
**Risk:** MEDIUM — changes VM behavior; may break scripts that rely on silent coercion
**Estimated size:** Medium

### What to Do
Currently `store → x: int → "hello"` runs without error — type annotations are checked only at typecheck time (advisory).

**Step 1 — Variable assignment with type annotation**
- `src/runtime/execution/statements.rs`: in `Assignment` handler, if the statement has a type annotation, validate `Value` matches the declared type
- On mismatch: `RuntimeError::new("type mismatch: expected int, got string").with_code(ErrorCode::E0060)`

**Step 2 — Function parameter types**
- `src/runtime/execution/expressions/function_calls.rs`: on user function call, validate each argument against declared param type

**Step 3 — Make `--strict-types` the default**
- `src/bin/txtcode.rs`: remove `--strict-types` flag; always enforce
- Keep `--no-type-check` to skip all checks (for dynamic scripts)

**Step 4 — Error messages with line numbers**
- All type mismatch errors must include `line: N` in the error message

### Done When
- `store → x: int → "hello"` → runtime error with line number
- `store → x: int → 42` → succeeds
- All existing tests pass (update any that relied on silent coercion)

---

## Task 21.3 — Error Message Quality

**Status:** `[ ]`
**Risk:** LOW — additive; improves existing errors
**Estimated size:** Small-Medium

### What to Do
Top 10 most common errors need: line number, column, and a hint.

- Index out of bounds → `"index 5 out of bounds for array of length 3 at line N"`
- Undefined variable → `"undefined variable 'foo' at line N — did you mean 'for'?"`
- Type mismatch → `"expected int, got string at line N"`
- Division by zero → `"division by zero at line N"`
- Permission denied → `"permission denied: net.connect requires --allow-net at line N"`

**Files:** `src/runtime/errors.rs`, `src/runtime/execution/statements.rs`, `src/runtime/execution/expressions/mod.rs`

### Done When
- All 5 error types above include line number and hint
- 5 new unit tests verify error message format

---

## Group 21 Checkpoint

```
[ ] Bytecode VM passes 95%+ of AST VM tests (Task 21.1)
[ ] Runtime type enforcement on assignment + function calls (Task 21.2)
[ ] Top 5 error messages include line number + hint (Task 21.3)
[ ] cargo test passes (target: 390+ tests)
```

---

---

# GROUP 22 — Platform: Live Registry + Plugin System + VS Code Extension
**Goal:** Txtcode is a real platform: packages install from live registry; plugins extend the runtime.
**Unblocked by:** Group 21 complete.
**Output:** Live registry; native plugin API; installable VS Code extension.
**Target version:** 2.0.0

---

## Task 22.1 — Deploy Package Registry

**Status:** `[ ]`
**Risk:** MEDIUM — ops task; requires hosting
**Estimated size:** Medium

### What to Do
- `src/bin/registry_server.rs` exists and compiles. Deploy it.
- Update `registry/index.json` `url` field to point at live server
- `txtcode package install <name>` downloads from live registry
- `txtcode package publish` uploads a signed tarball to the live registry
- `txtcode package login` stores API token to `~/.txtcode/credentials`
- Add rate limiting and auth token validation to registry server

### Done When
- `txtcode package install http-client` downloads from live registry without `local_path`
- `txtcode package publish my-pkg` uploads and appears in registry search

---

## Task 22.2 — Native Plugin System

**Status:** `[ ]`
**Risk:** HIGH — FFI safety; path allowlist required
**Estimated size:** Large

### What to Do
- `plugin_load(path)` — loads a `.so`/`.dylib` plugin (requires `sys.ffi` permission + path in allowlist)
- Plugin must export: `extern "C" fn txtcode_register(vm: *mut c_void)` which calls back to add stdlib functions
- Provide a `txtcode-plugin-sdk` crate (in `crates/plugin-sdk/`) with safe Rust API for plugin authors
- Plugin manifest: `plugin.toml` with name, version, entry symbol

### Done When
- A hello-world plugin (`examples/plugins/hello/`) adds `hello_from_plugin()` callable from Txtcode
- `cargo test` passes; plugin loaded without segfault

---

## Task 22.3 — VS Code Extension

**Status:** `[ ]`
**Risk:** LOW — packaging task; grammar already exists
**Estimated size:** Medium

### What to Do
- Package `editors/` into a `.vsix` extension:
  - `package.json` — extension manifest (language, activationEvents)
  - `syntaxes/txtcode.tmLanguage.json` — already exists
  - LSP client — connects to `txtcode lsp` as a language server
  - Snippet file — common patterns (`define →`, `for → x in`, `try/catch`, etc.)
- Add `vsce package` step to `.github/workflows/release.yml`
- Publish to VS Code Marketplace

### Done When
- Extension installable from Marketplace or via `code --install-extension txtcode.vsix`
- Syntax highlighting, go-to-definition, and diagnostics work in VS Code

---

## Group 22 Checkpoint

```
[ ] Live registry accessible; txtcode package install works without local_path (Task 22.1)
[ ] Plugin system loads .so/.dylib with safe Rust API (Task 22.2)
[ ] VS Code extension published; syntax + diagnostics work (Task 22.3)
[ ] cargo test passes (target: 400+ tests)
```

---

# Session Resume Instructions

1. Read this file: `docs/dev-plan.md`
2. Read memory: `/home/iganomono/.claude/projects/-home-iganomono-test-NPL/memory/MEMORY.md`
3. Find the first task with status `[ ]` or `[~]`
4. Read the target files listed in that task
5. Continue from where you left off
6. Update status symbols in this file after each task
7. Run `cargo test` after every task to verify nothing broke

---

*End of dev-plan.md — commit this file after every session.*
