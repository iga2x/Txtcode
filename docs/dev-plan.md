# Txtcode Development Plan
**Version:** 0.5.0 → 1.0.0
**Last Updated:** 2026-03-19
**Status Legend:** `[ ]` todo · `[~]` in progress · `[x]` done · `[!]` blocked

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
Group 1: Foundation Stability          [x] COMPLETE (244 tests passing)
Group 2: Language Completeness         [x] COMPLETE (261 tests passing)
Group 3: Type Enforcement              [x] COMPLETE (179 tests passing)
Group 4: Async Runtime                 [x] COMPLETE (179 tests passing)
Group 5: Stdlib Gaps                   [x] COMPLETE (194 tests passing)
Group 6: Ecosystem                     [x] COMPLETE (194 tests passing)
Group 7: Performance Baseline          [x] COMPLETE (194 tests passing)
─────────────────────────────────────────────────────────────────────
Group 8: Security Correctness          [x] COMPLETE (202 tests passing)
Group 9: Module System Overhaul        [x] COMPLETE (209 tests passing)
─────────────────────────────────────────────────────────────────────
Group 10: Type System Promotion        [ ] NEXT — start here
Group 11: Developer Experience         [ ] unblocked (can parallel 10)
Group 12: Platform & Compilation       [ ] blocked by Groups 9+10
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

**Status:** `[ ]`
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

**Status:** `[ ]`
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

**Status:** `[ ]`
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
[ ] Default txtcode run shows type warnings without aborting
[ ] --strict-types aborts on type violations
[ ] --no-type-check silences type output
[ ] Array<T> / Map<K,V> annotations produce errors/warnings when violated
[ ] Return type checking implemented in type checker
[ ] Arity checking implemented in type checker
[ ] Null arithmetic warnings implemented
[ ] cargo test passes
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

**Status:** `[ ]`
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

**Status:** `[ ]`
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

**Status:** `[ ]`
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

**Status:** `[ ]`
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

**Status:** `[ ]`
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
[ ] Pre-built binaries on GitHub Releases for Linux x86_64, macOS arm64, Windows x64
[ ] install.sh installs from binary without requiring Rust toolchain
[ ] txtcode debug <file> enters interactive loop with break/step/print/continue/quit
[ ] Source line shown at each debugger break
[ ] txtcode doc generates markdown API docs from ## comments
[ ] LSP: go-to-definition works for same-file symbols
[ ] LSP: hover shows function signature
[ ] REPL: multiline input with continuation prompt
[ ] REPL: history persists across sessions
[ ] cargo test passes
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

**Status:** `[ ]`
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

**Status:** `[ ]`
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

**Status:** `[ ]`
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

**Status:** `[ ]`
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

**Status:** `[ ]`
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

**Status:** `[ ]`
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
[ ] Tokio async runtime integrated; http_serve handles concurrent requests
[ ] await_all / await_any / cancel implemented
[ ] Struct methods (impl blocks) work in both VMs
[ ] WASM compilation: txtcode compile --target wasm works for basic programs
[ ] docs/llvm-backend.md written with Cranelift recommendation
[ ] Or-patterns and range patterns work in match
[ ] ? error propagation operator works in both VMs
[ ] cargo test passes
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
