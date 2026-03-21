# Changelog

All notable changes to Txt-code are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [1.0.0] — 2026-03-20

**v1.0.0 — Production Release.** 465 tests passing.

### Added — Group 19 (Ecosystem & Platform)
- Community package registry server (`src/bin/registry_server.rs`) — HTTP API for package publish, search, download, and listing
- `registry/index.json` updated with live registry URL (`https://registry.txtcode.dev`)
- Official `Dockerfile` — multi-stage Alpine build; `docker run txtcode/txtcode:latest script.tc`
- `.github/workflows/docker.yml` — automated Docker image publishing to GHCR + Docker Hub on tag push
- `mkdocs.yml` updated with full v1.0 navigation structure for documentation site
- `docs/stability.md` — stability guarantees and semver policy for v1.0
- `CHANGELOG.md` — complete v1.0.0 entry

### Added — Group 18 (Production Readiness)
- `txtcode package publish` — validates, tarballs, signs, and POSTs packages to registry
- `txtcode package login` — stores API token to `~/.txtcode/credentials`
- Migration tool: `txtcode migrate [--check] <file>` — three source-level migration passes (string interpolation, assert syntax, yield arrow)
- `txtcode test --coverage` — line coverage tracking with `coverage/index.html` report
- `expect_error(result, pattern)` — new test assertion for error-path testing
- LSP workspace-wide symbol resolution — `textDocument/references`, `workspace/symbol`, cross-file `textDocument/definition`
- `txtcode bench --compare` — regression detection with 10% threshold, exits 1 on regression; `benches/baseline.json` committed
- `.github/workflows/bench.yml` — CI benchmark regression job on PRs

### Added — Group 17 (Stdlib Application Layer)
- SQLite integration: `db_open`, `db_exec`, `db_close` (bundled rusqlite)
- YAML/TOML aliases: `yaml_parse`, `yaml_stringify`, `toml_parse`, `toml_stringify`
- Template engine: `template_render` — Mustache-compatible `{{var}}`, `{{#if}}`, `{{#each}}`
- `cli_parse(args, spec)` — structured CLI argument parsing in scripts
- `proc_run(cmd, opts)` / `proc_pipe([cmds])` — process control with stdin/env/cwd/timeout

### Added — Group 16 (Networking & Security)
- TLS/HTTPS: `tls_connect(host, port)`, all HTTP clients use rustls-tls
- WebSocket: `ws_connect`, `ws_send`, `ws_recv`, `ws_close`, `ws_serve`
- Crypto: `crypto_sha256`, `crypto_hmac_sha256`, `crypto_aes_encrypt`, `crypto_aes_decrypt`
- JWT: `jwt_sign`, `jwt_verify`, `jwt_decode`
- DNS/Network: `dns_resolve`, `net_port_open`, `net_ping`

### Added — Group 15 (Runtime & Async Overhaul)
- Structured concurrency: `async → nursery { nursery_spawn(fn) }` pattern
- Async generators: `async define` + `yield →` → `Future<Array>`
- `with_timeout(ms, fn)` — async timeout primitive
- `async_read_file`, `async_write_file` — thread-based async file I/O
- `docs/async.md` — comprehensive async guide

### Added — Groups 1–14
See prior changelog entries for Groups 1–14 features.

---

## [0.8.0-dev] — 2026-03-19

Group 12 (Platform & Compilation) complete. 234 tests passing.

### Group 12 — Platform & Compilation

#### Task 12.1 — Async Event Loop

- **`await_all(futures_array)`** stdlib function — blocks until all futures in the array resolve; returns an array of results in the same order. Non-`Future` values are passed through unchanged (transparent, JavaScript-style).
- **`await_any(futures_array)`** stdlib function — returns the value of the first future in the array to resolve. Non-`Future` values are returned immediately.
- Both functions work with the existing `Value::Future` / OS thread mechanism introduced in v0.5.

#### Task 12.2 — Struct Methods (impl Blocks)

- **`impl → StructName` blocks** — define methods on a struct type using the new `impl` statement:
  ```
  impl → Point
    define → sum → (self)
      return → self.x + self.y
    end
  end
  ```
- Methods are called as `obj.method(args)`; `self` (the receiver) is auto-prepended by the runtime.
- Works in both the AST VM and the bytecode VM. Bytecode VM dispatches dotted-name calls to the struct method registry.
- `self` is a conventional first parameter, not a reserved keyword — keeps the parser simple.

#### Task 12.3 — WASM Compilation Target

- **`txtcode compile --target wasm script.tc`** produces `script.wat` (WebAssembly Text Format).
- Supported subset: integers, floats, booleans, arithmetic, comparisons, logical operators, local variables, and function calls.
- The produced `.wat` file can be converted to binary WASM with `wat2wasm script.wat -o script.wasm`.
- Feature-gated behind the `bytecode` Cargo feature (enabled by default).

#### Task 12.4 — LLVM Native Compilation Planning

- **`docs/llvm-backend.md`** — planning document evaluating `inkwell` (LLVM bindings) vs `cranelift` (Rust-native code generation). Recommends **Cranelift**: pure Rust, lighter, no LLVM toolchain required, already used by Wasmtime.
- Design sketch for `src/compiler/native.rs` — `NativeCompiler` that emits Cranelift IR from `Bytecode`.
- Scoped for v1.0: integers, floats, strings, direct function calls, basic control flow, stdlib via C FFI into `libtxtcode_rt.a`.
- Performance target: 10x faster than bytecode VM for compute-heavy scripts.

#### Task 12.5 — Or-Patterns and Range Patterns in Match

- **Or-patterns**: `1 | 2 | 3` syntax in `case` arms — matches if the value equals any of the listed patterns:
  ```
  match → x
    case → 1 | 2 | 3
      print → "low"
    case → _
      print → "other"
  end
  ```
- **Range patterns** (inclusive): `1..=5` syntax in `case` arms — matches if the value falls within the inclusive range:
  ```
  match → x
    case → 10..=20
      print → "medium"
    case → _
      print → "other"
  end
  ```
- Both patterns work in the AST VM and bytecode VM. Or-patterns and range patterns may be combined in the same match expression.

#### Task 12.6 — `?` Error Propagation Operator

- **Postfix `?` operator** on a `Result` value — ergonomic early-return on error:
  - If the value is `Err(e)`: immediately returns `Err(e)` from the enclosing function.
  - If the value is `Ok(v)`: unwraps to `v` and continues.
  - If the value is not a `Result`: passes through unchanged.
  ```
  define → risky → ()
    store → r → err("oops")
    store → v → r?    ;; early-returns Err("oops") if r is Err
    return → v
  end
  ```
- Implemented in both AST VM (`Expression::Propagate`) and bytecode VM (`Instruction::Propagate`).

---

## [0.5.1-dev] — 2026-03-19

All 7 development groups complete. This release covers Groups 4–7 additions on top of v0.5.0.

### Group 4 — Async Runtime

- **`Instruction::Await` in Bytecode VM** — `await expr` resolves `Value::Future` by blocking the calling thread; non-Future values pass through unchanged (JavaScript-style transparent await)
- **HTTP functions return `Value::Future`** — `http_get`, `http_post`, `http_put`, `http_delete`, `http_patch` now spawn a background OS thread and return a Future immediately; use `await` to block for the result
- **Async test support** — `txtcode test` now resolves `Value::Future` results automatically so async test functions work without manual `await`

### Group 5 — Standard Library Gaps

- **HTTP server** — `http_serve(port, handler)`, `http_response(status, body, headers)`, `http_request_method(req)`, `http_request_path(req)`, `http_request_body(req)`; implemented with `std::net::TcpListener` (no extra deps)
- **Timezone-aware datetime** — `now_utc()`, `now_local()`, `parse_datetime(s, fmt)`, `format_datetime(ts, fmt, tz)`, `datetime_add(ts, amount, unit)`, `datetime_diff(ts1, ts2, unit)`; uses `chrono`, supports UTC and local timezone
- **CSV write** — `csv_write(path, rows)` writes rows to file; `csv_to_string(rows)` returns CSV string (alias for `csv_encode`)
- **ZIP** — `zip_create` and `zip_extract` verified and covered by integration test
- **Streaming file I/O** — `file_open(path, mode)` → handle id; `file_read_line(handle)` → string or `null` at EOF; `file_write_line(handle, line)`; `file_close(handle)`. Handles stored in global `lazy_static` `Mutex<HashMap<i64, BufReader/BufWriter>>` registry
- **Process stdin piping** — `exec(cmd, {stdin: "...", capture_stderr: bool})` accepts options map; `exec_pipe(commands)` creates OS-level pipeline from array of command strings

### Group 6 — Ecosystem

- **LSP server** — `txtcode lsp` starts a synchronous JSON-RPC Language Server Protocol server on stdin/stdout. Supports: `initialize`, `textDocument/didOpen`, `textDocument/didChange` (→ `publishDiagnostics`), `textDocument/completion` (stdlib + keywords), `shutdown/exit`. No `tower-lsp` dependency.
- **TextMate grammar** — `editors/txtcode.tmLanguage.json` defines scopes for keywords, all string types (`f"..."`, `r"..."`, `"""..."""`), comments, numbers (hex/bin/float/int), operators (`→`, `|>`, `?.`, `?[]`), function definitions and calls, type annotations. `editors/txtcode-language-configuration.json` provides bracket matching and indent rules for VS Code.
- **Package registry** — `registry/index.json` with all 20 packages. New `local_path` field on `RegistryVersionEntry` allows installing directly from local directory (no tarball needed). `TXTCODE_REGISTRY_INDEX_FILE` env var for offline override already existed.
- **Lockfile** — `Txtcode.lock` written by `install_dependencies`, pinned on re-install, removed on `update` (already existed; verified).
- **20 core packages** — All packages installable via `txtcode package install <name>`:
  - Pre-existing: `npl-math`, `npl-strings`, `npl-collections`, `npl-datetime`
  - New: `npl-http-client`, `npl-http-server`, `npl-json-schema`, `npl-csv`, `npl-env`, `npl-template`, `npl-semver`, `npl-base64`, `npl-uuid`, `npl-retry`, `npl-assert`, `npl-cli-args`, `npl-colors`, `npl-table`, `npl-hash`, `npl-path`

### Group 7 — Performance Baseline

- **`docs/performance.md`** — real benchmark results from `cargo bench` (Criterion.rs): lexer ~2 µs, parser ~9 µs, loop×1000 ~327 µs, fib(20) ~50 ms, array_ops×100 ~138 µs, string_concat×500 ~282 µs, json_ops×100 ~318 µs, gc_alloc×10k ~5.76 ms. Includes scaling estimates and roadmap.
- **New benchmarks** — 5 new benchmark programs (`fib_ast.txt`, `array_ops.txt`, `string_concat.txt`, `json_ops.txt`, `gc_alloc.txt`) + 7 new criterion bench functions covering all major operations, including AST-vs-bytecode comparison
- **GC documentation** — `src/runtime/gc.rs` has comprehensive module-level doc comment explaining: RAII model, what `collect()` actually does (counter reset only, no sweep), measured allocation overhead, future arena allocator plan
- **Bytecode-only production path** — v0.6 plan (bytecode VM as default), v0.8 plan (`txtcode exec .txtc` bytecode-only execution), AST VM deprecation timeline documented in `docs/dev-plan.md`

### Fixed

- `csv_write` routing: was swallowed by `csv_` prefix → CoreLib; fixed with explicit exclusion in `src/stdlib/mod.rs`
- `csv_write` permission: now uses `FileSystem("write")` matching the pattern used by `write_file`
- `file_open` permission: uses `FileSystem("read")` or `FileSystem("write")` instead of path string
- Unused `Read` import removed from `src/cli/lsp.rs` — `{}` sequential and `{N}` positional placeholder formatting. `str_format("{} + {} = {}", 1, 2, 3)` → `"1 + 2 = 3"`.
- **`str_repeat(s, n)`** — repeat a string n times.
- **`str_contains(s, substr)`** — boolean membership test (cleaner than `indexOf() != -1`).
- **`str_chars(s)`** — split a string into an array of single-character strings.
- **`str_reverse(s)`** — reverse the characters of a string.
- **`str_center(s, width, pad_char?)`** — center-pad a string to the given width (default pad char `' '`).
- **`array_sum(arr)`** — sum all numeric elements; preserves int vs float.
- **`array_flatten(arr)`** — flatten one level of nesting.
- **`array_enumerate(arr)`** — produce `[[0, v0], [1, v1], ...]` for indexed iteration.
- **`array_zip(arr1, arr2)`** — produce `[[a0, b0], [a1, b1], ...]` pairs (stops at shorter array).
- **`array_contains(arr, val)`** — boolean membership test.
- **`array_push(arr, val)`** — return a new array with `val` appended.
- **`array_pop(arr)`** — return `[new_arr, last_element]`; errors on empty array.
- **`array_head(arr)`** — first element, or `null` for empty array.
- **`array_tail(arr)`** — all but the first element, or `[]` for empty array.

### Fixed
- **Lexer: `{` in plain strings no longer triggers interpolation** — only `f"..."` strings support `{expr}` interpolation. Regular `"..."` strings treat `{` and `}` as literal characters. This fixes a long-standing bug where writing `"{}"` in a string produced an interpolated empty expression instead of the two-character literal `{}`. Escaping with `\{` still works as before.

---

## [0.5.0] — 2026-03-18

### Summary

v0.5.0 is the **API and language-spec freeze** release. All seven planned phases are complete.
The public crate API, permission model, stdlib function names, and language syntax are now stable
for the v0.5 series. Breaking changes will be deferred to v0.6.0 and signalled in advance.

### Added
- **Package registry backend** (Phase 5-C) — `registry/index.json` is the single source of truth for all published packages. New `RegistryIndex` type (with `from_str`, `from_file`, `search`, `get_package`, `latest_version`). `PackageRegistry::load_index()` resolves in order: `TXTCODE_REGISTRY_INDEX_FILE` env var (local/test) → HTTPS fetch (net feature). `download_package` is un-stubbed — it looks up the version entry in the index, verifies SHA-256 (skipped when empty), then extracts the tarball. `txtcode package search` and `txtcode package info` now use the index instead of custom HTTP calls.
- **Real async/await execution** (Phase 6-B) — `async define → fn → (args)` now spawns an OS thread when called. The return value is `Value::Future`; `await expr` blocks the calling thread until the task completes. Non-future values passed to `await` are returned as-is (identity). The child thread receives a snapshot of the parent's global scope so it can call other user-defined functions and see global constants. Both AST VM and bytecode VM handle `Value::Future` in display/type/JSON.
- **Starter packages** (Phase 5-B) — four curated packages in `packages/` ship with the repo:
  - `npl-math@0.1.0` — GCD, LCM, primality, factorial, Fibonacci, clamp, lerp, mean, median, ipow
  - `npl-strings@0.1.0` — pad_left/right, center, repeat_str, truncate, words, count_substr, wrap
  - `npl-collections@0.1.0` — zip, flatten, chunk, unique, frequencies, take, drop, range, range_step
  - `npl-datetime@0.1.0` — timestamp, today, format_date, format_datetime, relative_time, elapsed_seconds, is_leap_year, days_in_month
- **`txtcode package install-local <PATH>`** — new subcommand that copies a local package directory (containing `Txtcode.toml`) into `~/.txtcode/packages/{name}/{version}/`; idempotent, safe against path traversal.
- **Semver resolver unit tests** — 11 tests covering `^`, `~`, `>=`, exact, non-semver, missing-package, and `get_installed_version`/`is_installed` paths in `DependencyResolver` (Phase 5-A).
- **`txtcode run --permissions-report`** — parse a `.tc` script, print every privileged permission it would request (grouped by permission string), then exit without running. Supports `--json` output.
- **`CapabilityResult` enum** (`Granted`, `NotFound`, `Revoked { token_id }`, `Expired { token_id }`) — replaces raw `bool` returns from `CapabilityManager::is_valid`. New `is_valid_detailed()` method returns typed result; `is_valid()` now delegates to it.  Both AST VM and Bytecode VM `use_capability()` now emit actionable denial reasons instead of generic "Invalid or expired" messages.
- **`RestrictionChecker::collect_privileged_calls_pub` / `required_capability_pub`** — public wrappers used by the permissions report feature.
- **Async function warning** — defining an `async` function now emits a `[WARNING]` explaining that it executes synchronously (E0051), preventing silent concurrency misexpectations.
- **FFI stdlib** (`ffi` feature, Phase 6-C) — load native shared libraries and call C functions from Txtcode scripts.
  `ffi_load(path)` opens a `.so`/`.dll` and returns an integer handle.
  `ffi_call(handle, fn_name, ret_type, args)` resolves the symbol and dispatches with 0–4 `i64` arguments; supported return types: `"int"` (i64), `"float"` (f64), `"void"` (null).
  `ffi_close(handle)` unloads the library.
  All three require the `sys.ffi` permission. Enabled with `cargo build --features ffi`; off by default.
  Names (`ffi_load`, `ffi_call`, `ffi_close`) are protected from obfuscation via the `ffi_` prefix.
- **Struct field assignment type checking** — `store → s["field"] → value` (bracket-notation struct field assign) now validates the value type against the struct definition. Unknown field writes and type mismatches emit `[WARNING]` in advisory mode, `E0016` error in strict mode.
- **`struct_defs()` / `strict_types()` added to `StatementVM` trait** — enables statement-level type enforcement consistent with expression-level checking.
- **`Value::type_name()`** — returns a human-readable type name string (`"int"`, `"float"`, `"string"`, etc.) for use in error messages.
- **Struct field type enforcement** — struct construction now validates field values against declared types (from `struct Point(x: int, y: int)` definitions).
  In advisory mode (default): type mismatches emit a `[WARNING]` to stderr and execution continues.
  In strict mode (`--strict-types`): type mismatches raise a hard `RuntimeError` with code `E0016`.
- **Unknown field detection** — constructing a struct with a field not declared in the struct definition emits a `[WARNING]` (or hard error in strict mode).
- **`ErrorCode::E0016`** (StructFieldTypeMismatch), **`E0051`** (AsyncWithoutExperimental), **`E0052`** (ExperimentalDisabled) added to the stable error code table.
- **`vm.set_strict_types(bool)`** — new setter on `VirtualMachine`; wired from `txtcode run --strict-types` into runtime struct type checking.
- **`xml_decode`** — canonical name for the XML parsing function. `xml_parse` is kept as a legacy alias (both names work).
- **`json_encode` / `json_decode`** — added to obfuscator's `STDLIB_NAMES` (were already the canonical names in the stdlib; now protected from mangling).

### Changed
- `run_file_with_allowlists` now accepts a `strict_types: bool` parameter and threads it into the VM.
- Obfuscator `STDLIB_NAMES`: removed stale `debug`/`info`/`warn`/`error` bare log aliases (removed from stdlib in v0.4.1); added `xml_decode`, `json_encode`, `json_decode`.

---

## [0.4.1] — 2026-03-18

### Removed
- **`PermissionResource::WiFi` and `PermissionResource::Bluetooth`** — removed from the permission model entirely.
  These variants were unimplemented placeholders with dangerous subcategories (`deauth`, `inject`, `fuzz`).
  Attempting to use `wifi.*` or `ble.*` permission strings now returns a clear error.
- **`pentest` feature flag** — removed from `Cargo.toml`. Was an empty placeholder with no implementation.
- **`WiFiCapability` / `BLECapability` structs** — removed from `src/capability/`. Files emptied; will be deleted in v0.5.0.
- **Bare logging aliases** — `debug`, `info`, `warn`, `error` removed from stdlib exports.
  Use `log_debug`, `log_info`, `log_warn`, `log_error` (canonical names, unchanged).
- **`random_bytes`, `random_int` in CryptoLib** — renamed to `crypto_random_bytes`, `crypto_random_int`
  to distinguish from `math_random_int` / `math_random_float` in CoreLib.
  Old names are gone. Update call sites to use the new names.

### Fixed
- **Package registry URL** — `PackageRegistry::download_package()` no longer silently 404s against a
  nonexistent GitHub org. Remote installs now return a clear error with instructions to use local path deps.
  Remote registry will be available in v0.7.0.
- **`LockFile` checksums** — removed `compute_checksum(name, version)` which hashed name strings instead
  of file content. `LockFile::add_package()` now requires actual tarball bytes and hashes those.
- **`tar` subprocess** — package tarball extraction no longer calls `std::process::Command::new("tar")`.
  Replaced with pure-Rust `flate2` + `tar` crates. Cross-platform, no external binary required.
  Path traversal (zip-slip) protection maintained on all entries.
- **Bytecode VM `grant_permission` / `deny_permission`** — fixed incorrect single-argument calls in
  `BcvmExecutor`; now correctly passes `(resource, scope)` tuple.

### Dependencies added
- `flate2 = "1.0"` — pure-Rust gzip decompression (replaces system `tar`)
- `tar = "0.4"` — pure-Rust tar archive reading (replaces system `tar`)

---

## [0.4.0] — 2026-03-11

### Virtual Environment System

- **`txtcode env` command** — full project-local environment management with 12 subcommands:
  `init`, `install`, `use`, `status`, `list`, `clean`, `remove`, `doctor`, `diff`, `freeze`, `shell-hook`, `path`
- **Auto-detection** — walks up from cwd to find `.txtcode-env/`; no manual activation needed
- **Named environments** — dev / prod / test / sandbox presets or custom names; `active` file tracks current
- **env.toml** — per-env TOML config with `[env]`, `[permissions]`, `[settings]` sections
- **Permission wiring** — `allow`/`deny` lists from `env.toml` are applied to `VirtualMachine` before every `run` or `repl` invocation; env `safe_mode = true` propagates to VM
- **Local package isolation** — `ModuleResolver` auto-prepends `.txtcode-env/{active}/packages/` to search paths
- **Shell hook** — `txtcode env shell-hook` outputs a bash/zsh/fish function for prompt integration

### Bytecode VM improvements

- **Pre-increment execute loop** — fixed a latent off-by-one bug where `Jump(loop_start)` after `ip += 1` would skip the first instruction of a while loop
- **`break`/`continue`** — `LoopContext` stack with `break_patches` / `continue_patches` correctly patches jump targets after loop compilation
- **`++`/`--` operators** — now compile to `LoadVar → PushConstant(1) → Add/Sub → Dup → StoreVar` sequences; no new instruction needed
- **`for x in arr`** — new `ForSetup(String, usize)` and `ForNext(usize)` instructions with a `for_iters` stack on `BytecodeVM`; handles empty arrays (body skipped) and multi-element arrays correctly
- **`repeat N`** — compiles to a counter-based while loop using hidden `__repeat_counter__` / `__repeat_limit__` variables
- **`match` statement** — bytecode compiler handles string/integer/boolean/null literal patterns and wildcard (`_`) with proper jump patching; `case → "+"` pattern correctly resolves `__literal_+` prefix
- **String interpolation** — `InterpolatedString` expression compiles to empty-string + successive `Add` instructions per segment
- **`try/catch/finally`** — body and finally always run; error interception not yet implemented in bytecode VM (sequential execution)
- **`const` statement** — compiles same as assignment (`StoreVar`)

### Safety improvements

- **Integer overflow guards** — all `i64` arithmetic (`+`, `-`, `*`, `**`) in both AST VM and bytecode VM now uses Rust's `checked_*` methods; returns a `RuntimeError` instead of wrapping silently
- **Recursion depth limit** — `call_user_function` and `call_function_value` both check `call_stack_depth() >= 100` and return a `RuntimeError` before the OS stack is exhausted
- **`call_stack_depth()` method** — added to `ExpressionVM` trait; implemented by `VirtualMachine`

### User-defined functions in bytecode VM

- **`RegisterFunction` instruction** — new `Instruction::RegisterFunction(name, params, body_start_ip)` emitted after each function definition; stores `(param_names, start_ip)` in `BytecodeVM.functions` HashMap at runtime
- **Jump-around compilation** — `FunctionDef` now emits `Jump(after_body)` before the body so normal top-level execution skips the function code; `RegisterFunction` is emitted after the body to wire the function up at runtime
- **Caller/callee scope isolation** — `Call` handler swaps out `self.variables` with `std::mem::take`, pushes `(return_ip, saved_vars)` onto the call stack, and binds args to params in a fresh scope; `Return`/`ReturnValue` restore saved variables on return
- **Recursion depth guard** — `Call` handler checks `self.call_stack.len() >= 100` and raises a `RuntimeError` before OS stack exhaustion

### Test additions

- 4 new bytecode control-flow tests: while loop, for loop over array, for loop over empty array, `++` operator
- 4 new safety tests: integer overflow on add, multiply, recursion depth limit, bytecode `match` with string cases
- 3 new bytecode user-function tests: single-param function, two-param function, bytecode recursion depth limit
- Total: **83 tests**, all passing

---

## [0.2.0] — 2026-03-07

### Summary

Release 0.2 is a hardening release focused on documentation accuracy, bytecode VM correctness,
permission model wiring, and developer clarity. No new user-facing language features are
introduced; the release ensures every documented feature either works or is explicitly marked
as not yet implemented.

### Added

- **`NullCoalesce` bytecode instruction** — `??` now emits a real `NullCoalesce` instruction
  instead of a silent `Nop`. The bytecode VM evaluates both sides and returns the non-null value.
- **Explicit bytecode VM errors for unimplemented ops** — `OptionalGetField`, `OptionalIndex`,
  `OptionalCall`, `SetIndex`, `SetField` now raise clear `RuntimeError` messages explaining
  the limitation and pointing users to `txtcode run` (AST VM).
- **Permission wiring in `call_function_with_combined_traits`** — stdlib calls for net (`http*`,
  `tcp*`, `resolve`), IO (`read*`, `write*`, `file*`, `delete`, `mkdir`, `rmdir`), process
  (`exec`, `spawn`, `kill`), and environment (`getenv`, `setenv`) now perform an upfront
  permission check via `PermissionChecker` before dispatch.
- **CLI `debug` command** — documented in README (was implemented but undocumented).
- **CLI behavior section in README** — clarifies `--safe-mode`, `--allow-exec`, version flags,
  REPL invocation, and engine note (AST VM vs bytecode VM).
- **Appendix D (Execution Engine Reference)** in language spec — explains the role of each
  engine, v0.2 limitations of BytecodeVM, and a usage decision table.
- **Appendix E (Migration Tooling Reference)** in language spec — documents supported
  transformations, current limitations (no source regeneration, no auto version detection),
  and planned v0.3 improvements.
- **Appendix F (Security Guarantees v0.2)** in language spec — tables of what is and is not
  enforced, safe mode guarantees, and the dual-mechanism permission model explanation.
- **Top-level not-yet-implemented summary** in language spec header — quick reference of
  features parsed but not fully executed in v0.2.

### Changed

- **Cargo.toml version**: `0.1.0` → `0.2.0`.
- **Binary `--version` string**: updated to `0.2.0`.
- **REPL banner**: updated to `Txt-code REPL v0.2.0`.
- **Language spec status**: updated from "v0.1 Pre-release" to "v0.2".
- **`++`/`--` documentation**: now clearly says "not yet implemented in v0.2; use `x + 1`".
- **`??` documentation**: now says "bytecode VM emits Nop; use explicit null check in bytecode".
- **`?.`/`?[]`/`?()` documentation**: now says "AST VM only; bytecode VM emits error".
- **README installation tarball**: updated from `v0.1.0` to `v0.2.0`.
- **Migration output**: when `--dry-run=false` but source regeneration is not available,
  a clear `[WARNING]` line is emitted explaining files were not modified.
- **`OptionalMember` / `OptionalCall` / `OptionalIndex` in compiler**: emit named instruction
  variants (`OptionalGetField`, `OptionalCall`, `OptionalIndex`) instead of anonymous `Nop`,
  enabling better error messages at runtime.

### Fixed

- **Pre-existing compilation error**: `user_config.runtime.quiet` field did not exist on
  `RuntimeConfig`; fixed by using `cli.quiet` directly (no config-file analogue).

### Known Limitations (v0.2)

- `txtcode compile` output (`.txtc`) does not enforce permissions or record audit events.
  Use `txtcode run` for all production execution.
- Source code regeneration (`txtcode migrate --dry-run=false`) is not implemented; files are
  validated but not modified.
- Version detection in `txtcode migrate` defaults to 0.1.0 when `--from` is not specified.
- Integer overflow on `i64` arithmetic is undefined; overflow guards are planned for v0.3.
- Call stack depth is unbounded; deep recursion causes OS stack overflow instead of a
  graceful RuntimeError.
- `for` loop iterator in bytecode VM is not implemented (iterable is evaluated and discarded).
- `break`/`continue` in bytecode VM emit `Nop` and do not affect control flow.

---

## [0.1.0] — 2026-01-15

### Added

- Initial release of the Txt-code programming language.
- Complete lexer, parser, and AST for the core language surface.
- AST VM (`VirtualMachine`) with:
  - Full expression evaluation (arithmetic, comparison, logical, bitwise)
  - Control flow (if/elseif/else, while, do-while, for, repeat, match)
  - Functions with defaults, variadic params, closures, generics
  - Async/await with `await_all` for parallel execution
  - Module system with circular import detection and caching
  - Try/catch/finally error handling
  - Capability declarations (`allowed`/`forbidden`)
  - Intent and AI-hint annotations
- Bytecode compiler (`BytecodeCompiler`) and bytecode VM (`BytecodeVM`) — experimental.
- Standard library: core, crypto, net, io, sys, time, json, regex, path, log, url, test, tools.
- Security infrastructure: permission system, capability model, policy engine, audit trail.
- Developer tools: formatter, linter (type + style), debugger (breakpoints, step), docgen.
- Package manager: `Txtcode.toml`, semver resolution, lockfile, GitHub releases registry.
- REPL with `rustyline` integration.
- CLI: run, repl, compile, format, lint, test, doc, bench, doctor, migrate, package, debug, init.
- Cross-compile CI setup for Linux and Windows.
