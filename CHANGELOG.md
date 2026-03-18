# Changelog

All notable changes to Txt-code are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- **`str_format(template, args...)` / `format(template, args...)`** — `{}` sequential and `{N}` positional placeholder formatting. `str_format("{} + {} = {}", 1, 2, 3)` → `"1 + 2 = 3"`.
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
