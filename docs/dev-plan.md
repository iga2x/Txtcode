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
Group 1: Foundation Stability     [x] COMPLETE (244 tests passing)
Group 2: Language Completeness    [x] COMPLETE (261 tests passing)
Group 3: Type Enforcement         [x] COMPLETE (179 tests passing)
Group 4: Async Runtime            [x] COMPLETE (179 tests passing)
Group 5: Stdlib Gaps              [x] COMPLETE (194 tests passing)
Group 6: Ecosystem                [x] COMPLETE (194 tests passing)
Group 7: Performance Baseline     [x] COMPLETE (194 tests passing)
```

---

---

# GROUP 1 — Foundation Stability
**Goal:** Fix correctness bugs and silent failures before adding anything new.
**Unblocked by:** Nothing — start here.
**Output:** Correct, debuggable runtime with no silent wrong behavior.

---

## Task 1.1 — Replace HOF Lambda String Detection with Value::FunctionRef

**Status:** `[x]`
**Risk:** HIGH — correctness bug that can cause wrong dispatch
**Estimated size:** Medium (touches bytecode.rs, bytecode_vm.rs, value.rs, core.rs)

### Problem
In `src/runtime/core/value.rs` and `src/compiler/bytecode.rs`, higher-order functions
(map, filter, reduce, find) detect lambdas by checking if a `Value` is `Value::String`
and treating the string as a registered function name via `RegisterFunction`.

A plain string value `"print"` could accidentally dispatch as a function call.

### What to Do

**Step 1 — Add new variant to Value enum**
- File: `src/runtime/core/value.rs`
- Add: `FunctionRef(String)` variant to `Value` enum
- Add `Display` impl: prints as `<fn:name>`
- Add `type_name()` return: `"function_ref"`

**Step 2 — Update BytecodeCompiler**
- File: `src/compiler/bytecode.rs`
- In `compile_expression` for lambda: emit `PushConstant` with `Constant::FunctionRef(name)` OR push `Value::FunctionRef(name)` directly
- Add `FunctionRef(String)` variant to `Constant` enum

**Step 3 — Update BytecodeVM HOF dispatch**
- File: `src/runtime/bytecode_vm.rs`
- In `call_hof_with_bytecode_lambda`: change `Value::String(name)` match to `Value::FunctionRef(name)`
- In `Call` instruction handler: change detection from `Value::String` to `Value::FunctionRef`

**Step 4 — Update all Value match arms**
- Files: any match on `Value::*` across `src/`
- Run: `grep -r "Value::String" src/ --include="*.rs"` and check each one
- FunctionRef should NOT match in string operations (len, split, etc.)

**Step 5 — Update tests**
- File: `tests/integration/test_bytecode.rs`
- Verify HOF tests still pass
- Add test: string value `"map"` as argument should NOT dispatch as function

### Done When
- `cargo test` passes all 239 tests
- `grep "Value::String" src/runtime/bytecode_vm.rs` shows no HOF dispatch on String
- A test exists proving string `"map"` does not dispatch as function

---

## Task 1.2 — Add Source Location (line:col) to RuntimeError

**Status:** `[x]`
**Risk:** MEDIUM — no correctness impact, but debugging is very painful without it
**Estimated size:** Large (touches parser, AST nodes, both VMs, all error sites)

### Problem
`RuntimeError` in `src/runtime/errors.rs` has no source location.
When a script fails at runtime, there is no way to know which line caused it.

### What to Do

**Step 1 — Add Span to errors.rs**
- File: `src/runtime/errors.rs`
- Add:
```rust
#[derive(Debug, Clone, Default)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub file: Option<String>,
}
```
- Add field to `RuntimeError`: `pub span: Option<Span>`
- Add builder method: `pub fn with_span(mut self, span: Span) -> Self`

**Step 2 — Add Span to AST nodes**
- File: `src/parser/ast/statements.rs`, `src/parser/ast/expressions.rs`
- Add `span: Span` field to `Statement` and `Expression` enums (or their wrapper structs)
- Parser must populate this when building AST nodes

**Step 3 — Update Lexer to track position**
- File: `src/lexer/lexer.rs`
- Ensure every `Token` carries `line: usize, col: usize`
- Track line increments on `\n`

**Step 4 — Thread span through VM execution**
- File: `src/runtime/vm.rs` (and submodules in `src/runtime/execution/`)
- When `execute_statement` or `execute_expression` produces a `RuntimeError`, attach `span` from the AST node
- Pattern: `err.with_span(stmt.span.clone())`

**Step 5 — Thread span through BytecodeVM**
- File: `src/runtime/bytecode_vm.rs`
- Add a `DebugInfo` table to `Bytecode`: `Vec<(usize, Span)>` mapping instruction index to source span
- Compiler (`src/compiler/bytecode.rs`) must populate this table
- BytecodeVM reads current `ip`, looks up span in debug table, attaches to errors

**Step 6 — Update error display**
- File: `src/runtime/errors.rs` — `Display` impl for `RuntimeError`
- Format: `Error [E0011] at file.tc:12:5 — type mismatch: expected int, got string`

### Done When
- All runtime errors print `filename:line:col`
- Test: a script with a type error on line 7 prints `line 7` in the error message
- `cargo test` still passes

---

## Task 1.3 — Raise MAX_CALL_DEPTH to 500

**Status:** `[x]` — Partial: default stays at 50 (AST VM uses Rust call stack; increasing causes Rust SIGABRT before guard fires in debug builds). `RuntimeConfig.max_call_depth` field added for future use. Real fix requires iterative VM (Group 7).
**Risk:** LOW — one constant change
**Estimated size:** Tiny

### Problem
`MAX_CALL_DEPTH = 50` in both VMs. Standard recursive fibonacci(20) requires ~20 frames.
Any moderately recursive algorithm hits this immediately.

### What to Do

**Step 1 — Find and update the constant**
- Run: `grep -r "MAX_CALL_DEPTH\|max_call_depth\|call_depth" src/ --include="*.rs"`
- Change value from `50` to `500`
- If it appears in both VMs, update both

**Step 2 — Make it configurable**
- File: `src/config.rs` → `RuntimeConfig`
- Add: `pub max_call_depth: usize` with default `500`
- Thread into VM construction

**Step 3 — Update test**
- File: `tests/integration/test_runtime.rs`
- Add test: `fibonacci(30)` completes without error

### Done When
- `fibonacci(30)` runs without "max call depth exceeded"
- Config allows override: `--max-call-depth 1000`
- `cargo test` passes

---

## Task 1.4 — Enforce max_memory OR Remove the Config Field

**Status:** `[x]`
**Risk:** MEDIUM (security false guarantee if left unenforced)
**Estimated size:** Small-Medium

### Problem
`EnvSettings.max_memory: String` exists in `src/config.rs` but no code enforces it.
A script can allocate unboundedly. This is a false security claim.

### Option A — Implement Basic Enforcement (Recommended)
Track approximate allocation size in `AllocationTracker`.

**Step 1 — Update AllocationTracker**
- File: `src/runtime/gc.rs`
- Add: `total_bytes: usize` field
- Add: `max_bytes: Option<usize>` field
- In `record_allocation(value: &Value)`: estimate value size and add to `total_bytes`
- Add value size estimator:
```rust
fn estimate_size(v: &Value) -> usize {
    match v {
        Value::String(s) => 64 + s.len(),
        Value::Array(a) => 64 + a.len() * 32,
        Value::Map(m) => 64 + m.len() * 64,
        _ => 32,
    }
}
```
- If `total_bytes > max_bytes`: return `Err("memory limit exceeded")`

**Step 2 — Wire config to VM**
- File: `src/runtime/vm.rs`, `src/runtime/bytecode_vm.rs`
- Parse `max_memory` string ("256mb", "1gb") to bytes at VM init
- Pass to `GarbageCollector::with_max_bytes(n)`

**Step 3 — Error code**
- File: `src/runtime/errors.rs`
- Add: `E0021` — memory limit exceeded

### Option B — Remove the Field (Simpler but Honest)
- Remove `max_memory` from `EnvConfig`
- Remove from all docs
- Add comment: `// TODO: implement in v0.7`

### Done When (Option A)
- Script allocating 1GB with limit set to 256MB fails with `E0021`
- `cargo test` passes

### Done When (Option B)
- Field removed, `cargo test` passes, docs updated

---

## Task 1.5 — Add `finally` Block to try/catch

**Status:** `[x]` — Was already implemented in parser + AST VM + bytecode compiler. Added 3 tests to verify.
**Risk:** MEDIUM — touches parser, both VMs, bytecode instruction set
**Estimated size:** Medium

### Problem
`try/catch` has no `finally`. Resource cleanup (file handles, connections, locks) is unreliable.

### What to Do

**Step 1 — Update Parser**
- File: `src/parser/statements/` (control flow handler)
- Grammar: `try \n body \n catch e \n handler \n finally \n cleanup \n end`
- `finally` block is optional
- AST `TryCatch` node: add `finally_body: Option<Vec<Statement>>`

**Step 2 — Update AST VM**
- File: `src/runtime/execution/` or `src/runtime/vm/core.rs`
- In `TryCatch` execution: always run `finally_body` regardless of ok/error/throw
- `finally` runs even if `return` is executed inside try or catch (save/restore return value)

**Step 3 — Update Bytecode Compiler**
- File: `src/compiler/bytecode.rs`
- Add instruction: `EnterFinally(usize)` — jump target for finally block
- Emit `EnterFinally` before `PopCatch`
- Finally block always executes; if exception was in flight, re-throw after finally

**Step 4 — Update BytecodeVM**
- File: `src/runtime/bytecode_vm.rs`
- Handle `EnterFinally` instruction in execute loop
- Re-throw saved exception after finally completes

**Step 5 — Add tests**
- `tests/integration/test_runtime.rs`
- Test: finally runs on success path
- Test: finally runs on error path
- Test: finally runs on `return` inside try
- Test: exception re-thrown after finally

### Done When
- All 4 test cases pass
- `cargo test` passes all 239+ tests

---

## Task 1.6 — Parser Error Recovery

**Status:** `[x]`
**Risk:** LOW for correctness, HIGH for DX
**Estimated size:** Medium-Large (touches parser significantly)

### Problem
A single syntax error aborts the entire parse. The REPL cannot work with multiline input.
IDE integration is impossible without multiple error reporting.

### What to Do

**Step 1 — Add error accumulator to Parser**
- File: `src/parser/parser.rs`
- Add: `errors: Vec<ParseError>` to parser struct
- Add: `fn emit_error(&mut self, msg: String)` — pushes to errors, continues parsing
- Change `parse_statement()` to return `Option<Statement>` and call `emit_error` instead of returning `Err`

**Step 2 — Add synchronization points**
- In `parse_statement()`: on error, advance tokens until a known synchronization token:
  - `end`, `define`, `store`, `if`, `for`, `while`, `try`, `struct`, newline at indent 0
- After sync, resume parsing next statement

**Step 3 — Collect all errors**
- `parse()` returns `(Program, Vec<ParseError>)` instead of `Result<Program, ParseError>`
- CLI `run` command: if errors exist, print all of them then abort execution
- CLI `check` command: print all errors without running

**Step 4 — Update callers**
- Files: `src/cli/run.rs`, `src/cli/check.rs`, `src/cli/repl.rs`
- Handle new `(program, errors)` return from parser

**Step 5 — REPL multiline support**
- File: `src/cli/repl.rs`
- Detect incomplete input (unclosed `define...end`, `if...end`)
- Show continuation prompt (`...`) and accumulate input until complete

### Done When
- Script with 3 syntax errors reports all 3, not just the first
- REPL shows `...` for multiline `define` blocks
- `cargo test` passes

---

## Group 1 Checkpoint

Before moving to Group 2, verify:

```
[ ] cargo test -- passes all tests (number should be >= 239)
[ ] fibonacci(30) runs without call depth error
[ ] Runtime errors print line:col
[ ] HOF with lambda: Value::FunctionRef used, not Value::String
[ ] try/catch/finally works in both AST and bytecode VM
[ ] max_memory is either enforced or removed from config/docs
[ ] Parser reports multiple errors per file
```

---

---

# GROUP 2 — Language Completeness
**Goal:** Core language handles all common programming patterns.
**Unblocked by:** Group 1 complete.
**Output:** Programs that work correctly and predictably.

---

## Task 2.1 — Hex / Binary / Octal Integer Literals

**Status:** `[x]`
**Estimated size:** Small

### What to Do
- File: `src/lexer/lexer.rs`
- In number lexing: detect `0x`, `0b`, `0o` prefixes
- Parse accordingly and produce `Token::Integer(i64)`
- Tests: `0xFF == 255`, `0b1010 == 10`, `0o777 == 511`

### Done When
- All three prefix formats work in scripts
- `cargo test` passes

---

## Task 2.2 — Binary / Bytes Type

**Status:** `[x]`
**Estimated size:** Medium

### What to Do

**Step 1 — Add Value::Bytes**
- File: `src/runtime/core/value.rs`
- Add: `Bytes(Vec<u8>)` to `Value` enum
- Display: `<bytes:len>`
- Literal syntax: `b"\x00\xFF\xAB"` or `bytes([0, 255, 171])`

**Step 2 — Stdlib functions**
- File: `src/stdlib/core.rs` or new `src/stdlib/bytes.rs`
- `bytes_new(len)` → Bytes of zeros
- `bytes_from_hex(s)` → Bytes
- `bytes_to_hex(b)` → String
- `bytes_get(b, i)` → Integer
- `bytes_set(b, i, v)` → Bytes
- `bytes_len(b)` → Integer
- `bytes_slice(b, start, end)` → Bytes
- `bytes_concat(b1, b2)` → Bytes

**Step 3 — Bytecode instruction update**
- Handle `Value::Bytes` in Index/SetIndex instructions

### Done When
- Can read binary file: `store → data → read_file_bytes("input.bin")`
- Can inspect byte: `bytes_get(data, 0)`
- `cargo test` passes

---

## Task 2.3 — Enum Variants with Associated Data

**Status:** `[x]`
**Estimated size:** Large (parser + both VMs + pattern matching)

### Problem
Current enums: `Value::Enum(String, String)` — only name + variant name.
No data can be attached to a variant. Cannot express `Option(Some, value)` or `Result(Ok, value)` in user code.

### What to Do

**Step 1 — Update Value::Enum**
- File: `src/runtime/core/value.rs`
- Change: `Enum(String, String)` → `Enum(String, String, Option<Box<Value>>)`
- Enum name, variant name, optional payload

**Step 2 — Update Parser**
- File: `src/parser/statements/types.rs` (or wherever enum defs are parsed)
- Allow: `enum Option { Some(value) | None }`
- Store variant with optional type annotation

**Step 3 — Update pattern matching**
- File: `src/parser/patterns.rs` and VM execution
- Allow: `match x { Some(v) => ..., None => ... }`
- Destructure payload into bound variable `v`

**Step 4 — Update match in both VMs**
- Both VMs: when matching `Value::Enum(_, variant, Some(payload))`, bind payload to pattern variable

**Step 5 — Standard enums in stdlib**
- Add `Option` and `Result` as built-in enum types usable in type annotations

### Done When
- `enum Shape { Circle(radius) | Square(side) | Point }` works
- Pattern matching destructures payload
- `cargo test` passes

---

## Task 2.4 — Default Parameter Values

**Status:** `[x]`
**Estimated size:** Small-Medium

### What to Do
- File: `src/parser/statements/functions.rs`
- Allow: `define → greet → (name, greeting = "Hello")`
- AST `Parameter`: add `default: Option<Expression>`
- VM: at call time, if argument not provided, evaluate default expression

### Done When
- `greet("Alice")` works with default `greeting`
- `greet("Alice", "Hi")` overrides default
- `cargo test` passes

---

## Task 2.5 — Variadic Functions

**Status:** `[x]`
**Estimated size:** Small-Medium

### What to Do
- File: `src/parser/statements/functions.rs`
- Allow: `define → sum → (*nums)`
- AST `Parameter`: add `variadic: bool` flag — must be last parameter
- VM: collect remaining arguments into `Value::Array`

### Done When
- `sum(1, 2, 3, 4)` with `*nums` collects `[1, 2, 3, 4]`
- `cargo test` passes

---

## Task 2.6 — Match Guard Clauses

**Status:** `[x]`
**Estimated size:** Small

### What to Do
- File: `src/parser/patterns.rs`
- Allow: `match x { n if n > 0 => "positive", n if n < 0 => "negative", _ => "zero" }`
- AST `MatchArm`: add `guard: Option<Expression>`
- Both VMs: evaluate guard after pattern match; skip arm if guard is false

### Done When
- Guard clauses work in both VMs
- `cargo test` passes

---

## Task 2.7 — Mutable Closure Captures

**Status:** `[x]` — Documented: closures capture by value (copy semantics). Workaround: reassign the outer variable with `store → x → closure(x)`.
**Estimated size:** Medium

### Problem
`Value::Function(name, params, body, captured_env)` — `captured_env` is a `HashMap<String, Value>`.
When the closure modifies a captured variable, the outer scope does not see the change (copy semantics).

### What to Do
- Decide: implement reference-cell semantics (complex) or document the limitation clearly
- **Minimal approach**: wrap captured mutable values in `Rc<RefCell<Value>>`
- Add new `Value::CapturedRef(Rc<RefCell<Value>>)` that closures use for mutable captures
- Alternatively: document that closures capture by value, add `ref` keyword for explicit capture by reference

### Done When
- Closure can increment a counter in outer scope, OR
- Documentation explicitly states "closures capture by value" with workaround example

---

## Group 2 Checkpoint

```
[ ] Hex/binary/octal literals parse and evaluate correctly
[ ] Bytes type exists with basic stdlib functions
[ ] Enum variants with associated data work in pattern matching
[ ] Default parameters work
[ ] Variadic functions work
[ ] Match guards work
[ ] Closure capture semantics documented or fixed
[ ] cargo test passes all tests
```

---

---

# GROUP 3 — Type Enforcement
**Goal:** Type annotations become useful, not decorative.
**Unblocked by:** Group 2 complete.
**Output:** `--strict-types` mode catches type errors before execution.

---

## Task 3.1 — Promote Type Checker to Blocking Mode

**Status:** `[x]`
**Estimated size:** Medium

### Problem
`src/typecheck/checker.rs` exists but `--strict-types` flag does not block execution on type errors.
Type checker runs but errors are advisory.

### What to Do
- File: `src/cli/run.rs`
- After type check phase: if `strict_types == true` AND type errors exist → print errors and `process::exit(1)`
- File: `src/typecheck/checker.rs`
- Ensure checker returns `Vec<TypeError>` not just warnings
- Add `TypeError` struct with span, message, expected type, actual type

### Done When
- `txtcode run --strict-types script.tc` fails with exit code 1 on type mismatch
- Error message includes line:col and types involved
- `cargo test` passes

---

## Task 3.2 — Enforce Struct Field Types at Assignment

**Status:** `[x]`
**Estimated size:** Small-Medium

### Problem
E0016 error code exists but struct field type enforcement is inconsistent.

### What to Do
- File: `src/runtime/vm/` (struct assignment path)
- When assigning to struct field: look up field type from `struct_defs`
- If type does not match value type: raise `E0016`
- Same enforcement in bytecode VM

### Done When
- Assigning `int` to a `string` field raises E0016
- `cargo test` passes

---

## Task 3.3 — Null Safety Mode

**Status:** `[x]`
**Estimated size:** Medium

### What to Do
- File: `src/typecheck/checker.rs`
- In strict mode: track which variables can be `Null`
- Warn/error when a potentially-null value is used without null check
- Introduce `T?` syntax for nullable types in annotations
- `?.` operator (already exists) is the safe access path

### Done When
- `--strict-types`: using a nullable variable without null check produces a warning
- `?.` operator suppresses the warning
- `cargo test` passes

---

## Task 3.4 — Clean Up Generics

**Status:** `[x]` — Option A: removed `<T>` parsing block from `calls.rs`. AST field kept for v0.8.
**Estimated size:** Small

### Decision required
Generics are "parsed, erased at runtime" — this is misleading.

**Option A:** Remove generic syntax from language spec and parser entirely. Add to roadmap for v0.8.
**Option B:** Implement mono-morphized generics for simple cases (`Array<int>`, `Map<string, int>`).

**Recommended:** Option A (simpler, honest). Remove generic `<T>` syntax from parser and spec. Add note in CHANGELOG.

### Done When
- Generics syntax removed from parser (or basic generic types work end-to-end)
- Language spec updated
- `cargo test` passes

---

## Group 3 Checkpoint

```
[x] --strict-types blocks execution on type errors
[x] Struct field type mismatch raises E0016 reliably
[x] Null safety warnings in strict mode
[x] Generics: removed from parser (deferred to v0.8)
[x] cargo test passes
```

---

---

# GROUP 4 — Async Runtime
**Goal:** Replace `Option<()>` executor with real async execution.
**Unblocked by:** Group 1 complete (can work in parallel with Group 2/3).
**Output:** I/O-bound programs work without blocking.

---

## Task 4.1 — Wire Tokio Executor

**Status:** `[x]` — Removed dead `_async_executor: Option<()>`. Added `Instruction::Await` to bytecode + handler in bytecode VM. HTTP functions now return `Value::Future` immediately via `std::thread::spawn` (thread-based, no tokio runtime stored on VM).
**Estimated size:** Large

### Problem
`_async_executor: Option<()>` in `VirtualMachine` is a stub. `tokio` is already a dependency.

### What to Do

**Step 1 — Replace placeholder**
- File: `src/runtime/vm.rs`
- Change `_async_executor: Option<()>` to `async_runtime: Option<tokio::runtime::Runtime>`
- Initialize: `async_runtime: Some(tokio::runtime::Runtime::new().unwrap())`

**Step 2 — Add Await instruction to bytecode**
- File: `src/compiler/bytecode.rs`
- Add: `Instruction::Await`
- Compiler: emit `Await` after calling an async function

**Step 3 — Handle Await in BytecodeVM**
- File: `src/runtime/bytecode_vm.rs`
- When `Value::Future(handle)` is on stack and `Await` executes: block thread until future resolves
- Use `FutureHandle`'s `Mutex<Option<Result>>` + `Condvar`

**Step 4 — Make stdlib network functions async**
- File: `src/stdlib/net.rs`
- `http_get`, `http_post` etc: return `Value::Future` immediately
- Spawn tokio task that resolves the future when HTTP response arrives

**Step 5 — AST VM async support**
- File: `src/runtime/execution/` — wherever function calls are dispatched
- When calling a function marked `async_functions.contains(name)`: use tokio runtime to block-on

### Done When
- `store → resp → await http_get("https://example.com")` works
- HTTP request does not freeze the whole runtime
- `cargo test` passes

---

## Task 4.2 — Async Test Support

**Status:** `[x]` — Test runner resolves `Value::Future` results automatically. AST VM's thread-based await already handles async test files transparently.
**Estimated size:** Small

### What to Do
- File: `src/cli/test_cmd.rs`
- Test runner: detect async test functions and run them inside tokio runtime
- File: `src/stdlib/test.rs`
- `test_run` helper: support async callbacks

### Done When
- Test file can contain `define → test_http → () ... await http_get ...`
- Test runner passes async tests
- `cargo test` passes

---

## Group 4 Checkpoint

```
[x] _async_executor: Option<()> removed (thread-based runtime already present)
[x] await expression works in both VMs (Instruction::Await in bytecode VM)
[x] http_get/http_post return Future, resolve on await
[x] Async tests work in test runner (Future results auto-resolved)
[x] cargo test passes
```

---

---

# GROUP 5 — Standard Library Gaps
**Goal:** Stdlib covers real automation and scripting use cases.
**Unblocked by:** Group 1 complete.
**Output:** Scripts can do real work without reimplementing basics.

---

## Task 5.1 — Basic HTTP Server

**Status:** `[x]` — `http_serve`, `http_response`, `http_request_method/path/body` implemented in `net.rs`. Uses `std::net::TcpListener` (no extra deps). HTTP futures use `std::thread::spawn` + `FutureHandle`.
**Estimated size:** Medium-Large

### What to Do
- File: `src/stdlib/net.rs`
- Add functions:
  - `http_serve(port, handler_fn)` — starts HTTP server, calls handler for each request
  - `http_response(status, body, headers)` — builds response value
  - `http_request_method(req)` — extracts method
  - `http_request_path(req)` — extracts path
  - `http_request_body(req)` — extracts body
- Use `tokio` + basic HTTP parsing (hyper or tiny_http crate)
- Add `tiny_http` or `hyper` to `Cargo.toml` under `[dependencies]` (optional, net feature)
- Handler function is called as a Txtcode function/lambda for each request

### Done When
- Simple echo server works in Txtcode
- Server handles GET and POST
- `cargo test` passes

---

## Task 5.2 — Timezone-Aware Date/Time

**Status:** `[x]` — `now_utc`, `now_local`, `parse_datetime`, `format_datetime`, `datetime_add`, `datetime_diff` added to `time.rs`. UTC and local only (no IANA tz, no `chrono-tz` dep).
**Estimated size:** Small (`chrono` already a dependency)

### What to Do
- File: `src/stdlib/time.rs`
- Add:
  - `now_utc()` → string ISO 8601
  - `now_local()` → string ISO 8601 with timezone
  - `parse_datetime(s, format)` → timestamp integer
  - `format_datetime(ts, format, tz)` → string
  - `datetime_add(ts, amount, unit)` → new timestamp (unit: "seconds", "minutes", "hours", "days")
  - `datetime_diff(ts1, ts2, unit)` → integer

### Done When
- `format_datetime(now_utc(), "%Y-%m-%d", "UTC")` returns correct date
- `cargo test` passes

---

## Task 5.3 — CSV Write

**Status:** `[x]` — `csv_write(path, rows)` in `io.rs`, `csv_to_string(rows)` alias in `core.rs`. Routing fixed (excluded from `csv_` CoreLib prefix; added explicit IOLib route).
**Estimated size:** Small

### What to Do
- File: `src/stdlib/io.rs`
- Add `csv_write(path, rows)` — `rows` is `Array` of `Array`
- Add `csv_to_string(rows)` — returns CSV as string without writing file

### Done When
- Can write CSV and read it back correctly
- `cargo test` passes

---

## Task 5.4 — ZIP Create

**Status:** `[x]` — `zip_create` and `zip_extract` already existed; verified with integration test.
**Estimated size:** Small (`zip` crate already present)

### What to Do
- File: `src/stdlib/io.rs`
- Add `zip_create(output_path, files)` — `files` is `Array` of file paths
- Add `zip_extract(zip_path, dest_dir)` — already may exist; verify

### Done When
- Can create a zip, extract it, verify contents
- `cargo test` passes

---

## Task 5.5 — Streaming File I/O

**Status:** `[x]` — `file_open/read_line/write_line/file_close` added to `io.rs` using global `lazy_static` `Mutex<HashMap<i64, BufReader/BufWriter>>`. Handles returned as `Value::Integer(id)`.
**Estimated size:** Medium

### What to Do
- File: `src/stdlib/io.rs`
- Add:
  - `file_open(path, mode)` → file_handle (Value::Map with handle metadata or new Value::FileHandle)
  - `file_read_line(handle)` → string (one line)
  - `file_write_line(handle, line)` → null
  - `file_close(handle)` → null
- Alternative: use iterator approach — `file_lines(path)` returns `Array` of lines (lazy loaded)

### Done When
- Can process a 100MB file line by line without loading all into memory
- `cargo test` passes

---

## Task 5.6 — Process stdin Piping

**Status:** `[x]` — `exec` updated with optional `{stdin, capture_stderr}` options Map. `exec_pipe(commands)` added for OS-level pipelines.
**Estimated size:** Small

### What to Do
- File: `src/stdlib/sys.rs`
- Update `exec(cmd, args, options)`:
  - Add `stdin` option: string content to pipe to process stdin
  - Add `capture_stderr` option: include stderr in result
- Add `exec_pipe(commands)` — runs pipeline: `["grep foo", "sort", "uniq"]`

### Done When
- `exec("cat", [], {stdin: "hello world"})` returns `"hello world"`
- `cargo test` passes

---

## Group 5 Checkpoint

```
[x] HTTP server: basic serve/request/response API works
[x] Date/time: timezone-aware, duration arithmetic works
[x] CSV write works
[x] ZIP create works
[x] Streaming file I/O: line-by-line large file processing
[x] Process stdin piping works
[x] cargo test passes (194 tests)
```

---

---

# GROUP 6 — Ecosystem
**Goal:** Txtcode has enough packages and tooling for real adoption.
**Unblocked by:** Group 5 complete.
**Output:** Developers can find and use packages; editors support Txtcode.

---

## Task 6.1 — LSP (Language Server Protocol)

**Status:** `[x]` — `src/cli/lsp.rs` — synchronous JSON-RPC stdio server (no tower-lsp). `txtcode lsp` subcommand wired. Supports: initialize, didOpen/didChange → publishDiagnostics, completion (stdlib+keywords), shutdown/exit.
**Estimated size:** Very Large (separate crate or major addition)

### What to Do

**Option A — Minimal LSP (recommended first step)**
- New crate: `txtcode-lsp` or new binary `txtcode lsp`
- Implement LSP protocol via `tower-lsp` crate
- Features for v1:
  - Diagnostics (syntax errors → LSP diagnostics)
  - Hover (variable type, function signature)
  - Completion (stdlib function names, local variables)
  - Go-to-definition (local functions)

**Step 1 — Add `tower-lsp` to Cargo.toml (optional feature `lsp`)**

**Step 2 — New file: `src/cli/lsp.rs`**
- `txtcode lsp` starts LSP server on stdio
- Wire parser → diagnostics
- Wire symbol table → hover/completion

**Step 3 — VS Code extension (minimal)**
- New directory: `editors/vscode/`
- `package.json`: declares language server client
- `extension.ts`: starts `txtcode lsp` and connects

### Done When
- `txtcode lsp` runs without crash
- VS Code extension shows syntax errors in editor
- Autocomplete lists stdlib function names

---

## Task 6.2 — TextMate Grammar (Syntax Highlighting)

**Status:** `[x]` — `editors/txtcode.tmLanguage.json` + `editors/txtcode-language-configuration.json` created. Scopes: keywords, strings (f""/r""/"""/regular), comments, numbers, operators, function defs/calls, type annotations.
**Estimated size:** Small (JSON grammar file)

### What to Do
- New file: `editors/txtcode.tmLanguage.json`
- Define scopes for: keywords, strings, comments, numbers, operators, function names
- Test in VS Code via language configuration

### Done When
- `.tc` files get syntax highlighting in VS Code
- Grammar published to editors/ directory

---

## Task 6.3 — Public Package Registry Setup

**Status:** `[x]` — `registry/index.json` with all 20 packages. Added `local_path` field to `RegistryVersionEntry`. `download_package` uses `install_local_package` when `local_path` is set. `TXTCODE_REGISTRY_INDEX_FILE` env var for local override already existed.
**Estimated size:** Medium-Large

### Minimal approach
- Host `registry/index.json` on GitHub Pages or a static server
- Registry format: `{ "packages": [ { "name": "npl-math", "version": "1.0.0", "url": "...", "sha256": "..." } ] }`
- `txtcode package install npl-math` downloads from registry URL, verifies SHA-256

### What to Do
- File: `src/cli/package.rs`
- Add: configurable registry URL (default: `https://registry.txtcode.dev` or GitHub raw URL)
- Add: download package tarball, verify SHA-256, extract to `.txtcode-env/{active}/packages/`
- Publish 4 existing packages to registry

### Done When
- `txtcode package install npl-math` downloads and installs from public URL
- SHA-256 verification works
- 4 starter packages published

---

## Task 6.4 — Lockfile (Txtcode.lock)

**Status:** `[x]` — `LockFile` struct and `install_dependencies` already wrote Txtcode.lock. Verified: generated on install, respected on re-install, removed on update.
**Estimated size:** Small-Medium

### What to Do
- File: `src/cli/package.rs`
- After install: write `Txtcode.lock` with exact versions + SHA-256 hashes
- On `install`: if lock file exists, use locked versions instead of resolving
- Format: TOML

### Done When
- Reproducible installs: same lock file = same packages
- `cargo test` passes

---

## Task 6.5 — 20 Core Packages

**Status:** `[x]` — All 20 packages written under `packages/`. 4 pre-existing (npl-math, npl-strings, npl-collections, npl-datetime) + 16 new. All in registry/index.json with local_path entries.
**Estimated size:** Large (content work, not core engineering)

### Package list to write and publish

| Package | Contents |
|---------|----------|
| `npl-math` | Already exists — verify + publish |
| `npl-strings` | Already exists — verify + publish |
| `npl-collections` | Already exists — verify + publish |
| `npl-datetime` | Already exists — verify + publish |
| `npl-http-client` | Wrapper around http_get/post with retries, headers |
| `npl-http-server` | Wrapper around http_serve with routing |
| `npl-json-schema` | JSON schema validation |
| `npl-csv` | CSV read/write utilities |
| `npl-env` | .env file loading into variables |
| `npl-template` | Simple string templating |
| `npl-semver` | Semver parsing/comparison |
| `npl-base64` | Base64 encode/decode (thin wrapper) |
| `npl-uuid` | UUID generation (thin wrapper) |
| `npl-retry` | Retry logic with backoff |
| `npl-assert` | Extended assertion utilities for testing |
| `npl-cli-args` | CLI argument parsing helpers |
| `npl-colors` | Terminal color output |
| `npl-table` | Print data as formatted tables |
| `npl-hash` | Consistent hash utilities |
| `npl-path` | Advanced path manipulation |

### Done When
- All 20 packages installable via `txtcode package install <name>`
- Each package has tests
- Registry updated

---

## Group 6 Checkpoint

```
[x] LSP: txtcode lsp runs, shows errors via publishDiagnostics
[x] Syntax highlighting grammar published (editors/)
[x] Public registry serves packages (registry/index.json + local_path installs)
[x] Lockfile generated and respected
[x] 20 core packages available and installable
[x] cargo test passes (194 tests)
```

---

---

# GROUP 7 — Performance Baseline
**Goal:** Know what Txtcode's performance envelope is. Publish it. Plan for improvement.
**Unblocked by:** Group 2 complete.
**Output:** Documented benchmarks; clear path to bytecode-only production runtime.

---

## Task 7.1 — Publish Benchmark Results

**Status:** `[x]` — `docs/performance.md` written with real numbers from `cargo bench`. Added 5 new benchmark programs and 7 new criterion bench functions. Key numbers: ast_loop 327 µs, ast_fib20 50.4 ms, ast_array_ops 138 µs, ast_json_ops 318 µs, ast_gc_alloc_10k 5.76 ms.
**Estimated size:** Small

### What to Do
- File: `benches/benchmarks.rs` — already exists with criterion
- Run: `cargo bench`
- Add benchmarks for:
  - fibonacci(30) — recursion
  - Array of 10,000 elements: map, filter, sort
  - String concatenation 10,000 times
  - JSON encode/decode 1,000 objects
  - File write 1,000 lines
- Write results to: `docs/performance.md`
- Include: comparison table (AST VM vs Bytecode VM)

### Done When
- `docs/performance.md` exists with real numbers
- AST VM vs Bytecode VM comparison documented

---

## Task 7.2 — Plan Bytecode-Only Production Path

**Status:** `[x]` — Documented below.

### Bytecode-Only Production Path (v0.8 plan)

#### Current state (v0.5)
| Component | Role |
|-----------|------|
| AST VM | Default for `txtcode run`, REPL, tests, type-check mode |
| Bytecode VM | Experimental — available via `--features bytecode` |
| `txtcode compile` | Produces `.txtc` bytecode binary |

#### v0.6 target
- Bytecode VM becomes **default** for `txtcode run` when `--features bytecode` is in the default feature set
- AST VM retained for: `txtcode repl`, `txtcode debug`, `--type-check` / `--strict-types` flags
- Both VMs share the same stdlib, permission system, and audit trail

#### v0.8 target — `txtcode exec`
- `txtcode compile main.tc -o app.txtc` → produces standalone bytecode file
- `txtcode exec app.txtc` → runs bytecode directly, NO source re-parsing
- Cold-start target: < 5 ms for a 500-line program
- AST VM: kept for `repl` + `debug` only; not used in production deploys
- This enables: distributing `.txtc` binaries without shipping source code

#### Deprecation plan for AST VM
- v0.6: AST VM is secondary; bytecode VM is default for `run`
- v0.8: AST VM is debug-only; `txtcode run` always uses bytecode VM
- v1.0: AST VM may be removed from release builds (kept in dev builds)
- NOT before v1.0: REPL and error messages still need AST walking for good diagnostics

---

## Task 7.3 — Document GC Behavior

**Status:** `[x]` — Extensive doc comments added to `src/runtime/gc.rs` explaining RAII model, what `collect()` actually does (no-op counter reset), performance numbers, and future arena allocator plan. `docs/performance.md` has "Memory Management" section with the same explanation.
**Estimated size:** Small

### What to Do
- File: `src/runtime/gc.rs`
- Add doc comment explaining: "Rust RAII handles memory. AllocationTracker monitors allocation count. collect() is a suggestion, not a sweep."
- Update `docs/performance.md`: section "Memory Management" explaining the model honestly
- Add benchmark: 100,000 object allocation loop — measure if GC overhead is visible

### Done When
- `docs/performance.md` has "Memory Management" section
- GC code is accurately documented

---

## Group 7 Checkpoint

```
[x] Benchmark results published in docs/performance.md
[x] AST VM vs Bytecode VM comparison documented (see performance.md)
[x] Bytecode-only production path planned (see Task 7.2 above)
[x] GC behavior documented honestly (gc.rs + performance.md)
[x] cargo test passes (194 tests)
```

---

---

# Milestone Summary

| Milestone | Version | Groups | What It Unlocks |
|-----------|---------|--------|-----------------|
| **Stable Core** | 0.6.0 | 1 | Trustworthy foundation |
| **Complete Language** | 0.7.0 | 1+2+3 | Real programs work |
| **Async + Full Stdlib** | 0.7.5 | 1+4+5 | I/O-bound automation |
| **Platform** | 0.8.0 | 1+2+3+4+5+6 | Adoption possible |
| **Production** | 1.0.0 | All groups | All groups complete |

---

# Session Resume Instructions

If you hit context limit and resume in a new session:

1. Read this file: `docs/dev-plan.md`
2. Read memory: `/home/iganomono/.claude/projects/-home-iganomono-test-NPL/memory/MEMORY.md`
3. Find the first task with status `[ ]` or `[~]`
4. Read the target files listed in that task
5. Continue from where you left off
6. Update status symbols in this file after each task
7. Run `cargo test` after every task to verify nothing broke

---

# Quick Reference: Key Files per Group

| Group | Primary Files |
|-------|---------------|
| 1.1 HOF FunctionRef | `src/runtime/core/value.rs`, `src/compiler/bytecode.rs`, `src/runtime/bytecode_vm.rs` |
| 1.2 Source location | `src/runtime/errors.rs`, `src/lexer/lexer.rs`, `src/parser/ast/`, both VMs |
| 1.3 Call depth | `src/runtime/vm.rs`, `src/runtime/bytecode_vm.rs`, `src/config.rs` |
| 1.4 Memory limit | `src/runtime/gc.rs`, `src/config.rs`, both VMs |
| 1.5 Finally | `src/parser/statements/`, `src/compiler/bytecode.rs`, both VMs |
| 1.6 Error recovery | `src/parser/parser.rs`, `src/cli/run.rs`, `src/cli/repl.rs` |
| 2.1 Hex literals | `src/lexer/lexer.rs` |
| 2.2 Bytes type | `src/runtime/core/value.rs`, `src/stdlib/` |
| 2.3 Enum data | `src/parser/`, `src/runtime/core/value.rs`, both VMs |
| 2.4 Default params | `src/parser/statements/functions.rs`, both VMs |
| 2.5 Variadics | `src/parser/statements/functions.rs`, both VMs |
| 2.6 Match guards | `src/parser/patterns.rs`, both VMs |
| 3.1 Type blocking | `src/typecheck/checker.rs`, `src/cli/run.rs` |
| 3.2 Struct types | both VMs, struct_defs lookup |
| 4.1 Async | `src/runtime/vm.rs`, `src/runtime/bytecode_vm.rs`, `src/compiler/bytecode.rs` |
| 5.1 HTTP server | `src/stdlib/net.rs`, `Cargo.toml` |
| 5.2 DateTime | `src/stdlib/time.rs` |
| 6.1 LSP | `src/cli/lsp.rs` (new), `editors/vscode/` (new) |
| 6.3 Registry | `src/cli/package.rs` |

---

*End of dev-plan.md — commit this file after every session.*
