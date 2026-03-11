# Changelog

All notable changes to Txt-code are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

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
