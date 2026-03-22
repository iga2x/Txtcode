# Txtcode Development Plan — v5.0 (100% Layer Completion Target)
**Version:** 5.0 (senior architect audit — inside-out layered plan, all layers to 100%)
**Last Updated:** 2026-03-22
**Status Legend:** `[ ]` todo · `[~]` in progress · `[x]` done · `[!]` blocked · `[-]` deferred

---

## WHAT THIS PLAN IS

This plan is built from a senior compiler-engineer audit targeting **100% completion on
every layer**. It replaces v4.0 (now archived). All completed groups (A–N) are preserved
as history. New groups (O–U) cover all remaining gaps.

**Inside-out development order — never advance past a blocked layer:**
```
Layer 1  Language Core       (Lexer · Grammar · Operators · Strings)
Layer 2  Type System         (Checker · Inference · Enforcement · Narrowing)
Layer 3  Parser + AST        (Completeness · Spans · Edge Cases · Recovery)
Layer 4  Execution Model     (AST VM · Bytecode · TCO · Closures)
Layer 5  Runtime System      (Memory · Modules · Async · Errors · Dispatch)
Layer 6  Standard Library    (Functions · I/O · Network · Crypto · DB)
Layer 7  Security Model      (Permissions · Sandbox · Audit · Supply Chain)
Layer 8  Tooling             (CLI · LSP · Formatter · Linter · Debugger)
Layer 9  Ecosystem           (Registry · Releases · Install · Playground · Docs)
```

---

## ⚠️ REMINDERS / OPEN NOTES

```
[RESOLVED 2026-03-21] tests/tc/*.tc — W.5 DONE: tests/integration/test_tc_files.rs wires
           all 4 .tc files into `cargo test` via subprocess (CARGO_BIN_EXE_txtcode +
           CARGO_MANIFEST_DIR for stable CWD).

[RESOLVED 2026-03-21] Milestone 1 language-core bugs — Group W ALL DONE:
             W.1 ✓  Integer division truncation (removed floor adjustment; Rust a/b)
             W.2 ✓  Optional chaining ?[ parse (lookahead for [ before ternary handler)
             W.3 ✓  Closure capture (snapshot_local_vars when is_in_local_scope)
             W.4 ✓  Dotted method definition (Type.method parse + register_struct_method)

[RESOLVED 2026-03-21] Type checker false positives — ALL FIXED:
             ✓ Enum variables: define_variable(name, Unknown) added after define_enum
             ✓ For-loop variable: register var as Unknown before checking body
             ✓ Nested fn return: register fn name before checking body
             ✓ Binary ops on Unknown: suppress false-positive errors in inference.rs
```

---

## CURRENT POSITION (750 tests · 2026-03-21)

```
Layer 1  Language Core       [x] 100%  — W bugs ✓; V.3 grammar.ebnf 9 fixes + 14 Rust tests ✓
Layer 2  Type System         [x] 100%  — null-narrowing ✓; struct fields ✓; E0029 ✓;
                                         elseif branches ✓; compound/index assign ✓; expr recursion ✓; struct fields ✓
Layer 3  Parser + AST        [x] 100%  — span tracking ✓; publishDiagnostics ✓ (5 tests); grammar.ebnf ✓
Layer 4  Execution Model     [x] 100%  — closure ✓; O.1 break/continue boundary ✓ (4 tests);
                                          O.5 bytecode parity ✓ + constant-fold optimizer ✓ (3 tests)
Layer 5  Runtime System      [x] 100%  — P.1 ✓ O(1) dispatch; S.3 ✓ RSS limits; P.2 ✓ Arc<str> O(1) clone; P.4 ✓ arg pool (P.3 CoW deferred)
Layer 6  Standard Library    [x] 100%  — db_transaction ✓; conn limit ✓; str_build ✓; R.3 audit tests ✓
Layer 7  Security Model      [~]  86%  — plugin unsandboxed; registry unsigned
Layer 8  Tooling             [~]  82%  — publishDiagnostics ✓; no DAP; formatter uncertified
Layer 9  Ecosystem           [-]  15%  — intentionally deferred

WEIGHTED TOTAL  98%  →  TARGET: 100% per layer

TESTS: 605 integration + 165 unit = 770 total (918 with --features bytecode)
```

---

## COMPLETED GROUPS (history — all passing)

```
Group A  Bug Fixes                  [x] COMPLETE — except A.7 db_transaction (→ Group R)
Group B  Dead Code Cleanup          [x] COMPLETE — AIMetadata removed; migration trimmed
Group C  Call Depth Fix             [x] COMPLETE — stacker::maybe_grow; MAX_CALL_DEPTH=500
Group D  Async Model                [x] COMPLETE — multi-worker pool; permission snapshot
Group E  Language Completeness      [x] COMPLETE — E.1 protocol; E.2 generics; E.3 errors;
                                                    E.4 parser recovery; E.5 TCO
Group F  Tooling Quality            [x] COMPLETE — F.1 formatter; F.2 10 lint rules;
                                                    F.3 LSP completions+signatureHelp; F.4 test assertions
Group G  Security Hardening         [x] COMPLETE — G.1 audit log; G.2 seccomp allowlist;
                                                    G.3 macOS sandbox_init(); G.4 Ed25519 key
Group H  WASM String Support        [x] COMPLETE — H.1 string constants in binary output (+5 tests)
Group I  Embed API Fix              [x] COMPLETE — I.1 eval_string/last_error_code; I.2 set_string_n
Group J  Version & Project Hygiene  [x] COMPLETE — version=3.0.0; deferred.md; archive/
Group K  Core Language Correctness  [x] COMPLETE — K.1 Type::Unknown; K.2 check_strict();
                                                    K.3 exhaustiveness; K.4 grammar.ebnf
Group L  Stdlib Completeness        [x] COMPLETE — L.1 http_serve (+5 tests); L.2 regex cache (+2);
                                                    L.3 plugin JSON ABI (+3)
Group M  Runtime Hardening          [x] COMPLETE — M.1 async back-pressure; M.2 GC rename;
                                                    M.3 migration trim
Group N  Language Edge Cases        [x] COMPLETE — N.1 Pattern::Literal; N.2 protocol compliance;
                                                    N.3 optional-chain typecheck; N.4 extended TCO;
                                                    N.5 E0012 modulo; N.6 rest pattern validation
```

---

## NEW GROUPS (v5.0 — targeting 100% per layer)

```
Group O  Runtime Architecture       [~] Layer 4+5 — ExecResult; span tracking ✓; module isolation ✓
Group P  Performance                [ ] Layer 4+5 — stdlib HashMap; string intern; value RC
Group Q  Type System Completion     [x] Layer 2   — null narrowing ✓; struct fields ✓; E0029 ✓
Group R  Stdlib Correctness         [x] Layer 6   — db_transaction ✓; pool ✓; audit ✓; str_build ✓
Group S  Security Completeness      [ ] Layer 7   — plugin sandbox; registry signing; real RSS
Group T  Tooling Completion         [ ] Layer 8   — LSP diagnostics; DAP; formatter cert; linter+
Group U  Ecosystem                  [ ] Layer 9   — registry; releases; install; playground; docs
Group V  Language Spec              [x] Layer 1+3 — Unicode escapes ✓; associativity ✓; grammar.ebnf 9 fixes ✓; 14 grammar tests ✓
Group W  Language Core Bug Fixes    [x] Layer 1+4 — W.1 truncation ✓; W.2 ?[ ✓; W.3 closures ✓;
                                                     W.4 dotted methods ✓; W.5 tc test wiring ✓
```

---

## MILESTONE PLAN (v5.0 — 100% targets)

### Milestone 1 — Language Core 100% (Layers 1–3)
**Groups V, Q (partial)**
Exit: `docs/grammar.ebnf` verified against parser; Unicode escapes work; operator
associativity tests pass; struct field types enforced; null narrowing active.

### Milestone 2 — Execution + Runtime 100% (Layers 4–5)
**Groups O, P**
Exit: ExecResult enum replaces control-flow signal hack; module sub-VMs have isolated
permissions; stdlib dispatch is O(1) HashMap; runtime errors include source line numbers;
per-task async timeouts work.

### Milestone 3 — Standard Library 100% (Layer 6)
**Group R**
Exit: `db_transaction()` auto-rollbacks; every stdlib function audited for stub/fake
returns; connection pool for db; string concat O(n²) linted.

### Milestone 4 — Security 100% (Layer 7)
**Group S**
Exit: Plugin libraries run under seccomp namespace; registry packages verified with
signed manifests; real RSS memory limits on Linux; Windows Job Object sandbox.

### Milestone 5 — Tooling 100% (Layer 8)
**Group T**
Exit: LSP publishes inline diagnostics; DAP debug adapter works in VS Code; formatter
passes idempotency on 30 programs; linter has 25+ rules; debugger supports conditional
breakpoints and bytecode VM.

### Milestone 6 — Ecosystem 100% (Layer 9)
**Group U**
Exit: One-line install (`curl https://txt.sh | sh`); public binary releases; web playground
live; community docs at docs.txtcode.dev; Windows CI passing.

---

# GROUP W — LANGUAGE CORE BUG FIXES
**Goal:** Fix real bugs found by running `tests/tc/*.tc` against the live binary.
**Layers:** 1 (Language Core) + 4 (Execution Model)
**Target:** v3.1.0
**Expected test delta:** +12 tests (4 `.rs` unit + 8 `.tc` integration via W.5)
**Priority:** MUST DO BEFORE Milestone 1 is declared complete.

---

## Task W.1 — Integer Division: Truncation Not Floor

**Status:** `[x]` DONE — arithmetic.rs: truncation (a / b)
**Priority:** HIGH — test_arithmetic.tc fails; contradicts all C-family language conventions
**Risk:** LOW — single arithmetic.rs change; add tests
**File:** `src/runtime/operators/arithmetic.rs`

### What is wrong

`-7 / 2` returns `-4` (floor division, Python-style) but the language spec and every
C-family language (C, Java, JavaScript, Rust) use truncation toward zero: `-7 / 2 = -3`.
The comment in the code even acknowledges "Floor division (Python-style)".

```
// Currently (WRONG for NPL):
-7 / 2  →  -4    (floor toward -∞)
 7 / -2 →  -4    (floor toward -∞)

// Should be (truncation toward zero):
-7 / 2  →  -3
 7 / -2 →  -3
```

### What to do

In `arithmetic.rs` `divide()` int/int branch, remove the floor adjustment:
```rust
// BEFORE:
let d = a / b;
let r = a % b;
let floor_d = if r != 0 && (*a < 0) != (*b < 0) { d - 1 } else { d };
Ok(Value::Integer(floor_d))

// AFTER (Rust's `/` already truncates toward zero):
Ok(Value::Integer(a / b))
```

**Done When:**
- `-7 / 2 == -3` ✓, `7 / -2 == -3` ✓, `-7 / -2 == 3` ✓
- `test_arithmetic.tc` passes fully
- 3 new unit tests for negative int division

---

## Task W.2 — Optional Chaining `?[` Conflicts with Ternary `?`

**Status:** `[x]` DONE — calls.rs: ?[ lookahead before ternary
**Priority:** HIGH — `obj?[key]` is completely broken; parse error
**Risk:** MEDIUM — parser lookahead change
**File:** `src/parser/parser.rs` (or wherever ternary `?` is parsed)

### What is wrong

The parser sees `m?["key"]` and interprets `?` as the start of a ternary
(`condition ? then : else`). It then expects `:` after `["key"]` but finds
a newline → parse error.

```
m?["key"]     →  Parse error: Expected Colon, got Newline
m?.key        →  Works correctly (optional member access)
```

### What to do

In the expression parser, after consuming `?`, look ahead one token:
- If next token is `[` → parse as `OptionalIndex(obj, key_expr)`
- If next token is `.` → parse as `OptionalMember(obj, field)` (already works)
- Otherwise → parse as ternary `condition ? then : else`

**Done When:**
- `m?["key"]` returns the value when key exists
- `m?["missing"]` returns `null` when key absent
- `arr?[0]` works on arrays
- 3 new integration tests

---

## Task W.3 — Closures Don't Capture Enclosing Scope

**Status:** `[x]` DONE — scope.rs + statements.rs: snapshot_local_vars capture
**Priority:** CRITICAL — closures are completely broken; any nested function referencing
                         outer variables fails with "undefined variable"
**Risk:** MEDIUM — changes `Statement::FunctionDef` execution; must not break top-level fns
**File:** `src/runtime/execution/statements.rs`

### What is wrong

`Statement::FunctionDef` always stores the function with an empty captured environment:
```rust
vm.set_global(
    name.clone(),
    Value::Function(name.clone(), params.clone(), body.clone(),
        HashMap::new(),  // ← ALWAYS EMPTY — outer scope never captured
    ),
)?;
```

When an inner function `adder` references outer variable `n`, the runtime finds nothing
in `captured_env` and then fails with `E0010 undefined variable 'n'`.

```
define → make_adder → (n)
  define → adder → (x)
    return → x + n   // n is NEVER captured → runtime error
  end
  return → adder
end
store → add5 → make_adder(5)
print → add5(3)   // Error: undefined variable 'n'
```

### What to do

**Step 1 — Detect if we are inside a function scope**

`VirtualMachine` has a scope stack. If `scope_depth() > 0` (inside a function body),
the `FunctionDef` is a nested/closure definition and must capture the current local scope.

**Step 2 — Snapshot current locals as captured_env**

```rust
// In statements.rs FunctionDef handler:
let captured_env = if vm.is_in_local_scope() {
    vm.snapshot_local_vars()  // returns HashMap<String, Value> of current locals
} else {
    HashMap::new()  // top-level fn: no capture needed
};

// Store with captured scope (use set_local, not set_global, for nested fns):
if vm.is_in_local_scope() {
    vm.set_variable(name.clone(),
        Value::Function(name.clone(), params.clone(), body.clone(), captured_env))?;
} else {
    vm.set_global(name.clone(),
        Value::Function(name.clone(), params.clone(), body.clone(), captured_env))?;
}
```

**Step 3 — Add `snapshot_local_vars()` to VirtualMachine**

Returns all variables in the current (innermost non-global) scope as a `HashMap<String, Value>`.

**Done When:**
- `make_adder(5)(3)` returns `8` ✓
- `make_adder(10)(3)` returns `13` ✓
- Nested closures (`adder` inside `outer`) work
- Top-level functions still work (no regression)
- 4 new integration tests: basic closure; closure mutation; multiple closures; top-level fn unchanged

---

## Task W.4 — Method Definition Syntax (`define → Type.method`) Parse Error

**Status:** `[x]` DONE — functions.rs: dotted name parse; register_struct_method
**Priority:** MEDIUM — OOP pattern completely unavailable
**Risk:** LOW — additive parser change
**File:** `src/parser/statements/functions.rs`

### What is wrong

Defining a method on a struct type fails to parse:
```
define → Point.to_string → (self)   // Error: Expected LeftParen, got Dot '.'
  return → "point"
end
```

The function name parser expects a plain identifier, not `Type.method`.

### What to do

In `parse_function_def()`, after reading the function name identifier, check if
the next token is `.` — if so, read the method name and store as `"Type.method"`:

```rust
let base_name = parser.expect_identifier()?;
let fn_name = if parser.check(TokenKind::Dot) {
    parser.advance(); // consume '.'
    let method = parser.expect_identifier()?;
    format!("{}.{}", base_name, method)
} else {
    base_name
};
```

The runtime already handles `"Type.method"` lookups via `call_struct_method`.

**Done When:**
- `define → Point.to_string → (self)` parses and executes
- `p.to_string()` dispatches to the method
- 2 new integration tests

---

## Task W.5 — Wire `tests/tc/*.tc` into `cargo test`

**Status:** `[x]` DONE — test_tc_files.rs: 4 subprocess tests; current_dir fix
**Priority:** HIGH — `.tc` tests exist but are orphaned from CI
**Risk:** NONE — additive test harness only
**File:** `tests/integration/test_tc_files.rs` (new)

### What is wrong

`tests/tc/test_arithmetic.tc`, `test_strings.tc`, `test_collections.tc`,
`test_functions.tc` exist and cover core language behavior — but `cargo test`
never runs them. They are only run manually via `txtcode test tests/tc/`.

### What to do

Create `tests/integration/test_tc_files.rs` with a helper that:
1. Builds the path to the compiled binary (`target/debug/txtcode`)
2. Spawns `txtcode run <file>` as a subprocess
3. Asserts exit code 0 and no `❌ ASSERTION FAILED` in output

```rust
fn run_tc_file(path: &str) {
    let binary = env!("CARGO_BIN_EXE_txtcode");
    let output = std::process::Command::new(binary)
        .args(["run", path])
        .output()
        .expect("failed to run txtcode");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(),
        "tc test failed: {}\nstderr: {}", path, stderr);
    assert!(!stderr.contains("ASSERTION FAILED"),
        "assertion failed in {}: {}", path, stderr);
}

#[test] fn test_tc_arithmetic()   { run_tc_file("tests/tc/test_arithmetic.tc"); }
#[test] fn test_tc_strings()      { run_tc_file("tests/tc/test_strings.tc"); }
#[test] fn test_tc_collections()  { run_tc_file("tests/tc/test_collections.tc"); }
#[test] fn test_tc_functions()    { run_tc_file("tests/tc/test_functions.tc"); }
```

Note: W.1 (int division) must be done before `test_tc_arithmetic` can pass.
Note: W.3 (closures) must be done before `test_tc_functions` can pass.

**Done When:**
- All 4 `.tc` test files pass via `cargo test`
- `tests/integration/mod.rs` includes `mod test_tc_files`
- 4 new tests added to CI

---

# GROUP O — RUNTIME ARCHITECTURE CORRECTNESS
**Goal:** Runtime is internally correct — no design debt that causes latent bugs.
**Layers:** 4 (Execution Model) + 5 (Runtime System)
**Target:** v4.1.0
**Expected test delta:** +8 tests

---

## Task O.1 — Separate Control Flow Signals from RuntimeError

**Status:** `[x]` DONE — break/continue boundary enforced in call_user_function (all 3 paths); 4 tests
**Priority:** HIGH — latent correctness bug at every unguarded call site
**Risk:** HIGH — touches every execution path in `src/runtime/execution/`
**Files:** `src/runtime/errors.rs`, `src/runtime/execution/statements.rs`,
          `src/runtime/execution/expressions/function_calls.rs`,
          `src/runtime/vm/core.rs`

### What is wrong

`RuntimeError` has a `signal: Option<ControlFlowSignal>` field. `return`, `break`,
and `continue` are encoded as `Err(RuntimeError { signal: Some(...) })`. Every
function call site must discriminate:

```rust
if let Some(signal) = err.take_signal() {
    match signal {
        ControlFlowSignal::Return(v) => return Ok(v),
        ControlFlowSignal::Break    => propagate_break!(),
        ControlFlowSignal::Continue => propagate_continue!(),
    }
}
```

There are ~30 such sites in `execution/`. Missing one check causes control flow to
escape function or loop boundaries silently. This has caused real bugs (break
propagating through function calls in generators).

### What to do

**Step 1 — Define ExecResult**

```rust
// src/runtime/errors.rs (new enum alongside RuntimeError)
pub enum ExecResult {
    Value(Value),
    Return(Value),   // return → expr
    Break,           // break
    Continue,        // continue
    Yield(Value),    // yield → expr
}
pub type StmtResult = Result<ExecResult, RuntimeError>;
```

**Step 2 — Change statement evaluation return type**

```rust
// BEFORE:
fn execute_statement(&mut self, stmt: &Statement) -> Result<Option<Value>, RuntimeError>

// AFTER:
fn execute_statement(&mut self, stmt: &Statement) -> StmtResult
```

**Step 3 — Update all call sites**

In `execute_block()` (the place that runs a sequence of statements):
```rust
fn execute_block(&mut self, stmts: &[Statement]) -> StmtResult {
    for stmt in stmts {
        match self.execute_statement(stmt)? {
            ExecResult::Value(_) => {}          // continue loop
            early @ ExecResult::Return(_)  => return Ok(early),
            early @ ExecResult::Break      => return Ok(early),
            early @ ExecResult::Continue   => return Ok(early),
            early @ ExecResult::Yield(_)   => return Ok(early),
        }
    }
    Ok(ExecResult::Value(Value::Null))
}
```

**Step 4 — Function call boundary**

In `call_user_function()`, unwrap Return to a Value; error on Break/Continue outside loop:
```rust
match self.execute_block(&body)? {
    ExecResult::Return(v) | ExecResult::Value(v) => Ok(v),
    ExecResult::Break | ExecResult::Continue =>
        Err(RuntimeError::new("break/continue outside loop").with_code(E0040)),
    ExecResult::Yield(v) => { /* generator handling */ Ok(v) }
}
```

**Done When:**
- `RuntimeError.signal` field removed
- All 30+ call sites updated to match on `ExecResult`
- `break` inside a function body correctly errors
- `return` inside a nested loop correctly returns from the function
- 4 tests: return-inside-loop; break-at-correct-level; continue-in-nested-loop;
  generator-yield-sequence

---

## Task O.2 — Module Sub-VM Permission Isolation

**Status:** `[x]` DONE — module sub-VM clones permission snapshot on import
**Priority:** HIGH — privilege escalation via module import
**Risk:** MEDIUM
**Files:** `src/runtime/module.rs`, `src/runtime/vm/core.rs`

### What is wrong

When `import → "package"` runs, a sub-VM executes the module file. The sub-VM receives
the parent VM's `PermissionManager` by shared reference. A malicious or buggy module
can call functions that invoke `vm.grant_permission(...)`, which modifies the **parent's**
grants — effectively allowing a module to escalate the importer's permissions.

Example attack vector:
```
// malicious_module.tc
grant_permission("sys.exec", null)   // grants exec to PARENT VM
```

### What to do

**Step 1 — Clone PermissionManager for sub-VM**

```rust
// In module.rs, when creating sub-VM for import:
let mut sub_vm = VirtualMachine::new();
// BEFORE (wrong):
sub_vm.permission_manager = self.permission_manager.clone_ref();
// AFTER (correct):
sub_vm.permission_manager = self.permission_manager.snapshot();
```

Add `snapshot()` to `PermissionManager`:
```rust
impl PermissionManager {
    /// Clone all grants and denials into a new independent manager.
    /// The sub-VM cannot affect the parent's grants.
    pub fn snapshot(&self) -> Self {
        Self {
            grants: self.grants.clone(),
            denials: self.denials.clone(),
        }
    }
}
```

**Step 2 — Export-only permission propagation**

After module executes, its exports are safe (values, not permissions). Do NOT copy
sub-VM's permission manager back to parent.

**Step 3 — Document in module system**

Add to module.rs doc comment:
```
/// Security note: each imported module runs with a SNAPSHOT of the
/// importer's permissions. Modules cannot grant or revoke permissions
/// for the importing scope. This is intentional and security-critical.
```

**Done When:**
- Module import creates an independent PermissionManager snapshot
- `grant_permission()` inside a module does not affect the importer
- 2 tests: module cannot escalate parent permissions; module can use its own permissions

---

## Task O.3 — Span Tracking Through Execution

**Status:** `[x]` DONE — span stored in thread-local ExecutionContext
**Priority:** HIGH — runtime errors have no source location
**Risk:** MEDIUM — requires threading span context through execution
**Files:** `src/runtime/execution/statements.rs`, `src/runtime/errors.rs`,
          `src/runtime/vm/core.rs`

### What is wrong

AST nodes carry `span: Span { line, column }`. But when execution reaches a stdlib
call or a runtime arithmetic operation, the span is not threaded through. Result:

```
Runtime error: division by zero [E0012]
```

instead of:

```
Runtime error at line 47, col 12: division by zero [E0012]
```

### What to do

**Step 1 — Add current_span to VirtualMachine**

```rust
// In vm.rs:
pub struct VirtualMachine {
    // existing fields ...
    current_span: Option<Span>,  // updated at each statement execution
}
```

**Step 2 — Set span at statement execution**

```rust
fn execute_statement(&mut self, stmt: &Statement) -> StmtResult {
    self.current_span = stmt.span;  // record before executing
    // ... existing dispatch
}
```

**Step 3 — Attach span to RuntimeError on creation**

```rust
impl VirtualMachine {
    fn runtime_error(&self, msg: impl Into<String>) -> RuntimeError {
        let mut err = RuntimeError::new(msg.into());
        if let Some(span) = self.current_span {
            err = err.with_span(span);
        }
        err
    }
}
```

**Step 4 — Display span in error messages**

```rust
impl RuntimeError {
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
    pub fn display(&self) -> String {
        match self.span {
            Some(s) => format!("line {}, col {}: {}", s.line, s.column, self.message),
            None    => self.message.clone(),
        }
    }
}
```

**Step 5 — Wire into panic display in CLI**

In `src/bin/txtcode.rs`, when printing runtime errors, call `.display()` instead of
`.to_string()` to get the location.

**Done When:**
- All runtime errors show `line N, col M:` prefix when span is available
- Division by zero shows line number
- Type mismatch shows line number
- 3 tests: span in division-by-zero; span in type-error; span in undefined-variable

---

## Task O.4 — Per-Task Async Timeout

**Status:** `[x]` DONE — async_run_timeout() stdlib fn; AbortHandle on exceed
**Priority:** MEDIUM — production stability
**Risk:** LOW
**Files:** `src/runtime/event_loop.rs`, `src/stdlib/mod.rs`

### What is wrong

`async_run(fn)` submits a task with no per-task timeout. 64 tasks can each run
an infinite loop — all workers occupied, program appears hung. The global `--timeout`
cancels the whole program but not individual tasks.

### What to do

**Step 1 — Add optional duration to async_run**

```
// New signature in Txt-code:
async_run(fn)                   // no timeout (existing)
async_run_timeout(fn, timeout_ms: int)  // new
```

**Step 2 — Implement in stdlib/mod.rs**

```rust
"async_run_timeout" => {
    let func   = args[0].clone();
    let millis = match &args[1] { Value::Integer(n) => *n as u64, _ => ... };
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel2 = Arc::clone(&cancel);
    // Spawn timeout thread
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(millis));
        cancel2.store(true, Ordering::Relaxed);
    });
    // Submit task with cancel flag
    event_loop::submit_with_cancel(func, cancel)
}
```

**Step 3 — Pass cancel flag to submitted sub-VM**

The worker's `VirtualMachine` already has `cancel_flag: Option<Arc<AtomicBool>>`.
Set it when submitting with timeout.

**Done When:**
- `async_run_timeout(fn, 1000)` cancels after 1 second
- Cancelled task raises `E0051` in the future result
- Regular `async_run` is unchanged
- 2 tests: task completes before timeout; task cancelled by timeout

---

## Task O.5 — VM Strategy Decision

**Status:** `[x]` DONE — bytecode VM at full parity (745/745 tests); constant-fold optimizer (3 tests)
**Priority:** HIGH — blocks all performance work; doubles maintenance burden
**Risk:** HIGH (strategic decision)
**Files:** `src/runtime/bytecode_vm.rs`, `src/compiler/bytecode.rs`,
          `src/compiler/optimizer.rs`

### What is wrong

Two execution engines exist with no migration plan:
- **AST VM:** default, full feature parity, ~3× slower than bytecode potential
- **Bytecode VM:** feature-gated, partial parity, optimizer is a stub

Every feature must be implemented twice. Every security patch must be applied twice.
The bytecode optimizer (zero passes implemented) is the only path to real performance
improvements, but it can't be used because the bytecode VM is not the default path.

### Decision Required

**Option A — Promote Bytecode VM (RECOMMENDED)**
```
v4.2: Feature-complete bytecode VM (all AST VM features ported)
v4.3: Bytecode VM becomes default execution path
v5.0: AST VM deprecated; removed in v6.0
```

**Option B — Consolidate on AST VM**
```
v4.2: Remove bytecode VM entirely
v4.3: Implement tree-walking optimizer on AST VM
v5.0: Consider JIT or LLVM IR backend
```

### Minimum tasks for Option A (RECOMMENDED)

**Step 1 — Audit bytecode VM feature gaps**

Run the full test suite against bytecode VM:
```bash
cargo test --features bytecode -- --nocapture 2>&1 | grep "FAILED"
```

List all failing tests. Each failure is a feature gap.

**Step 2 — Port all failing features to bytecode VM**

Expected gaps:
- `Optional chaining` (`?.`, `?[]`, `?()`)
- `Async/await` in bytecode execution context
- `Protocol` declaration and `implements` runtime check
- `Generator` function (`yield`) support
- `Pattern matching` with `Pattern::Literal`

**Step 3 — Implement constant folding optimizer**

```rust
// src/compiler/optimizer.rs
pub fn optimize(bytecode: &mut Bytecode) {
    constant_fold(bytecode);
    // Future: dead_code_elimination, inlining
}

fn constant_fold(bc: &mut Bytecode) {
    let mut i = 0;
    while i + 2 < bc.instructions.len() {
        match (&bc.instructions[i], &bc.instructions[i+1], &bc.instructions[i+2]) {
            (PushConstant(Value::Integer(a)),
             PushConstant(Value::Integer(b)),
             Add) => {
                bc.instructions.drain(i..i+3);
                bc.instructions.insert(i, PushConstant(Value::Integer(a + b)));
            }
            // ... other constant expressions
            _ => { i += 1; }
        }
    }
}
```

**Step 4 — Fix Lambda HOF string hack**

Currently bytecode lambdas are stored as `Value::String(fn_name)`. The bytecode VM
detects these strings and treats them as function references. This causes a bug:
any string that coincidentally matches a registered function name gets called as a function.

Fix: Add `Value::Lambda(u64)` variant (opaque ID) and update all HOF dispatch:
```rust
pub enum Value {
    // ... existing
    Lambda(u64),  // bytecode lambda ID — NOT a string
}
```

**Done When (Option A):**
- All 691 tests pass with `--features bytecode` (bytecode VM as execution path)
- Constant folding optimizer reduces instruction count in benchmark programs
- `Value::Lambda(id)` replaces string-based lambda hack
- `txtcode run --engine=bytecode` (or make it default after parity verified)
- 5 tests: optimizer reduces fib bytecode; lambda HOF correctness; protocol in bytecode;
  optional chain in bytecode; generator in bytecode

---

# GROUP P — PERFORMANCE
**Goal:** Hot execution paths are efficient enough for production scripts.
**Layers:** 4 (Execution) + 5 (Runtime)
**Target:** v4.2.0
**Expected test delta:** +4 tests (correctness, not benchmarks)

---

## Task P.1 — Stdlib Dispatch HashMap (O(n) → O(1))

**Status:** `[ ]`
**Priority:** HIGH — called on every stdlib function invocation
**Risk:** LOW — pure refactor, no behavior change
**File:** `src/stdlib/mod.rs`

### What is wrong

The stdlib dispatcher is a sequence of `if name == "..." || name == "..."` blocks.
With 110+ functions, every stdlib call performs up to ~100 string comparisons.
In a loop calling `len()` 1,000,000 times, that is 100,000,000 unnecessary string comparisons.

```rust
// Current (O(n) per call):
if name == "len" || name == "length" { CoreLib::... }
else if name == "str_upper" { CoreLib::... }
else if name == "str_lower" { CoreLib::... }
// ... 100+ more branches
```

### What to do

**Step 1 — Define function registry type**

```rust
// stdlib/registry.rs (new file)
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub type DispatchFn = fn(&str, &[Value], DispatchContext) -> Result<Value, RuntimeError>;

pub struct DispatchContext<'a> {
    pub executor: Option<&'a mut dyn FunctionExecutor>,
    pub permission_checker: Option<&'a dyn PermissionChecker>,
    pub seed_override: Option<u64>,
}

pub static STDLIB_REGISTRY: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Core functions
    m.insert("len",          "core");
    m.insert("length",       "core");
    m.insert("str_upper",    "core");
    m.insert("str_lower",    "core");
    m.insert("array_push",   "core");
    // ... all 110+ entries
    // Network
    m.insert("http_get",     "net");
    m.insert("http_serve",   "net");
    // Crypto
    m.insert("sha256",       "crypto");
    // DB
    m.insert("db_connect",   "db");
    m
});
```

**Step 2 — Replace the if-chain with a single lookup**

```rust
pub fn call_function_with_combined_traits(...) -> Result<Value, RuntimeError> {
    // O(1) lookup
    match STDLIB_REGISTRY.get(name) {
        Some(&"core")   => CoreLib::call_function(name, args, executor),
        Some(&"net")    => {
            #[cfg(feature = "net")] { NetLib::call_function(name, args, executor, pc) }
            #[cfg(not(feature = "net"))] { Err(...) }
        }
        Some(&"crypto") => CryptoLib::call_function(name, args, seed_override),
        Some(&"io")     => IoLib::call_function(name, args, pc),
        Some(&"db")     => DbLib::call_function(name, args, pc),
        Some(&"regex")  => RegexLib::call_function(name, args),
        Some(&"sys")    => SysLib::call_function(name, args, pc),
        Some(&"ffi")    => FfiLib::call_function(name, args, pc),
        Some(&"plugin") => PluginLib::call_function(name, args, pc),
        None            => Err(RuntimeError::new(format!("Unknown function: {}", name)))
    }
}
```

**Done When:**
- All existing stdlib tests pass (no behavior change)
- `cargo test` green
- 2 tests: known function routes correctly; unknown function gives clear error

---

## Task P.2 — String Interning / Arc<str>

**Status:** `[ ]`
**Priority:** MEDIUM — O(n²) string concat in loops is a real footgun
**Risk:** MEDIUM — changes Value::String variant
**File:** `src/runtime/core/value.rs`

### What is wrong

`Value::String(String)` clones the entire heap allocation on every:
- Function call argument binding
- Variable assignment
- Closure environment capture
- Return value

A function called in a 10,000-iteration loop with a 1KB string argument = 10MB of
allocation + memcpy. The GC (AllocationTracker) never collects because Rust Drop
handles it, but the CPU cost is real.

Additionally, string concatenation in a loop:
```
store → result → ""
for → i in range(0, 10000)
  store → result → result + to_string(i)
end
```
This is O(n²): each `+` allocates a new `String` of length `len(result) + len(item)`.

### What to do

**Step 1 — Change Value::String to use Arc<str>**

```rust
// value.rs
pub enum Value {
    String(Arc<str>),  // was: String(String)
    // ... rest unchanged
}
```

`Arc<str>` allows O(1) clone (just increments ref count). String content is shared
until mutation is needed.

**Step 2 — Update all String construction sites**

```rust
// BEFORE:
Value::String("hello".to_string())
// AFTER:
Value::String(Arc::from("hello"))
```

**Step 3 — Update all String read sites**

```rust
// BEFORE:
if let Value::String(s) = val { s.len() }
// AFTER:
if let Value::String(s) = val { s.len() }  // Arc<str> derefs to &str — same code
```

**Step 4 — Add str_concat_builder() to stdlib**

Detect `str_concat(array)` or add `str_build(parts: array)` that uses a single
`String::with_capacity()` + push loop (O(n) total).

**Step 5 — Add lint rule for string concat in loop** (see T.4)

**Done When:**
- `Value::String` uses `Arc<str>`
- Clone of string is O(1) (ref count only)
- `str_build(arr)` joins array elements in O(n) total
- 2 tests: Arc<str> clone is O(1); str_build correctness

---

## Task P.3 — Clone-on-Write for Arrays and Maps

**Status:** `[ ]`
**Priority:** MEDIUM — large arrays are cloned on every pass-by-value
**Risk:** HIGH — structural change to Value enum
**File:** `src/runtime/core/value.rs`

### What is wrong

```rust
pub enum Value {
    Array(Vec<Value>),   // cloned entirely on every assignment
    Map(IndexMap<String, Value>),  // same
}
```

Passing a 10,000-element array to a function = O(10,000) deep clone.

### What to do

**Step 1 — Wrap in Rc<RefCell<...>>**

```rust
pub enum Value {
    Array(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<IndexMap<String, Value>>>),
}
```

**Step 2 — Use clone-on-write semantics for mutation**

When mutating (array push, map insert), check `Rc::strong_count`:
- If `strong_count == 1`: mutate in place
- If `strong_count > 1`: clone first (copy-on-write)

```rust
fn array_push(arr: &Value, item: Value) -> Value {
    if let Value::Array(rc) = arr {
        if Rc::strong_count(rc) == 1 {
            rc.borrow_mut().push(item);
            arr.clone()  // O(1) — just increments refcount
        } else {
            let mut new_vec = rc.borrow().clone();
            new_vec.push(item);
            Value::Array(Rc::new(RefCell::new(new_vec)))
        }
    } else { panic!("expected array") }
}
```

**Important caveat:** This changes mutation semantics. Currently arrays are
value-semantics (pass-by-copy). With Rc, two variables could share the same array
if no mutation occurs. The current behavior must be preserved for user-visible semantics.

**Strategy:** Use `Rc` for read-sharing but always clone on write. The user still
sees value semantics; we just avoid the upfront clone cost when the value is only read.

**Done When:**
- Passing a large array to a function does not clone it upfront
- Mutating one copy does not affect other copies (value semantics preserved)
- 2 tests: shared array isolation; mutation independence

---

## Task P.4 — Function Call Argument Pooling

**Status:** `[ ]`
**Priority:** LOW — micro-optimization
**Risk:** LOW
**File:** `src/runtime/execution/expressions/function_calls.rs`

### What is wrong

Every function call does:
```rust
let args: Vec<Value> = evaluated_args.collect();
```

For a function called 1,000,000 times with 3 arguments, this is 1,000,000 `Vec::with_capacity(3)` allocations (one alloc + one dealloc per call).

### What to do

Use a thread-local small-vec pool. For calls with ≤ 8 arguments (99.9% of all calls),
reuse a pre-allocated buffer:

```rust
thread_local! {
    static ARG_BUF: RefCell<Vec<Value>> = RefCell::new(Vec::with_capacity(8));
}

// In call site:
ARG_BUF.with(|buf| {
    let mut args = buf.borrow_mut();
    args.clear();
    for arg_expr in arg_exprs { args.push(eval_expr(arg_expr)); }
    call_function(name, &args)
})
```

**Done When:**
- Function call argument evaluation reuses a thread-local buffer
- No regression in any existing test
- 1 test: function called 100,000 times completes in < 500ms

---

# GROUP Q — TYPE SYSTEM COMPLETION (Layer 2 → 100%)
**Goal:** Type system is correct, complete, and honestly enforced.
**Target:** v4.1.0
**Expected test delta:** +12 tests

---

## Task Q.1 — Null-Flow Type Narrowing

**Status:** `[x]` DONE — null-narrowing in checker.rs; narrowed HashMap
**Priority:** HIGH — every nullable check currently gives no type benefit
**Risk:** MEDIUM
**File:** `src/typecheck/checker.rs`

### What is wrong

The type checker does not narrow types after null-safety checks:

```
store → val: string? → get_or_null()
if val != null
  // val should be narrowed to 'string' here
  // but checker still sees it as 'string?' and warns on method calls
  print(str_upper(val))  // [WARNING] type: expected string, got string?
end
```

### What to do

**Step 1 — Add `NarrowedTypes` map to TypeChecker state**

```rust
pub struct TypeChecker {
    // ... existing fields
    narrowed: HashMap<String, Type>,  // variable → narrowed type within branch
}
```

**Step 2 — Detect null-check patterns**

In `check_statement` for `Statement::If`:
```rust
// Pattern: if x != null { body }
if is_null_inequality_check(&condition) {
    let var_name = extract_variable_name(&condition);
    let base_type = self.context.variable_type(&var_name);
    let narrowed_type = strip_nullable(base_type);
    self.narrowed.insert(var_name.clone(), narrowed_type);
    self.check_block(then_body);
    self.narrowed.remove(&var_name);  // exit narrow scope
}
```

**Step 3 — Use narrowed type in expression checking**

```rust
fn infer_expression_type(&self, expr: &Expression) -> Type {
    if let Expression::Identifier(name) = expr {
        // Check narrowed first, then fall back to context
        if let Some(t) = self.narrowed.get(name) { return t.clone(); }
        return self.context.variable_type(name);
    }
    // ...
}
```

**Done When:**
- `if x != null { use_x() }` does not produce false type warnings
- `if x == null { return } use_x()` (early exit narrowing) also works
- 4 tests: null-inequality narrowing; early-exit narrowing; nested narrowing;
  no narrowing outside branch

---

## Task Q.2 — Struct Field Type Enforcement at Construction

**Status:** `[x]` DONE — check_struct_literal_fields(); struct_field_types map
**Priority:** HIGH — struct construction ignores declared field types
**Risk:** LOW
**File:** `src/typecheck/checker.rs`, `src/runtime/execution/expressions/mod.rs`

### What is wrong

```
struct Point(x: int, y: int)
store → p → Point(x: "hello", y: "world")
// No error, no warning — the int annotation is silently ignored
```

Field types are declared but never checked at construction time.

### What to do

**Step 1 — Type-check struct literal in checker.rs**

In `check_expression` for `Expression::StructLiteral { name, fields }`:
```rust
if let Some(struct_def) = self.context.get_struct(&name) {
    for (field_name, field_expr) in fields {
        if let Some(expected_type) = struct_def.field_type(&field_name) {
            let actual_type = self.infer_expression_type(field_expr);
            if !types_compatible(&actual_type, expected_type) {
                self.emit(format!(
                    "struct {} field '{}': expected {}, got {}",
                    name, field_name, expected_type, actual_type
                ));
            }
        }
    }
}
```

**Step 2 — Store struct field types in TypeContext**

TypeContext already has `struct_types: HashMap<String, StructType>` from N.2 work.
Ensure field types are populated from `Statement::Struct` in `collect_declarations`.

**Step 3 — Runtime enforcement when --strict-types**

In `execution/expressions/mod.rs`, in the struct literal evaluator:
```rust
if vm.strict_types {
    for (field_name, value) in &fields {
        if let Some(expected) = struct_def.field_type(field_name) {
            if !type_matches_value(value, expected) {
                return Err(vm.runtime_error(format!(
                    "struct {} field '{}': type mismatch", struct_name, field_name
                )));
            }
        }
    }
}
```

**Done When:**
- Struct construction with wrong field type emits advisory warning
- With `--strict-types`, wrong field type halts before execution
- 4 tests: correct construction passes; wrong type warns; strict mode halts;
  unannotated field (no annotation = no warning)

---

## Task Q.3 — Protocol Violation → Typed Runtime Error

**Status:** `[x]` DONE — E0029 emitted; call_method checks Map protocol entries
**Priority:** MEDIUM — currently "undefined function" instead of "protocol violation"
**Risk:** LOW
**Files:** `src/runtime/vm/core.rs`, `src/runtime/errors.rs`

### What is wrong

If a struct declares `implements → Serializable` but is missing the `serialize` method,
calling `obj.serialize()` at runtime gives:

```
Runtime error: Undefined function 'Point.serialize'
```

This error code does not distinguish "method doesn't exist" from "protocol method missing".
The TypeChecker catches this at type-check time (N.2), but if `--no-type-check` is used or
the type checker misses the case, the runtime gives the wrong error.

### What to do

**Step 1 — Add E0029 error code**

```rust
// errors.rs
E0029 => "protocol method not implemented",
```

**Step 2 — Check __implements_ tag on method dispatch miss**

In `call_struct_method()` (or wherever method-not-found is raised):
```rust
// BEFORE:
return Err(RuntimeError::new(format!("Undefined function '{}.{}'", struct_name, method)));

// AFTER:
let implements = vm.get_variable(format!("__implements_{}", struct_name));
if let Some(Value::Array(protocols)) = implements {
    for proto in protocols {
        if let Value::String(proto_name) = proto {
            let methods = vm.get_variable(format!("__protocol_{}", proto_name));
            if method_in_protocol(method, &methods) {
                return Err(RuntimeError::new(format!(
                    "struct '{}' declares 'implements {}' but is missing required method '{}'",
                    struct_name, proto_name, method
                )).with_code(E0029));
            }
        }
    }
}
return Err(RuntimeError::new(format!("Undefined method '{}.{}'", struct_name, method)));
```

**Done When:**
- Missing protocol method gives E0029 with clear message naming the protocol
- Regular missing method still gives "undefined method"
- 2 tests: protocol-missing-method gives E0029; non-protocol-missing-method not E0029

---

## Task Q.4 — Remaining Type::Int Default Audit

**Status:** `[x]` DONE — all unwrap_or(Type::Int) → unwrap_or(Type::Unknown)
**Priority:** MEDIUM — K.1 fixed the main cases but may have missed some
**Risk:** LOW
**Files:** `src/typecheck/checker.rs`, `src/typecheck/inference.rs`

### What to do

```bash
grep -n "unwrap_or(Type::Int)" src/typecheck/checker.rs src/typecheck/inference.rs
grep -n "unwrap_or(Type::Int)" src/runtime/execution/
```

For every occurrence found:
- If it is a function parameter with no annotation → change to `Type::Unknown`
- If it is a literal integer expression → `Type::Int` is correct (keep)
- If it is a variable with no declared type → change to `Type::Unknown`

**Done When:**
- Zero `unwrap_or(Type::Int)` occurrences in unannotated-param contexts
- All regression tests pass
- 2 tests: unannotated param accepts any type; annotated param rejects wrong type

---

# GROUP R — STDLIB CORRECTNESS (Layer 6 → 100%)
**Goal:** Every stdlib function is correct, non-stub, and safe.
**Target:** v4.1.0
**Expected test delta:** +12 tests

---

## Task R.1 — Fix db_transaction Auto-Rollback (carry from A.7)

**Status:** `[x]` DONE — auto-rollback on Err; begin/commit/rollback wired
**Priority:** HIGH — data corruption risk
**Risk:** MEDIUM
**File:** `src/stdlib/db.rs`

### What is wrong

`db_transaction(conn_id)` issues `BEGIN` but requires the caller to manually commit
or rollback. A script that errors mid-transaction leaves an open transaction permanently
(until the connection is closed). SQLite silently rolls back on connection close, but
PostgreSQL/MySQL hold locks indefinitely.

```
store → conn → db_connect("sqlite:./data.db")
db_execute(conn, "BEGIN")
db_execute(conn, "INSERT INTO users VALUES (1, 'alice')")
error → MyError → "something failed"   // transaction never closed
```

### What to do

**Step 1 — Change db_transaction to accept a closure**

New signature:
```
db_transaction(conn_id: int, fn: function) → value
```

**Step 2 — Implement in db.rs**

```rust
"db_transaction" => {
    // Validate args: conn_id (int) + handler (function/lambda)
    let conn_id = match &args[0] { Value::Integer(n) => *n, _ => return Err(...) };
    let handler = args[1].clone();

    // Issue BEGIN
    db_execute_raw(conn_id, "BEGIN")?;

    // Call handler via VM executor
    let result = executor
        .as_mut()
        .ok_or_else(|| RuntimeError::new("db_transaction requires VM context"))?
        .call_function_value(&handler, &[]);

    match result {
        Ok(val) => {
            // Handler succeeded → COMMIT
            db_execute_raw(conn_id, "COMMIT")?;
            Ok(val)
        }
        Err(e) => {
            // Handler failed → ROLLBACK (best-effort; ignore rollback error)
            let _ = db_execute_raw(conn_id, "ROLLBACK");
            Err(e)  // re-raise original error
        }
    }
}
```

**Step 3 — Wire in stdlib/mod.rs**

Add `db_transaction` to the executor-required dispatch (same as `http_serve`, `ws_serve`):
```rust
} else if name == "db_transaction" {
    if let Some(exec) = executor {
        return DbLib::transaction_with_executor(args, exec, effective_permission_checker);
    }
    return Err(RuntimeError::new("db_transaction requires VM executor context"));
```

**Done When:**
- `db_transaction(conn, fn)` commits on success, rollbacks on error
- Manual `db_execute(conn, "BEGIN")` still works (backward compat)
- `db_commit(conn)` and `db_rollback(conn)` still work (backward compat)
- 3 tests: successful transaction commits; error in handler rolls back;
  nested db_execute inside transaction

---

## Task R.2 — Database Connection Safety

**Status:** `[x]` DONE — MAX_DB_CONNECTIONS=50; error E0053 on exceed
**Priority:** MEDIUM
**Risk:** LOW
**File:** `src/stdlib/db.rs`

### What is wrong

`db_connect()` opens a new connection each time with no limit. 1,000 concurrent
`db_connect()` calls = 1,000 open file handles (SQLite) or 1,000 TCP connections
(PostgreSQL). Most databases have a connection limit (typically 100 for PostgreSQL).

### What to do

**Step 1 — Add connection limit constant**

```rust
const MAX_DB_CONNECTIONS: usize = 50;

// In DB_CONNECTIONS registry:
static DB_CONNECTIONS: Lazy<Mutex<HashMap<i64, DbConnection>>> = ...;

// In db_connect:
let count = DB_CONNECTIONS.lock()?.len();
if count >= MAX_DB_CONNECTIONS {
    return Err(RuntimeError::new(format!(
        "db_connect: maximum {} connections reached; call db_close() first",
        MAX_DB_CONNECTIONS
    )));
}
```

**Step 2 — Warn on unclosed connections at VM shutdown**

In VM `Drop` implementation, check if any `DB_CONNECTIONS` entries were opened by
this VM but not closed. If so, log a warning:
```
[warning] db: 3 database connections were not closed. Call db_close(conn) to avoid leaks.
```

**Done When:**
- `db_connect()` enforces MAX_DB_CONNECTIONS limit
- Warning on unclosed connections at VM drop
- 2 tests: connection limit enforced; warning on unclosed

---

## Task R.3 — Full Stdlib Function Audit

**Status:** `[x]` DONE — full stdlib audit; panic-free; edge cases handled
**Priority:** HIGH — ensures no functions return hardcoded/fake values
**Risk:** LOW
**Files:** All `src/stdlib/*.rs`

### What to do

Audit every stdlib function for:
1. Functions that return hardcoded values (`Ok(Value::Null)` without doing work)
2. Functions that ignore their arguments
3. Functions that claim a feature but `cfg(not(feature = "..."))` returns an error
4. Functions with TODOs or FIXMEs

```bash
grep -n "todo!\|unimplemented!\|// TODO\|// FIXME" src/stdlib/
grep -n "Ok(Value::Null)" src/stdlib/ | grep -v "// null return is correct"
```

For each found:
- Mark as confirmed-stub: add `[STUB]` comment with issue ID
- Implement or file follow-up task
- Add a test that exercises the actual behavior

**Done When:**
- Zero `todo!()` or `unimplemented!()` in stdlib
- Zero undocumented `Ok(Value::Null)` returns
- All stubs either implemented or explicitly marked with tracking issue
- 3 tests: one per discovered and fixed stub

---

## Task R.4 — String Concat O(n²) Lint + stdlib Fix

**Status:** `[x]` DONE — str_build() added; L019 lint rule for O(n²) concat
**Priority:** MEDIUM — most common performance trap for Txt-code users
**Risk:** LOW
**Files:** `src/tools/linter.rs`, `src/stdlib/core.rs`

### What is wrong

```
store → result → ""
for → i in range(0, 10000)
  store → result → result + to_string(i)  // O(n²) total allocations
end
```

This is the most common performance mistake in any language with immutable strings.
Users write this naturally; it silently degrades to O(n²) without warning.

### What to do

**Step 1 — Add lint rule L020: string concatenation in loop body**

In `linter.rs`, detect the pattern:
- A `for` or `while` loop body
- Contains an assignment `store → x → x + ...` where `x` is a `string` variable

```rust
LintRule::StringConcatInLoop => {
    // Detect: for/while loop containing 'store → s → s + ...'
    // Emit: L020 — string concat in loop is O(n²); use str_join(array) instead
}
```

**Step 2 — Add `str_join` if not present**

`str_join(arr: array, sep: string) → string` — already present; document it clearly.

**Step 3 — Add `str_build(parts: array) → string`**

```rust
"str_build" => {
    // Allocate with_capacity(sum of lengths) then push each part
    let total_len: usize = arr.iter().map(|v| v.to_string().len()).sum();
    let mut s = String::with_capacity(total_len);
    for v in arr { s.push_str(&v.to_string()); }
    Ok(Value::String(Arc::from(s.as_str())))
}
```

**Done When:**
- L020 lint rule detects string concat in loops
- `str_build()` builds a string in O(n) total
- 2 tests: L020 fires on loop concat; L020 does not fire on non-loop concat

---

# GROUP S — SECURITY COMPLETENESS (Layer 7 → 100%)
**Goal:** All security claims are verifiable and cross-platform.
**Target:** v4.2.0
**Expected test delta:** +8 tests

---

## Task S.1 — Plugin Library Sandboxing

**Status:** `[ ]`
**Priority:** HIGH — plugin system bypasses all VM security
**Risk:** HIGH — complex; platform-specific
**File:** `src/stdlib/plugin.rs`

### What is wrong

`plugin_load(path)` calls `libloading::Library::new(path)` which loads an arbitrary
`.so` into the VM process. The loaded library runs with **full OS permissions** of
the txtcode process, completely bypassing the VM permission system.

An `--allow-ffi` path check prevents loading from arbitrary locations, but a library
at an allowed path can still: spawn processes, read/write any files, make network
connections, and inject code.

### What to do

**Option A — Fork-based isolation (RECOMMENDED for Linux)**

Load the plugin in a child process. Communicate via a Unix socket using JSON.
The child process can have seccomp applied before loading the library.

```rust
#[cfg(target_os = "linux")]
fn plugin_load_sandboxed(path: &str) -> Result<i64, RuntimeError> {
    // Fork a child process
    let (parent_sock, child_sock) = UnixStream::pair()?;
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // Child: apply seccomp, load library, enter request loop
        drop(parent_sock);
        apply_plugin_seccomp();
        plugin_child_main(path, child_sock);
        unsafe { libc::exit(0) };
    }
    // Parent: register child socket as plugin handle
    let handle = register_plugin_process(pid, parent_sock);
    Ok(handle)
}
```

**Option B — Runtime warning (INTERIM)**

For now, emit a clear security warning when plugin_load is called:
```rust
eprintln!(
    "[security] WARNING: plugin_load() loads a native library with full OS permissions. \
     The library is NOT sandboxed. Only load plugins from trusted sources. \
     Future versions will sandbox plugin execution."
);
```

Document this limitation prominently in `crates/plugin-sdk/README.md`.

**Done When (Option B — interim):**
- `plugin_load()` prints security warning to stderr
- `crates/plugin-sdk/README.md` documents the security limitation
- 2 tests: warning is printed; plugin_load with bad path gives clear error

**Done When (Option A — full):**
- Plugin runs in isolated child process
- Plugin seccomp prevents syscalls outside the plugin protocol
- 3 tests: plugin executes correctly; plugin cannot affect parent process; fork sandbox works

---

## Task S.2 — Package Registry Manifest Signing

**Status:** `[ ]`
**Priority:** MEDIUM — supply chain integrity
**Risk:** LOW
**Files:** `src/cli/package.rs`, `registry/index.json`

### What is wrong

`cli/package.rs` calls `verify_sha256_manifest()` which checks tarball SHA-256 against
a manifest file. But the manifest itself is downloaded from the registry without
authentication. A MITM attack can replace both the manifest and the tarball with
identical SHA-256 checksums for malicious content.

### What to do

**Step 1 — Sign registry manifests with Ed25519**

Use the same Ed25519 infrastructure already in `src/security/auth.rs`:

```rust
// When publishing a package (registry_server):
let manifest_bytes = serde_json::to_vec(&manifest)?;
let signature = ed25519_sign(&manifest_bytes, &REGISTRY_PRIVATE_KEY)?;

// Add signature to index.json:
{
  "name": "stdlib-extra",
  "version": "1.0.0",
  "sha256": "abc...",
  "signature": "ed25519:BASE64...",
  "signer_pubkey": "BASE64..."
}
```

**Step 2 — Verify signature in package install**

```rust
// In package.rs, before accepting a manifest:
if let Some(sig) = manifest.signature {
    let pubkey = REGISTRY_TRUSTED_KEY;  // built into binary or ~/.txtcode/trusted_keys
    ed25519_verify(&manifest_bytes, &sig, &pubkey)
        .map_err(|_| "package manifest signature invalid — possible supply chain attack")?;
}
```

**Step 3 — Trusted key management**

Embed registry public key in binary (same approach as `update_verifier.rs`).
Allow `--trusted-key FILE` to override for air-gapped environments.

**Done When:**
- Registry manifests are signed
- `package install` verifies signature before SHA-256 check
- Invalid signature gives clear error message
- 2 tests: valid signature accepted; invalid signature rejected

---

## Task S.3 — Real Memory Limits (RSS-based)

**Status:** `[x]` DONE 2026-03-22
**Priority:** MEDIUM — current heuristic over/under-estimates
**Risk:** LOW
**File:** `src/runtime/gc.rs`

### What is wrong

`estimate_value_bytes()` uses conservative over-estimates:
```rust
Value::String(s)    => 64 + s.len()
Value::Array(arr)   => 64 + arr.len() * 40
Value::Boolean(_)   => 1  // correct
```

A `Value::Array` of 1M booleans: real = 1MB, estimated = 40MB (40× overcount).
The limit triggers at 40MB even though real usage is 1MB.

Conversely, a `Value::Map` with 10K 1-char keys: real ≈ 2MB, estimated ≈ 800KB.
The limit does NOT trigger even though real usage is 2MB.

### What to do

**Step 1 — Read real RSS on Linux**

```rust
#[cfg(target_os = "linux")]
fn get_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
            return Some(kb * 1024);
        }
    }
    None
}
```

**Step 2 — Use RSS in collect_checked()**

```rust
pub fn collect_checked(&mut self) -> Result<(), RuntimeError> {
    if let Some(max) = self.max_bytes {
        let actual = get_rss_bytes()
            .unwrap_or_else(|| self.estimate_total_bytes()); // fallback to estimate
        if actual > max {
            return Err(RuntimeError::new(format!(
                "memory limit exceeded: {} MB used, {} MB limit",
                actual / 1024 / 1024, max / 1024 / 1024
            )).with_code(E0021));
        }
    }
    Ok(())
}
```

**Step 3 — macOS: use task_info**

```rust
#[cfg(target_os = "macos")]
fn get_rss_bytes() -> Option<u64> {
    // mach_task_self() → task_info(TASK_VM_INFO) → phys_footprint
    // Requires: mach/task_info.h bindings via libc or extern C
    None  // TODO: implement via libc::TASK_VM_INFO
}
```

**Done When:**
- Linux: memory limit uses real RSS from `/proc/self/status`
- Non-Linux: falls back to estimate (no regression)
- Memory limit tests updated to reflect actual behavior
- 2 tests: RSS-based limit triggers correctly; fallback to estimate when /proc unavailable

---

## Task S.4 — Expand RESERVED_ENV_KEYS

**Status:** `[ ]`
**Priority:** LOW
**Risk:** NONE
**File:** `src/cli/run.rs`

### What is wrong

`RESERVED_ENV_KEYS` blocks common injection vectors but misses platform variants:

| Missing Key | Platform | Attack Vector |
|---|---|---|
| `DYLD_FALLBACK_LIBRARY_PATH` | macOS | Library substitution |
| `DYLD_IMAGE_SUFFIX` | macOS | Suffix-based substitution |
| `LD_PRELOAD_64` | Some libc | 64-bit preload override |
| `GLIBC_TUNABLES` | Linux glibc | Heap exploitation via tunable |
| `NSS_PATH` | Linux | NSS library substitution |
| `PERL5LIB`, `PYTHONPATH`, `RUBYLIB` | All | Scripting language injection |

### What to do

```rust
const RESERVED_ENV_KEYS: &[&str] = &[
    // Existing
    "LD_PRELOAD", "LD_AUDIT", "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES", "DYLD_FORCE_FLAT_NAMESPACE", "DYLD_LIBRARY_PATH",
    "_FRIDA_AGENT", "FRIDA_TRANSPORT", "FRIDA_LISTEN",
    // New additions
    "DYLD_FALLBACK_LIBRARY_PATH", "DYLD_IMAGE_SUFFIX", "DYLD_VERSIONED_LIBRARY_PATH",
    "LD_PRELOAD_64", "LD_AUDIT_64", "GLIBC_TUNABLES",
    "NSS_PATH", "PERL5LIB", "PYTHONPATH", "RUBYLIB", "NODE_PATH",
    "JAVA_TOOL_OPTIONS", "JVM_FLAGS",
];
```

**Done When:**
- All listed keys are blocked
- Test that each blocked key returns the appropriate error
- 1 test: all RESERVED_ENV_KEYS produce "Forbidden env key" error

---

## Task S.5 — Windows Sandbox (Job Objects)

**Status:** `[ ]`
**Priority:** LOW — Windows support is not a v4 requirement
**Risk:** MEDIUM — requires Windows-specific API
**File:** `src/runtime/sandbox.rs`

### What to do

On Windows, `apply_sandbox()` should use a Job Object to restrict the process:

```rust
#[cfg(target_os = "windows")]
fn apply_windows_sandbox() -> SandboxResult {
    use windows_sys::Win32::System::JobObjects::*;

    let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job.is_null() { return Err("sandbox: CreateJobObject failed".into()); }

    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    limits.BasicLimitInformation.LimitFlags =
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE |
        JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;

    unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const _,
            std::mem::size_of_val(&limits) as u32
        );
        AssignProcessToJobObject(job, GetCurrentProcess());
    }
    Ok(())
}
```

**Done When:**
- `apply_sandbox()` on Windows applies a Job Object with kill-on-close
- `sandbox_description()` on Windows returns an accurate description
- 1 test: sandbox_available() returns true on Windows (or skip on non-Windows)

---

# GROUP T — TOOLING COMPLETION (Layer 8 → 100%)
**Goal:** Developer tools are production-quality and complete.
**Target:** v4.3.0
**Expected test delta:** +20 tests

---

## Task T.1 — LSP publishDiagnostics (Inline Error Squiggles)

**Status:** `[x]` DONE — wired on didOpen/didChange/didClose; lex+parse+typecheck+lint pipeline; 5 tests
**Priority:** CRITICAL — LSP without diagnostics is half-functional
**Risk:** MEDIUM
**File:** `src/cli/lsp.rs`

### What is wrong

The LSP server has definition, hover, rename, references, and completions. But it
never sends `textDocument/publishDiagnostics`. This means editors show **no inline
red underlines** for syntax errors or type errors. Users see errors only when they run
the program — not while editing.

### What to do

**Step 1 — Collect diagnostics on every document change**

In the `textDocument/didChange` handler:
```rust
"textDocument/didChange" => {
    let uri = params["textDocument"]["uri"].as_str().unwrap().to_string();
    let text = params["contentChanges"][0]["text"].as_str().unwrap();
    update_document_source(&uri, text);

    // Collect parse errors
    let mut diagnostics = Vec::new();
    let (ast, parse_errors) = parse_with_errors(text);

    for err in parse_errors {
        diagnostics.push(json!({
            "range": span_to_range(err.span),
            "severity": 1,  // Error
            "source": "txtcode",
            "message": err.message,
            "code": "parse-error"
        }));
    }

    // Collect type errors (advisory)
    let mut checker = TypeChecker::new();
    if let Err(type_errors) = checker.check(&ast) {
        for err in type_errors {
            diagnostics.push(json!({
                "range": /* best-effort span */ null_range(),
                "severity": 2,  // Warning
                "source": "txtcode/types",
                "message": err,
                "code": "type-warning"
            }));
        }
    }

    // Send publishDiagnostics
    send_notification("textDocument/publishDiagnostics", json!({
        "uri": uri,
        "diagnostics": diagnostics
    }));
}
```

**Step 2 — Clear diagnostics on document close**

```rust
"textDocument/didClose" => {
    send_notification("textDocument/publishDiagnostics", json!({
        "uri": params["textDocument"]["uri"],
        "diagnostics": []
    }));
}
```

**Step 3 — Map parse errors to proper ranges**

The parser's error recovery (E.4) emits `Statement::Error { message, span }`.
Use `span.line` and `span.column` to build LSP `Range` objects:
```rust
fn span_to_range(span: &Span) -> serde_json::Value {
    json!({
        "start": { "line": span.line - 1, "character": span.column - 1 },
        "end":   { "line": span.line - 1, "character": span.column + span.length - 1 }
    })
}
```

**Done When:**
- Parse errors appear as inline red underlines in VS Code / Neovim
- Type warnings appear as yellow underlines
- Diagnostics cleared on file close
- Diagnostics update within 100ms of typing (debounced)
- 4 tests: parse error diagnostic published; type warning published;
  diagnostics cleared on close; diagnostics debounced

---

## Task T.2 — DAP Debug Adapter Protocol

**Status:** `[ ]`
**Priority:** HIGH — without DAP, no editor can set breakpoints
**Risk:** HIGH — significant new subsystem
**File:** `src/cli/debug.rs` (extend), new `src/cli/dap.rs`

### What is wrong

The current debugger is a terminal-interactive REPL (`:step`, `:print`, `:break N`).
This format is not understood by VS Code, Neovim, or any IDE. The Debug Adapter Protocol
(DAP, by Microsoft) is the standard interface that all editors use.

Without DAP:
- No breakpoints in VS Code
- No variable inspection panels
- No call stack view
- No watch expressions

### What to do

**Step 1 — Add `txtcode dap` subcommand**

```rust
// txtcode.rs
Commands::Dap { file, port } => {
    dap::serve_dap(file, port)  // default port: 4711
}
```

**Step 2 — Implement DAP protocol in src/cli/dap.rs**

Minimum DAP request handlers required for breakpoint debugging:
```rust
// Required handlers:
"initialize"      → return capabilities (supportsBreakpointLocationsRequest etc.)
"launch"          → start the program with debug hooks
"setBreakpoints"  → register breakpoints at file:line pairs
"configurationDone" → begin execution
"threads"         → return thread list (single-threaded VM = 1 thread)
"stackTrace"      → return current call stack frames
"scopes"          → return variable scopes for a frame
"variables"       → return variables in a scope
"continue"        → resume execution until next breakpoint
"next"            → step over (execute current line, stop at next)
"stepIn"          → step into function call
"stepOut"         → execute until current function returns
"pause"           → interrupt running program
"disconnect"      → stop debugging session
```

**Step 3 — Wire VM execution to DAP hooks**

The AST VM already has `Debugger` integration. The DAP server needs to:
1. Set breakpoints on the `Debugger` instance
2. Block at breakpoints (channel/condvar)
3. Resume when DAP client sends `continue`/`next`/`step`

```rust
// In vm/core.rs, at each statement:
if let Some(dap) = &self.dap_session {
    let span = stmt.span;
    if dap.has_breakpoint(span.line) {
        dap.pause_and_wait(self);  // blocks until client sends continue
    }
}
```

**Step 4 — Return variable values as DAP Variable objects**

```rust
fn variables_response(scope: &HashMap<String, Value>) -> Vec<DapVariable> {
    scope.iter().map(|(name, val)| DapVariable {
        name: name.clone(),
        value: format_value_for_dap(val),
        variables_reference: 0,  // 0 = no children; >0 for arrays/maps
        type_: type_name_for_dap(val),
    }).collect()
}
```

**Done When:**
- `txtcode dap --file script.tc` starts a DAP server on port 4711
- VS Code extension connects and shows breakpoints, call stack, variables
- Single-step (next/stepIn/stepOut) works
- Variable values shown in Variables panel
- 5 tests: initialize; setBreakpoints; stackTrace at breakpoint;
  variables in scope; continue resumes

---

## Task T.3 — Formatter Idempotency Certification

**Status:** `[ ]`
**Priority:** HIGH — "format on save" loops are catastrophic in IDEs
**Risk:** LOW
**File:** `src/tools/formatter.rs`

### What is wrong

The formatter has never been tested for idempotency: `format(format(source)) == format(source)`.
This is a hard requirement for any "format on save" integration. If formatting a file twice
produces different results, the IDE will enter an infinite save loop.

### What to do

**Step 1 — Add idempotency test harness**

```rust
// tests/unit/test_formatter.rs
fn assert_idempotent(source: &str) {
    let once  = Formatter::new().format(source).unwrap();
    let twice = Formatter::new().format(&once).unwrap();
    assert_eq!(once, twice,
        "Formatter is not idempotent!\n\nFirst pass:\n{}\n\nSecond pass:\n{}",
        once, twice
    );
}

#[test] fn idempotent_simple_function() { assert_idempotent(r#"..."#); }
#[test] fn idempotent_nested_lambdas()  { assert_idempotent(r#"..."#); }
// ... 30 programs
```

**Step 2 — Run against all example programs**

```bash
for f in examples/*.tc; do
    txtcode format $f > /tmp/pass1.tc
    txtcode format /tmp/pass1.tc > /tmp/pass2.tc
    diff /tmp/pass1.tc /tmp/pass2.tc && echo "PASS: $f" || echo "FAIL: $f"
done
```

**Step 3 — Fix every discovered non-idempotency**

Common causes:
- Trailing blank lines after `end`
- Arrow spacing around nested lambdas
- Indentation of multi-line method chains
- Comment preservation with whitespace

**Done When:**
- `format(format(source)) == format(source)` on 30 distinct programs
- All examples in `examples/` pass idempotency check
- CI step added: `txtcode format --check` on all test programs
- 10 new idempotency tests covering identified edge cases

---

## Task T.4 — Linter Expansion to 25+ Rules

**Status:** `[ ]`
**Priority:** MEDIUM
**Risk:** LOW
**File:** `src/tools/linter.rs`

### Current rules (L001–L019, 10 confirmed active)

See F.2 (COMPLETE) for existing rules. Add these new rules:

### New rules to add

| Rule | Name | Description | Auto-fix |
|---|---|---|---|
| L020 | string-concat-loop | `s = s + item` in loop body (O(n²)) | No — suggest str_build() |
| L021 | unused-import | `import →` module never referenced | Yes — remove line |
| L022 | dead-code-after-return | Statements after `return` in same block | Yes — remove |
| L023 | infinite-loop-no-break | `while → true` with no `break` or `return` | No — needs review |
| L024 | shadowed-function-param | Function param shadows outer variable | No — rename param |
| L025 | function-too-long | Function body > 60 statements (configurable) | No — refactor |
| L026 | missing-return-on-all-paths | Return type declared; some paths return null | No |
| L027 | comparison-to-boolean | `if x == true` → use `if x` | Yes |
| L028 | empty-function-body | Function with no statements | No — likely a stub |
| L029 | async-run-no-await | `store → _ → async_run(...)` with result never awaited | No |

### Implementation sketch for L022 (dead code after return)

```rust
fn check_dead_code_after_return(stmts: &[Statement]) -> Vec<LintError> {
    let mut errors = Vec::new();
    let mut found_return = false;
    for stmt in stmts {
        if found_return {
            errors.push(LintError {
                rule: "L022",
                message: "unreachable code after return statement".to_string(),
                span: stmt.span,
                fix: Some(Fix::DeleteStatement),
            });
        }
        if matches!(stmt, Statement::Return { .. }
                         | Statement::Break
                         | Statement::Continue) {
            found_return = true;
        }
    }
    errors
}
```

**Done When:**
- 10 new rules (L020–L029) implemented
- Each rule has detect-case and no-detect-case tests
- Auto-fixable rules work with `txtcode lint --fix`
- `txtcode lint --rules L020,L022` allows selective rule execution
- 20 tests (2 per new rule)

---

## Task T.5 — Conditional Breakpoints and Bytecode Debugger

**Status:** `[ ]`
**Priority:** LOW
**Risk:** MEDIUM
**File:** `src/cli/debug.rs`

### What to do

**Conditional breakpoints:**
```rust
// Current breakpoint: stop at line N
pub fn add_breakpoint(&mut self, line: usize)

// New: conditional breakpoint with expression
pub fn add_conditional_breakpoint(&mut self, line: usize, condition: String) {
    self.breakpoints.push(Breakpoint {
        line,
        condition: Some(condition),
    });
}

// In execution: evaluate condition before stopping
if self.has_breakpoint(line) {
    if let Some(cond) = breakpoint.condition {
        let result = self.eval_expression(&cond);
        if result == Value::Boolean(true) { self.pause(); }
    } else { self.pause(); }
}
```

**Bytecode VM debugger:**
Add `debug_info: Vec<(usize, usize)>` (ip → line) to bytecode (already exists per
memory notes). Wire the bytecode VM's instruction loop to check breakpoints:
```rust
// In bytecode_vm execute loop:
if let Some(line) = debug_info.get(self.ip) {
    if self.debugger.should_pause_at(*line) { self.debugger.pause(self); }
}
```

**Done When:**
- `:break N if x > 5` sets conditional breakpoint in terminal debugger
- Bytecode VM supports line-level stepping (when debug info available)
- 2 tests: conditional breakpoint fires on true condition; skips on false

---

# GROUP U — ECOSYSTEM (Layer 9 → 100%)
**Goal:** Txt-code is usable by a new developer in under 5 minutes.
**Target:** v5.0.0
**Expected test delta:** +5 CI integration tests

---

## Task U.1 — Binary Releases CI

**Status:** `[ ]`
**Priority:** HIGH — public release requires installable binaries
**Risk:** LOW
**File:** `.github/workflows/release.yml`

### What to do

The release workflow exists (`.github/workflows/release.yml`) from Group 23.1.
Verify it produces:
1. `txtcode-linux-x86_64` — static binary (musl target)
2. `txtcode-linux-aarch64` — ARM64 Linux
3. `txtcode-macos-x86_64` — Intel Mac
4. `txtcode-macos-aarch64` — Apple Silicon
5. `txtcode-windows-x86_64.exe` — Windows
6. `txtcode_3.x.x_amd64.deb` — Debian/Ubuntu package
7. SHA-256 checksums file
8. Ed25519 signature for each binary

```yaml
# .github/workflows/release.yml
jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: "${{ matrix.target }}" }
      - run: cargo build --release --target ${{ matrix.target }}
      - run: |
          sha256sum target/${{ matrix.target }}/release/txtcode > txtcode.sha256
          scripts/sign_release.sh target/${{ matrix.target }}/release/txtcode
```

**Done When:**
- All 5 platform binaries build in CI
- SHA-256 + Ed25519 signature files included in release
- GitHub Release created on `v*` tag push
- 1 CI test: release workflow completes on tag push

---

## Task U.2 — Install Script

**Status:** `[ ]`
**Priority:** HIGH — no public release without a one-liner install
**Risk:** LOW
**File:** `scripts/install.sh`

### What to do

A POSIX shell script that:
1. Detects OS + architecture
2. Downloads the appropriate binary from GitHub Releases
3. Verifies SHA-256 checksum
4. Verifies Ed25519 signature
5. Installs to `/usr/local/bin/txtcode` (or `~/.local/bin/`)

```bash
#!/bin/sh
set -e
REPO="https://github.com/ORG/txtcode/releases/latest/download"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  ARCH="x86_64"  ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac
BINARY="txtcode-${OS}-${ARCH}"
echo "Downloading $BINARY..."
curl -fsSL "$REPO/$BINARY" -o txtcode
curl -fsSL "$REPO/$BINARY.sha256" -o txtcode.sha256
sha256sum --check txtcode.sha256
chmod +x txtcode
sudo mv txtcode /usr/local/bin/txtcode
echo "Installed: $(txtcode --version)"
```

Also provide:
- Homebrew formula (`Formula/txtcode.rb`)
- `.deb` package (from release CI)
- `cargo install txtcode` (publish to crates.io)

**Done When:**
- `curl https://get.txtcode.dev | sh` installs txtcode on Linux/macOS
- Homebrew formula works: `brew install txtcode`
- `cargo install txtcode` works
- 1 test: install script completes without error on Ubuntu 22.04

---

## Task U.3 — Web Playground Deployment

**Status:** `[ ]`
**Priority:** MEDIUM — zero-friction tryout for new users
**Risk:** LOW (infrastructure only)
**Files:** `playground/`, `.github/workflows/playground.yml`

### What to do

The WASM playground exists (`playground/index.html`, `playground/app.js`).
The CI workflow exists (`.github/workflows/playground.yml`).
It needs to actually deploy to GitHub Pages.

**Step 1 — Verify WASM compilation**

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --features wasm
wasm-bindgen target/wasm32-unknown-unknown/debug/playground.wasm \
  --out-dir playground/wasm --target web
```

**Step 2 — Enable GitHub Pages in repo settings**

Deploy branch: `gh-pages`
Source: GitHub Actions

**Step 3 — Fix playground.yml deploy step**

```yaml
- name: Deploy to GitHub Pages
  uses: JamesIves/github-pages-deploy-action@v4
  with:
    folder: playground/
    branch: gh-pages
```

**Done When:**
- `https://ORG.github.io/txtcode/` loads the web REPL
- Hello-world example runs in browser
- WASM compilation CI is green
- 1 test: playground page loads and returns expected output for hello-world

---

## Task U.4 — Community Documentation (docs.txtcode.dev)

**Status:** `[ ]`
**Priority:** HIGH — no public release without docs
**Risk:** LOW
**Files:** `docs/` (new site structure)

### What to do

**Minimum documentation for v4.0 release:**

1. **Getting Started** — install + hello world in 2 minutes
2. **Language Reference** — all 24 statement types, 18 expression types, operators
3. **Standard Library Reference** — every function, its signature, 1 example
4. **Security Model** — permissions, `--sandbox`, audit trails, signing
5. **Embedding Guide** — C ABI + Rust API + plugin ABI
6. **CLI Reference** — every flag of every subcommand

**Tooling:**
- Use mdBook (`cargo install mdbook`) — generates static HTML from Markdown
- `docs/book.toml` → mdBook config
- Auto-generate stdlib reference from source code doc comments

```bash
# Generate stdlib reference:
cargo doc --no-deps --document-private-items
mdbook build docs/
# Deploy to GitHub Pages (alongside playground)
```

**Done When:**
- `https://docs.txtcode.dev/` serves 6 documentation sections
- CLI `txtcode --help` links to docs URL
- Stdlib reference is auto-generated and up to date
- 1 test: docs site build succeeds in CI

---

## Task U.5 — Windows CI

**Status:** `[ ]`
**Priority:** MEDIUM — enables Windows users
**Risk:** MEDIUM — platform differences
**File:** `.github/workflows/ci.yml`

### What to do

**Step 1 — Add Windows job to CI**

```yaml
windows:
  runs-on: windows-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test --no-default-features
    # Explicitly skip Linux-only tests
    - run: cargo test --features bytecode
```

**Step 2 — Fix Windows-specific issues**

Known potential issues:
- `libc::prctl` → Windows has no prctl; sandbox module must be `#[cfg(not(windows))]`
- `/proc/self/status` → Windows has no /proc; RSS detection must handle this
- `UnixStream` → Not available on Windows; DAP/plugin IPC needs Win32 alternatives
- File paths: `/` vs `\` in test paths

**Step 3 — Add sandbox_available() → false on Windows**

```rust
#[cfg(target_os = "windows")]
pub fn sandbox_available() -> bool { true }  // Job Objects available
```

**Done When:**
- `cargo test` passes on Windows CI
- Windows binary produced by release workflow
- File path tests use `Path` not string literals
- 1 test: Windows CI job completes without errors

---

## Task U.6 — Docker Image

**Status:** `[ ]`
**Priority:** LOW
**Risk:** NONE
**File:** `Dockerfile`

### What to do

```dockerfile
# Multi-stage: build in Rust image, copy binary to minimal runtime
FROM rust:1.76-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/txtcode /txtcode
ENTRYPOINT ["/txtcode"]
```

Publish to Docker Hub and GitHub Container Registry on every release tag.

**Done When:**
- `docker run -v ./script.tc:/script.tc txtcode/txtcode run /script.tc` works
- Image size < 10MB (static musl binary)
- Docker Hub and GHCR images published on release

---

# GROUP V — LANGUAGE SPEC COMPLETION (Layer 1/3 → 100%)
**Goal:** Language is fully specified with verified, machine-checkable grammar.
**Target:** v4.1.0
**Expected test delta:** +6 tests

---

## Task V.1 — Unicode Escape Sequences

**Status:** `[x]` DONE — \uXXXX and \UXXXXXXXX in lexer; char literals too
**Priority:** MEDIUM — internationalization
**Risk:** LOW
**File:** `src/lexer/token.rs`

### What to do

Add `\uXXXX` and `\u{XXXXXX}` escape handling in the string lexer:

```rust
// In lex_string_content():
'\\' => match self.advance() {
    'u' => {
        if self.peek() == '{' {
            self.advance();  // consume '{'
            let hex = self.take_while(|c| c.is_ascii_hexdigit());
            self.advance();  // consume '}'
            let code_point = u32::from_str_radix(&hex, 16)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(|| LexError::new("invalid Unicode code point"))?;
            output.push(code_point);
        } else {
            // \uXXXX (exactly 4 hex digits)
            let hex: String = (0..4).map(|_| self.advance()).collect();
            let code_point = u32::from_str_radix(&hex, 16)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(|| LexError::new("invalid \\uXXXX escape"))?;
            output.push(code_point);
        }
    }
    // ... existing escape cases
}
```

**Done When:**
- `"\u0041"` == `"A"` (U+0041)
- `"\u{1F600}"` == `"😀"` (emoji via extended syntax)
- Invalid code point gives lexer error
- 3 tests: basic \uXXXX; extended \u{...}; invalid code point errors

---

## Task V.2 — Operator Associativity Tests

**Status:** `[x]` DONE — 6 assoc tests in test_runtime.rs; all pass
**Priority:** MEDIUM — parser correctness guarantee
**Risk:** NONE (tests only)
**File:** `tests/unit/test_parser.rs`

### What to do

For every binary operator, verify the parse tree for `a OP b OP c`:
- Left-associative: `(a OP b) OP c`
- Right-associative: `a OP (b OP c)` (assignment, some specific ops)

```rust
#[test] fn test_subtract_is_left_associative() {
    let ast = parse("a - b - c");
    // Should parse as (a - b) - c
    assert_matches!(ast, BinaryOp {
        op: Subtract,
        left: box BinaryOp { op: Subtract, left: Identifier("a"), right: Identifier("b") },
        right: Identifier("c")
    });
}

#[test] fn test_exponent_if_present_is_right_associative() { ... }
#[test] fn test_add_is_left_associative() { ... }
#[test] fn test_multiply_is_left_associative() { ... }
#[test] fn test_comparison_is_left_associative() { ... }
```

**Done When:**
- Tests for all 12 binary operators confirming associativity
- Any discovered wrong-associativity fixed in parser
- 12 tests (1 per operator)

---

## Task V.3 — Grammar.ebnf Corrections + Rust Verification Tests

**Status:** `[x]` DONE — grammar.ebnf: 9 corrections; test_grammar.rs: 14 Rust tests
**Priority:** MEDIUM — grammar doc has 9 confirmed divergences from the real parser
**Risk:** LOW — all fixes are doc-only; the parser is already correct in every case
**Files:** `docs/grammar.ebnf`, `tests/integration/test_grammar.rs`

> ⚠️ **Previous spec was wrong:** The old task described a Python script calling
> `cargo run` as a subprocess. This is removed. No external languages or tools.
> The parser is a Rust library — tests call `txtcode::parser::parse_program()`
> directly, same as every other integration test.

---

### Part 1 — Fix 9 divergences in docs/grammar.ebnf

All 9 bugs are in the grammar *doc*. The parser is already correct. Doc-only fixes.

**G1 — `struct_def` implements separator**
```ebnf
// WRONG (grammar says colon):
struct_def = "struct" identifier "(" fields ")" [ "implements" ":" identifier_list ] ;

// CORRECT (parser uses skip_optional_arrow — arrow OR nothing):
struct_def = "struct" identifier [ "<" type_param_list ">" ]
               "(" [ struct_field_list ] ")"
               [ "implements" [ "→" ] identifier_list ] ;
```

**G2 — `function_def` missing dotted method name**
```ebnf
// WRONG (only plain identifier):
function_def = "define" "→" identifier "→" ...

// CORRECT (also supports Type.method):
function_def = "define" "→" ( identifier | identifier "." identifier ) "→" ...
```

**G3 — `compound_assign` missing from `statement` rule**
```ebnf
// WRONG — compound_assign is defined but never referenced in statement:
statement = assignment | function_def | ... ;   (* compound_assign missing *)

// CORRECT — add it:
statement = assignment | compound_assign | function_def | ... ;
```

**G4 — `?[` is NOT a single token**
```ebnf
// WRONG (implies lexer emits a combined ?[ token):
postfix_op = "?[" expression "]" ;

// CORRECT (two separate tokens — parser does lookahead):
postfix_op = "?" "[" expression "]" ;   (* optional index: obj?[key] *)
(* Note: optional chain prefix "?." is a single OptionalChain token *)
```

**G5 — `import_stmt` from clause is optional**
```ebnf
// WRONG (from required):
import_stmt = "import" [ "→" ] identifier_list "from" string ;

// CORRECT (from is optional; arrow after import also optional):
import_stmt = "import" [ "→" ] identifier_list [ "from" [ "→" ] ( string | identifier ) ]
                [ "as" [ "→" ] identifier ] ;
```

**G6 — `pattern_list` EBNF notation wrong**
```ebnf
// WRONG ({ pattern "," } requires trailing comma on every element):
pattern_list = { pattern "," } [ "..." identifier ] ;

// CORRECT:
pattern_list = pattern { "," pattern } [ "," "..." identifier ] ;
```

**G7 — `set_literal` has NO `set` keyword prefix**
```ebnf
// WRONG (grammar invents a "set" keyword prefix that doesn't exist):
set_literal = "set" "{" [ expression { "," expression } ] "}" ;

// CORRECT (parser disambiguates at brace: if first elem has no ":", it's a set):
set_literal = "{" expression { "," expression } "}" ;
(* Disambiguation: "{" expr ":" ... } = map;  "{" expr "," ... "}" = set *)
```

**G8 — Slice syntax uses `:` not `::`**
```ebnf
// WRONG (double colon):
postfix_op = "[" [ expression ] "::" [ expression ] "]" ;

// CORRECT (single colon, Python-style [start:end:step]):
postfix_op = "[" [ expression ] ":" [ expression ] [ ":" [ expression ] ] "]" ;
```

**G9 — Optional call is `?.( )` not `?()`**
```ebnf
// WRONG (implies ?() without the dot):
postfix_op = "?()" ;

// CORRECT (OptionalChain token "?." then "("):
postfix_op = "?." "(" [ arg_list ] ")" ;   (* obj?.(args) *)
```

---

### Part 2 — Add tests/integration/test_grammar.rs (Rust, no external deps)

One `#[test]` per grammar rule, calling `txtcode::parser::parse_program()` directly.
Each test asserts that a valid snippet parses OK and an invalid snippet returns Err.

```rust
// tests/integration/test_grammar.rs
use txtcode::parser::parse_program;

fn parses_ok(src: &str) {
    assert!(parse_program(src).is_ok(),
        "expected parse OK but got error:\n{}\nsource: {}",
        parse_program(src).unwrap_err(), src);
}

fn parses_err(src: &str) {
    assert!(parse_program(src).is_err(),
        "expected parse error but succeeded:\nsource: {}", src);
}

// G1: struct implements with arrow
#[test]
fn test_grammar_struct_implements_arrow() {
    parses_ok("struct Foo(x: int) implements → Serializable");
    parses_ok("struct Foo(x: int) implements Serializable");
}

// G2: dotted method definition
#[test]
fn test_grammar_dotted_method_def() {
    parses_ok("define → Foo.greet → (self) → string\n  return → \"hi\"\nend");
}

// G3: compound assignment as statement
#[test]
fn test_grammar_compound_assign() {
    parses_ok("store → x → 1\nx += 5\nx -= 2\nx *= 3\nx /= 2\nx %= 3");
}

// G4: optional index ?[ without dot
#[test]
fn test_grammar_optional_index_no_dot() {
    parses_ok("store → m → {\"a\": 1}\nstore → v → m?[\"a\"]");
}

// G5: import without from
#[test]
fn test_grammar_import_no_from() {
    parses_ok("import math");
    parses_ok("import → json from \"json\"");
}

// G6: pattern list with rest
#[test]
fn test_grammar_pattern_list() {
    parses_ok("store → arr → [1, 2, 3]\nstore → [a, b, ...rest] → arr");
}

// G7: set literal — no 'set' keyword, bare braces
#[test]
fn test_grammar_set_literal() {
    parses_ok("store → s → {1, 2, 3}");
    parses_ok("store → m → {\"a\": 1}");
}

// G8: slice with single colon
#[test]
fn test_grammar_slice_single_colon() {
    parses_ok("store → arr → [1,2,3,4,5]\nstore → s → arr[1:3]");
    parses_ok("store → arr → [1,2,3,4,5]\nstore → s → arr[::2]");
}

// G9: optional call obj?.(args)
#[test]
fn test_grammar_optional_call() {
    parses_ok("store → f → null\nstore → r → f?.(1, 2)");
}
```

---

### Done When
- `docs/grammar.ebnf` has all 9 corrections applied
- `tests/integration/test_grammar.rs` added with 9 tests, all passing
- `tests/integration/mod.rs` has `mod test_grammar;`
- `cargo test test_grammar` passes with 0 failures
- No Python, no subprocess, no external tools

---

# ARCHITECTURE RISKS (updated v5.0)

## P0 — Must Fix Before Release

| Risk | Impact | Group | Status |
|---|---|---|---|
| Module sub-VM permission escalation | Security model hole | O.2 | `[x]` DONE |
| db_transaction no rollback | Data corruption | R.1 | `[x]` DONE |
| Control flow signals in error channel | Latent correctness bugs | O.1 | `[ ]` |
| LSP no diagnostics | Editor unusable | T.1 | `[x]` DONE |
| Formatter idempotency unverified | IDE save loops | T.3 | `[ ]` |

## P1 — High Impact

| Risk | Impact | Group | Status |
|---|---|---|---|
| Dual VM (no strategy) | Double maintenance | O.5 | `[ ]` |
| O(n) stdlib dispatch | Performance in hot loops | P.1 | `[ ]` |
| Value::String clone-heavy | Memory in string-heavy programs | P.2 | `[ ]` |
| Per-task async timeout | Hung programs under load | O.4 | `[x]` DONE |
| Real memory limits | heuristic over/under triggers | S.3 | `[x]` DONE |

## P2 — Medium Impact

| Risk | Impact | Group | Status |
|---|---|---|---|
| Plugin unsandboxed | Full OS access via plugin | S.1 | `[ ]` |
| Registry unsigned | Supply chain attack | S.2 | `[ ]` |
| No span in runtime errors | Poor error messages | O.3 | `[x]` DONE |
| No Unicode escapes | Internationalization gap | V.1 | `[x]` DONE |
| Clone-heavy arrays/maps | Memory in data-heavy scripts | P.3 | `[ ]` |

---

# LAYER COMPLETION SCORES (v5.0 targets)

```
Layer 1  Language Core       100% → 100%  [V.3 ✓ COMPLETE — 9 grammar.ebnf fixes + 14 Rust tests]
Layer 2  Type System         100% → 100%  [COMPLETE — elseif ✓; compound/index assign ✓; expr recursion ✓; struct required fields ✓]
Layer 3  Parser + AST        100% → 100%  [O.3 ✓; T.1 ✓ COMPLETE — publishDiagnostics + 5 tests]
Layer 4  Execution Model     100% → 100%  [O.1 ✓ break/continue boundary fix + 4 tests; O.5 ✓ bytecode parity + optimizer + 3 tests]
Layer 5  Runtime System      100% → 100%  [P.1 ✓; S.3 ✓; P.2 ✓ Arc<str> +2 tests; P.4 ✓ arg pool +1 test; P.3 CoW deferred HIGH risk]
Layer 6  Standard Library    100% → 100%  [R.1 ✓; R.2 ✓; R.3 ✓ +3 tests; R.4 ✓ — COMPLETE]
Layer 7  Security Model       86%  → 100%  [S.1 plugin sandbox; S.2 registry sign; S.4 env keys; S.5 Windows]
Layer 8  Tooling              82%  → 100%  [T.2 DAP; T.3 formatter cert; T.4 linter 25+; T.5 cond break]
Layer 9  Ecosystem            15%  → 100%  [U.1 releases; U.2 install; U.3 playground; U.4 docs; U.5 Windows CI; U.6 Docker]

CURRENT WEIGHTED:  98%  (Milestone 1 ✓; Milestone 3 ✓; Layer 1+2+3+4+5+6=100%)
TARGET:           100%

TESTS: 770 current (165 unit + 605 integration) → ~820 after all groups complete
  (P = +4; S = +5; T = +7; U = +5)
```

---

# EXECUTION ORDER (strict inside-out, v5.0)

```
MILESTONE 1 — Language Core + Type System 100%  (Layers 1–2)  ✓ COMPLETE (re-verified 2026-03-21)
  V.1  Unicode escape sequences             [x] LOW risk — lexer change
  V.2  Operator associativity tests         [x] NONE risk — tests only
  Q.4  Remaining Type::Int audit            [x] LOW risk — grep + fix
  Q.1  Null-flow type narrowing             [x] MEDIUM risk — checker change
  Q.2  Struct field type enforcement        [x] LOW risk — additive
  Q.3  Protocol violation → E0029           [x] LOW risk — better error message
  W.1  Integer division truncation fix      [x] LOW risk — arithmetic.rs: a/b
  W.2  Optional chaining ?[ parse fix       [x] MEDIUM risk — parser lookahead
  W.3  Closure capture (CRITICAL)           [x] MEDIUM risk — scope snapshot on define
  W.4  Method definition dotted name        [x] LOW risk — additive parser change
  W.5  Wire tests/tc/*.tc into cargo test   [x] NONE risk — new test harness file

MILESTONE 2 — Execution + Runtime 100%  (Layers 3–5)  [~] IN PROGRESS
  O.1  break/continue boundary fix          [x] DONE — Layer 4 correctness (+4 tests)
  O.2  Module permission isolation          [x] DONE — Layer 5 security
  O.3  Span tracking in execution           [x] DONE — Layer 3 DX
  O.4  Per-task async timeout               [x] DONE — Layer 5 stability
  O.5  Bytecode parity + optimizer          [x] DONE — Layer 4 architecture (+3 tests)
  P.1  Stdlib dispatch O(1) HashMap         [x] DONE — Layer 5 performance (+2 tests)
  S.3  Real memory limits (RSS)             [x] DONE — Layer 5 accuracy (+3 tests)
  P.2  String interning Arc<str>            [x] DONE — value enum changed (+2 tests)
  P.3  Clone-on-write arrays/maps           [-] DEFERRED — HIGH risk, structural change
  P.4  Function argument pooling            [x] DONE — thread-local pool (+1 test)

MILESTONE 3 — Standard Library 100%  (Layer 6)  ✓ COMPLETE
  R.1  db_transaction auto-rollback         [x] MEDIUM risk — API behavior
  R.2  Database connection safety           [x] LOW risk — additive limit
  R.3  Full stdlib audit                    [x] LOW risk — discovery + fix
  R.4  String concat O(n²) lint + fix       [x] LOW risk — new lint rule

MILESTONE 4 — Security 100%  (Layer 7)
  S.1  Plugin sandboxing                    [ ] HIGH risk — complex isolation
  S.2  Registry manifest signing            [ ] LOW risk — additive
  S.3  Real memory limits (RSS)             [x] DONE — Layer 5 (+3 tests)
  S.4  Expand RESERVED_ENV_KEYS             [ ] NONE risk — list expansion
  S.5  Windows sandbox (Job Objects)        [ ] MEDIUM risk — Windows API

MILESTONE 5 — Tooling 100%  (Layer 8)
  T.1  LSP publishDiagnostics               [x] DONE — lex+parse+type+lint; 5 tests
  T.2  DAP debug adapter                    [ ] HIGH risk — new subsystem
  T.3  Formatter idempotency certification  [ ] LOW risk — tests + fixes
  T.4  Linter 25+ rules                     [ ] LOW risk — additive rules
  T.5  Conditional breakpoints + bytecode   [ ] MEDIUM risk — debugger change
  V.3  Grammar.ebnf verification suite      [x] DONE — 9 doc fixes + 14 tests

MILESTONE 6 — Ecosystem 100%  (Layer 9)
  U.1  Binary releases CI                   [ ] LOW risk — CI config
  U.2  Install script                       [ ] LOW risk — shell script
  U.3  Web playground deployment            [ ] LOW risk — CI deploy
  U.4  Community documentation              [ ] LOW risk — mdBook
  U.5  Windows CI                           [ ] MEDIUM risk — platform compat
  U.6  Docker image                         [ ] LOW risk — Dockerfile
```

---

# TASK REGISTRY (v5.0 — all groups)

```
COMPLETED (770 tests; Layer 1–6=100%; P.3 CoW deferred):
  Group A  [x] B  [x] C  [x] D  [x] E  [x]
  Group F  [x] G  [x] H  [x] I  [x] J  [x]
  Group K  [x] L  [x] M  [x] N  [x]
  Group Q  [x] (Q.1 null-narrowing; Q.2 struct enforcement; Q.3 E0029; Q.4 Unknown audit)
  Group R  [x] (R.1 db_transaction; R.2 conn limit; R.3 stdlib audit; R.4 str_build)
  Group V  [x] (V.1 unicode escapes; V.2 assoc tests; V.3 grammar.ebnf 9 fixes + 14 tests)
  Group W  [x] (W.1 truncation; W.2 ?[; W.3 closures; W.4 dotted methods; W.5 tc wiring)
  O.1  [x] break/continue boundary fix     — Layer 4 correctness (+4 tests)
  O.2  [x] Module permission isolation     — Layer 5 security
  O.3  [x] Span tracking in runtime errors — Layer 3 DX
  O.4  [x] Per-task async timeout          — Layer 5 stability
  O.5  [x] Bytecode parity + optimizer     — Layer 4 architecture (+3 tests)
  P.1  [x] Stdlib dispatch O(1) HashMap    — Layer 5 performance (+2 tests)
  S.3  [x] Real memory limits (RSS)        — Layer 5 accuracy (+3 tests)
  T.1  [x] LSP publishDiagnostics          — Layer 3+8 DX (+5 tests)
  V.3  [x] Grammar.ebnf verification       — Layer 1+3 spec (+14 tests)

REMAINING (10 open):
  P.2  String interning Arc<str>            [x] DONE  — Layer 5, performance (+2 tests)
  P.3  Clone-on-write arrays/maps           [-] DEFER — Layer 5, HIGH risk
  P.4  Argument vector pooling              [x] DONE  — Layer 5, performance (+1 test)

  S.1  Plugin sandboxing                    [ ] HIGH  — Layer 7, security
  S.2  Registry manifest signing            [ ] MED   — Layer 7+9, security
  S.4  Expand RESERVED_ENV_KEYS             [ ] LOW   — Layer 7, security
  S.5  Windows sandbox                      [ ] LOW   — Layer 7, platform

  T.2  DAP debug adapter                    [ ] HIGH  — Layer 8, DX
  T.3  Formatter idempotency                [ ] MED   — Layer 8, stability
  T.4  Linter 25+ rules                     [ ] MED   — Layer 8, quality
  T.5  Conditional breakpoints + bytecode   [ ] LOW   — Layer 8, DX

  U.1  Binary releases CI                   [ ] HIGH  — Layer 9, release
  U.2  Install script                       [ ] HIGH  — Layer 9, release
  U.3  Web playground deployment            [ ] MED   — Layer 9, adoption
  U.4  Community documentation              [ ] HIGH  — Layer 9, adoption
  U.5  Windows CI                           [ ] MED   — Layer 9, platform
  U.6  Docker image                         [ ] LOW   — Layer 9, deployment

REMAINING TASKS:  17 open + 0 in-progress
CURRENT TESTS:    764 (599 integration + 165 unit)
TARGET TESTS:     ~820
```

---

# DEFERRED (see docs/deferred.md)

```
Registry server backend     — needs signed manifests (S.2) first
Self-update mechanism        — depends on U.1 binary releases
Package publishing workflow  — depends on U.1 and S.2
Community forum / Discord    — ecosystem, after tooling complete
Performance benchmark suite  — after P-group optimizations
Fuzzing CI                   — after core correctness (O-group) complete
JIT compilation              — after bytecode VM is default (O.5)
```

---

*This plan is the source of truth. Update status symbols after every task. Commit after every group.*
*Inside-out rule: do not start a layer until the layer below it is stable.*
*Last reviewed: 2026-03-21 — Group W complete; Milestones 1+3 verified.*
*Current completion: Layer 1: 100% | 2: 100% | 3: 100% | 4: 100% | 5: 82% | 6: 95% | 7: 86% | 8: 82% | 9: 15%*
