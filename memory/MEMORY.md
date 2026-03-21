# NPL / Txt-code Project Memory

## Version: 1.0.0-release (ALL GROUPS COMPLETE) — 465 tests (128 unit + 337 integration)

## Dev Plan Status — COMPLETE
- Full plan at `docs/dev-plan.md`
- Groups 1–7: ALL [x] COMPLETE
- Group 8 (v0.5.1): Security Correctness — ALL 5 TASKS COMPLETE
- Group 9 (v0.6.0): Module System Overhaul — ALL 5 TASKS COMPLETE (209 tests)
- Group 10 (v0.6.5): Type System Promotion — ALL 3 TASKS COMPLETE (311 tests)
- Group 11 (v0.7.0): Developer Experience — ALL 5 TASKS COMPLETE (322 tests)
- Group 12 (v0.8.0): Platform & Compilation — ALL TASKS COMPLETE (335 tests)
- Group 13: Language Correctness — ALL TASKS COMPLETE (259 tests)
- Group 14 (v0.9.5): Language Completeness II — ALL 5 TASKS COMPLETE (277 tests)
- Group 15 (v1.0.0-alpha): Runtime & Async Overhaul — ALL 5 TASKS COMPLETE (291 tests)
- Group 16 (v1.0.0-beta): Networking & Security — ALL 5 TASKS COMPLETE (314 tests)
- Group 17 (v1.0.0-rc): Stdlib Application Layer — ALL 5 TASKS COMPLETE (328 tests)
- Group 18 (v1.0.0): Production Readiness — ALL 5 TASKS COMPLETE (465 tests)
- Group 19 (v1.0.0-release): Ecosystem & Platform — ALL 5 TASKS COMPLETE (465 tests)

## Group 15 Completed Tasks (v1.0.0-alpha)
- 15.1: Structured Concurrency (Nursery Pattern)
  - `async → nursery\n  nursery_spawn(task_fn)\nend` syntax
  - Added `Statement::Nursery { body, span }` to AST
- 15.2: Async Generators/Streams — `async define` + `yield` → Future<Array>; `async → for` auto-resolves
- 15.3: Timeout Primitives — `with_timeout(ms, fn)` → Result; `FutureHandle::resolve_with_timeout()`
- 15.4: Async File I/O — `async_read_file(path)`, `async_write_file(path, content)` in stdlib/io.rs (thread-based, no tokio)
- 15.5: docs/async.md — comprehensive async guide (10 sections, cheat sheet)
  - Added `"nursery"` to keywords.rs
  - `parse_nursery()` in `functions.rs`, detected in `parse_define()` when `async → nursery`
  - Thread-local `NURSERY_HANDLES: RefCell<Option<Vec<FutureHandle>>>` in `statements.rs`
  - `nursery_spawn(fn)` intercepted in `function_calls.rs` before stdlib
  - `spawn_for_nursery()` on `ExpressionVM` trait + `VirtualMachine` impl
  - Nursery in `vm.rs::execute_statement`: sets up TL, runs body, awaits all handles, propagates first child error
  - 4 new tests (281 total)

## Group 14 Completed Tasks (v0.9.5)
- 14.1: Full Generic Functions — TypeParam struct, constraint parsing <T: Comparable>, check_generic_call() in checker.rs
- 14.2: (from prev session)
- 14.3: Iterator Protocol — range/enumerate/zip/chain stdlib; __Range__/__Enumerate__/__Zip__/__Chain__ structs; execute_for handles struct iterators; call_struct_method on ControlFlowVM
- 14.4: (from prev session)
- 14.5: Generator Functions — `yield → value`; thread-local GENERATOR_COLLECTOR; generator call returns Value::Array(collected); stmt_contains_yield() helper

## Group 11 Completed Tasks (v0.7.0)
- 11.1: install.sh — binary download with SHA-256 verify, falls back to cargo build; .github/workflows/release.yml already existed
- 11.2: Bytecode debugger — debug_info Vec<(ip,line)> added to Bytecode; compiler records source lines; add_breakpoint_at_line()/source_line_for_ip() on Debugger; print_source_context() in debug.rs; fixed FunctionRef lambda dispatch bug in bytecode_vm.rs; 4 new unit tests
- 11.3: docgen.rs — DocItem struct, JSON/HTML/Markdown rendering, ## comment extraction with params/return types, generate_package_index(); --format json/html in CLI; 5 new unit tests
- 11.4: LSP — added textDocument/definition, hover, rename handlers; symbol_at/find_definition/hover_info/find_all_occurrences helpers; advertise capabilities in initialize; 6 new unit tests
- 11.5: REPL — persistent history to ~/.txtcode/repl_history (rustyline load/save); :reset alias for :clear

## Group 10 Completed Tasks (v0.6.5)
- 10.1: --no-type-check flag added; type checking runs by default; [WARNING] type: prefix; fixed false positives (unknown function → Unknown not Error)
- 10.2: check_collection_element_types() in checker.rs — Array<T>/Map<T> literal element enforcement; 2 new tests
- 10.3: Return type checking (current_return_type field), arity checking, null arithmetic warnings in checker.rs; 5 new tests

## Group 9 Completed Tasks (v0.6.0)
- 9.1: Module namespace isolation — isolated sub-VM scope, exported_symbols enforcement, circular import detection via import_stack
- 9.2: Transitive dependency resolver — resolve_transitive() in package.rs, registry-based topological sort, cycle detection, conflict detection
- 9.3: Lockfile enforcement — install_dependencies() loads lock before install, calls verify_installed() (compute_dir_hash), records real dir hashes; 6 new tests
- 9.4: Deterministic map iteration — Value::Map changed HashMap→IndexMap; all construction sites updated; insertion-order display; 1 new integration test
- 9.5: Docs updated — README/index.md: "permission-transparent execution"; language-spec.md: map[T] insertion-order note

## Group 8 Completed Tasks (v0.5.1)
- 8.1: exec_allowed default changed to false in VirtualMachine::new() and with_all_options()
- 8.2: const enforcement in bytecode VM (StoreConst instruction, const_vars HashSet, StoreVar guard)
- 8.3: CLI sign/verify/keygen + --require-sig flag (uses ScriptAuth from security/auth.rs)
- 8.4: FFI path allowlisting (sys.ffi permission in ffi_load, --allow-ffi PATH CLI flag)
- 8.5: Audit log persistence (--audit-log FILE writes AuditTrail.export_json() after execution)

## New Files (Groups 4–7)
- `src/cli/lsp.rs` — synchronous JSON-RPC LSP server (`txtcode lsp`)
- `editors/txtcode.tmLanguage.json` — TextMate grammar for .tc files
- `editors/txtcode-language-configuration.json` — VS Code language config
- `registry/index.json` — 20 packages with `local_path` field for offline install
- `packages/` — 20 core packages (4 existing + 16 new)
- `docs/performance.md` — benchmark results (real numbers from cargo bench)
- `benches/programs/` — 5 new benchmark programs (fib, array_ops, string_concat, json_ops, gc_alloc)

## Key Files
- `src/bin/txtcode.rs` — CLI (clap), version string, `apply_env_permissions()`
- `src/config.rs` — EnvConfig, RuntimeConfig (safe_mode, allow_exec, debug, verbose; NO quiet)
- `src/cli/env.rs` — env subcommands: init/install/use/status/list/clean/remove/doctor/diff/freeze/shell-hook/path
- `src/runtime/vm.rs` — AST VM (production); `src/runtime/bytecode_vm.rs` — Bytecode VM (experimental)
- `src/compiler/bytecode.rs` — BytecodeCompiler, LoopContext stack
- `src/stdlib/mod.rs` — `call_function_with_combined_traits` (upfront permission check)
- `src/runtime/permissions.rs` — PermissionResource: FileSystem(String), Network(String), Process(Vec<String>), System(String)
- `src/runtime/module.rs` — auto-injects .txtcode-env/{active}/packages/ into search_paths
- See `memory/stdlib.md` for all stdlib functions

## Architecture
- AST VM: full permissions/audit/policy. Used by `txtcode run`/`repl`. GC collect() called per statement.
- Bytecode VM: has PermissionManager, GC, `current_bytecode` field for lambda inline execution. HOF (map/filter/reduce/find) with bytecode lambdas handled via `call_hof_with_bytecode_lambda` + `call_lambda_inline`.
- Lambdas in bytecode: compiled as `RegisterFunction(name, params, ip)` + `PushConstant(Value::String(name))`. HOF interception before stdlib dispatch in Call handler detects `Value::String` -> registered lambda.
- Obfuscator: two-pass AST walk. Pass 1 collects user names, Pass 2 substitutes `_o0`, `_o1`, ... Stdlib names and `__`-prefixed names are NOT mangled. Wired in `cli/run.rs` via `Config::load_config().compiler.obfuscate`.
- VirtualEnv: `.txtcode-env/` auto-detected; `env.toml` per env; perms: "fs.read","net.connect","process.exec","sys.getenv"

## Async Architecture (Group 15)
- `FutureHandle` in `src/runtime/core/value.rs` — Arc<(Mutex<Option<Result>>, Condvar)>; thread-based, NOT Tokio
- `maybe_spawn_async()` in `vm.rs` spawns OS threads for async functions, returns `Value::Future(handle)`
- `await_all`/`await_any` in `stdlib/mod.rs` — block-join arrays of futures
- **Nursery** (Task 15.1): `NURSERY_HANDLES` thread-local in `statements.rs`; `nursery_spawn(fn)` intercepted in `function_calls.rs`; `spawn_for_nursery()` on ExpressionVM trait; `Statement::Nursery` handled in `vm.rs::execute_statement`

## Lexer: String Interpolation (IMPORTANT fix)
- Only `f"..."` strings support `{expr}` interpolation — regular `"..."` strings treat `{` literally
- `\{` still escapes the brace in any string type
- This was fixed in v0.6 dev; before the fix ALL strings with `{` were marked `InterpolatedString`

## Language Syntax (critical gotchas)
- Assignment: `store → x → value` (NOT `let x = value`; `=` is not an arrow)
- Index assign: `store → arr[0] → 99`
- For loop: `for → x in arr`
- Else-if: `elseif → condition` (single keyword)
- Try/catch: `try\n  body\ncatch e\n  handler\nend` (keyword-delimited, no braces)
- Struct def: `struct Point(x: int, y: int)` — parens NOT braces
- Ternary: `cond ? true_expr : false_expr`
- Pipe: `x |> func` → desugars to `func(x)`
- Compound assign: `x += 5`, `x -= 3`, etc.
- Raw strings: `r"\n"`, multiline: `"""..."""`, number sep: `1_000_000`
- Method on literal: `"hello".len()` → MethodCall; on identifier: `s.len()` → FunctionCall("s.len")
- Nursery: `async → nursery\n  nursery_spawn(task_fn)\nend`
- **assert is a FUNCTION CALL** — `assert(cond, "msg")` NOT `assert → cond → msg`

## Values & Instructions
- `Value::Result(bool, Box<Value>)` — Ok/Err
- Bytecode: SetupCatch/PopCatch/Throw/BuildOk/BuildErr/BuildStructLiteral(N), ImportModule(String)
- Type alias: `type → UserId → int` → stored as `__type_alias_<name>`
- Named error: `error → NotFound → "msg"` → stored as `__named_error_<name>`

## Key Security / Architecture Changes (7-phase plan)
- `src/runtime/security_pipeline.rs` — shared 6-layer security pipeline trait for both VMs
- `src/runtime/errors.rs` — `ErrorCode` enum (E0000–E0050), inferred from message + `.with_code()`
- `src/runtime/migration.rs` — `MigrationRegistry` + `MigrationPass` trait; `default_registry()` for v0.2→v0.4
- `src/runtime/compatibility.rs` — `Version` now derives `PartialOrd+Ord`; `current()` uses `env!("CARGO_PKG_VERSION")`
- `src/runtime/gc.rs` — rewritten as `AllocationTracker` (no raw ptrs); `GarbageCollector` type alias for compat
- `src/security/update_verifier.rs` — `verify_update_binary()` + `verify_sha256()` for `self_update`
- `src/security/integrity.rs` — HMAC-SHA256 replaces SHA-256(data||key||version)
- `async_executor` + `trace/` subsystems removed; `_async_executor: Option<()>` placeholder in VirtualMachine
- `src/cli/package.rs` — `verify_sha256_manifest()` called before tarball extraction; warnings if no manifest
- `scripts/sign_release.sh` + `.github/workflows/security.yml` + `.cargo/audit.toml` + `clippy.toml` added
- `fuzz/` directory with 5 libFuzzer targets: lexer, parser, bytecode, zip, env_file
- `txtcode run --type-check` / `--strict-types` flags wire `TypeChecker` as advisory pre-execution step

## CLI Flags (current)
- `run`: --timeout, --sandbox, --env-file, --no-color, --json, --allow-fs=PATH, --allow-net=HOST, --allow-ffi=PATH, --type-check, --strict-types, --require-sig, --audit-log=FILE
- `sign`: --key FILE --signer ID --output FILE
- `verify`: --sig FILE --trusted-key FILE
- `keygen`: --key-out FILE --pub-out FILE
- `format`: --check
- `lint`: --format json, --fix
- `test`: --watch
- `bench`: --save FILE, --compare FILE
- `package`: search QUERY, info NAME

## Safety / Limits
- MAX_CALL_DEPTH = 50
- Integer overflow: checked_add/sub/mul/pow in both VMs
- `?.`/`?[]`/`?()`  raise RuntimeError in bytecode VM

## Tests
- 281 total (after Group 15 Task 15.1, +4 nursery tests)
- Match patterns: `Pattern::Identifier("__literal_<value>")`
- Function def syntax: `define → name → (params)\n  body\nend`
- Lambda syntax: `(x) → x * 2` (NOT `fn(x) x * 2`)
- Top-level `return →` generates ReturnValue signal — use `interpret_repl()` to get last expr value

## CI / Clippy Notes (v0.4.1 fixes)
- `#![allow(clippy::result_large_err)]` at top of `src/lib.rs` — RuntimeError carries ControlFlowSignals (intentionally large)
- `ReturnValue` signal propagation: `call_user_function` has fast-path for top-level `return` + signal-catch for nested control flow
- `scope.rs:set_variable` has `#[allow(clippy::map_entry)]` — entry API breaks borrow check in the loop
- `inherent_to_string` → all types implement `std::fmt::Display` instead
- `module_inception` in `lexer/mod.rs` and `parser/mod.rs` suppressed with `#[allow]`
- `&Box<T>` in expression evaluators changed to `&T` (collections, member_access, operators, optional)
