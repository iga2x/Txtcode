# Txtcode Development Roadmap

**Last Updated:** 2026-03-20
**Current Version:** 1.0.0
**Focus:** Secure automation scripting — the auditable, permission-controlled alternative to Bash/Python for system and cyber work.

---

## Strategic Direction

Txtcode's unique value is its **permission system + audit trail + safe execution model** — things Bash lacks entirely and Python bolts on as afterthoughts. The roadmap below builds on that foundation to make Txtcode **genuinely usable** for real-world scripting, not just impressive in demos.

> Every milestone below answers one question: *"Can a real developer ship something with this?"*

---

## Milestone Overview

| Milestone | Version | Theme | Blocker Fixes |
|---|---|---|---|
| **M1** | 1.1.0 | Stdlib Essentials | Regex, HTTPS, Time |
| **M2** | 1.2.0 | Real Async | True concurrency with tokio |
| **M3** | 1.3.0 | Data & Storage | SQLite, binary, bytes |
| **M4** | 1.4.0 | Engine Parity | Bytecode VM = AST VM |
| **M5** | 1.5.0 | Developer XP | LSP diagnostics, debugger step-through |
| **M6** | 2.0.0 | Platform | Live registry, plugin system, runtime types |

---

## Milestone 1 — Stdlib Essentials (v1.1.0)

**Goal:** A developer can write a real script that calls APIs, parses data, and handles text.
**Priority:** CRITICAL — these are day-one blockers.
**Target:** 2 weeks

### Tasks

#### 1.1 — Regex Support `[PRIORITY 1]`
- **Why:** Nearly every real script needs pattern matching. Without it, string processing is blocked.
- **What:** Add `regex` crate. Implement:
  - `regex_match(pattern, string)` → bool
  - `regex_find(pattern, string)` → string | null
  - `regex_find_all(pattern, string)` → array
  - `regex_replace(pattern, replacement, string)` → string
  - `regex_split(pattern, string)` → array
- **Files:** `src/stdlib/core.rs`, `Cargo.toml`
- **Done when:** `regex_match("[0-9]+", "abc123")` → true; 5 new tests pass.

#### 1.2 — HTTPS Client `[PRIORITY 2]`
- **Why:** Every real automation script hits an API. Current `http_get`/`http_post` has no TLS.
- **What:** Wire `reqwest` with TLS feature. Implement:
  - `http_get(url, headers?)` → `{status, body, headers}`
  - `http_post(url, body, headers?)` → `{status, body, headers}`
  - `http_put`, `http_delete`, `http_patch`
  - Custom headers as map argument
  - Timeout support
- **Files:** `src/stdlib/net.rs`, `Cargo.toml`
- **Permission:** requires `net.connect` grant (already exists)
- **Done when:** `http_get("https://api.github.com")` returns 200 body; 4 new tests pass.

#### 1.3 — Time & Date `[PRIORITY 3]`
- **Why:** Logging, scheduling, and file operations all need time. Current support is epoch-only.
- **What:** Add `chrono` crate. Implement:
  - `time_now()` → unix timestamp (already exists — keep)
  - `time_format(timestamp, format_string)` → string (e.g. `"%Y-%m-%d"`)
  - `time_parse(string, format_string)` → timestamp
  - `time_add(timestamp, seconds)` → timestamp
  - `time_diff(ts1, ts2)` → seconds
  - `time_utc()`, `time_local()`
- **Files:** `src/stdlib/core.rs`, `Cargo.toml`
- **Done when:** `time_format(time_now(), "%Y-%m-%d")` returns today's date; 4 tests pass.

#### 1.4 — Structured App Logging `[PRIORITY 4]`
- **Why:** Audit trail logs Txtcode-internal events. Scripts need their own log output.
- **What:** Implement:
  - `log_info(message)`, `log_warn(message)`, `log_error(message)`, `log_debug(message)`
  - Output format: `[LEVEL 2026-03-20T12:00:00Z] message`
  - `log_set_level(level)` — filter below level
  - `log_to_file(path)` — redirect to file (requires `fs.write`)
- **Files:** `src/stdlib/io.rs`
- **Done when:** `log_info("started")` prints formatted line; level filtering works; 3 tests pass.

---

## Milestone 2 — Real Async (v1.2.0)

**Goal:** `async`/`await` actually runs concurrently. Scripts can do parallel I/O.
**Priority:** HIGH — current fake async will mislead users writing concurrent code.
**Target:** 3 weeks after M1

### Tasks

#### 2.1 — Tokio Event Loop `[PRIORITY 1]`
- **Why:** Current async is sequential — `await` just runs the function. Not real concurrency.
- **What:**
  - Add `tokio` runtime to `Cargo.toml`
  - Replace `_async_executor: Option<()>` placeholder with real `tokio::runtime::Runtime`
  - `async_run(closure)` → spawns a tokio task
  - `await_all(array_of_futures)` → joins all, returns array of results
  - `async_sleep(ms)` — non-blocking sleep
- **Files:** `src/runtime/vm.rs`, `src/stdlib/core.rs`, `Cargo.toml`
- **Done when:** Two `async_run` blocks execute concurrently (measurable by timing); 4 tests pass.

#### 2.2 — Async HTTP `[PRIORITY 2]`
- **Why:** Parallel HTTP requests are the #1 use case for async in automation.
- **What:** Make `http_get`/`http_post` async-capable — when called inside `async_run`, they don't block the event loop.
- **Files:** `src/stdlib/net.rs`
- **Done when:** 5 parallel `http_get` calls complete in ~1x time, not 5x; test passes.

#### 2.3 — Channel Primitives `[PRIORITY 3]`
- **Why:** Tasks need to communicate results back.
- **What:**
  - `channel()` → `{send, recv}` pair
  - `chan_send(chan, value)` — non-blocking send
  - `chan_recv(chan)` — blocks until value arrives
  - `chan_try_recv(chan)` → value | null (non-blocking)
- **Files:** `src/stdlib/core.rs`, `src/runtime/core/value.rs`
- **Done when:** Producer/consumer pattern works across two `async_run` tasks; 3 tests pass.

---

## Milestone 3 — Data & Storage (v1.3.0)

**Goal:** Scripts can read/write databases and handle binary data.
**Priority:** HIGH — needed for any stateful application.
**Target:** 2 weeks after M2

### Tasks

#### 3.1 — SQLite Driver `[PRIORITY 1]`
- **Why:** The stub in `src/stdlib/db.rs` is incomplete. Real apps need persistent storage.
- **What:** Wire `rusqlite` fully:
  - `db_open(path)` → connection handle
  - `db_exec(conn, sql, params)` → array of row maps
  - `db_exec_one(conn, sql, params)` → single row map | null
  - `db_close(conn)`
  - `db_transaction(conn, closure)` — atomic block
  - Parameter binding via `?` — SQL injection impossible
- **Permission:** requires `fs.write` for path
- **Done when:** Create table → insert → select → delete round-trip works; SQL injection via params is impossible; 5 tests pass.

#### 3.2 — Binary / Bytes `[PRIORITY 2]`
- **Why:** Network protocols, file formats, and crypto all need byte-level work.
- **What:**
  - `bytes_from_string(s)` → byte array
  - `bytes_to_string(bytes)` → string (UTF-8)
  - `bytes_from_hex(hex_string)` → byte array
  - `bytes_to_hex(bytes)` → string
  - `bytes_slice(bytes, start, end)` → byte array
  - `bytes_concat(a, b)` → byte array
  - `bytes_len(bytes)` → int
- **Files:** `src/stdlib/core.rs`, `src/runtime/core/value.rs` (add `Value::Bytes`)
- **Done when:** Hex encode/decode round-trip works; 4 tests pass.

#### 3.3 — CSV Support `[PRIORITY 3]`
- **Why:** CSV is the most common data format in automation/scripting after JSON.
- **What:**
  - `csv_parse(string)` → array of row arrays
  - `csv_parse_with_headers(string)` → array of maps (first row = keys)
  - `csv_stringify(array_of_arrays)` → string
  - Handle quoted fields, commas in values, empty fields
- **Files:** `src/stdlib/core.rs`
- **Done when:** Parse/stringify round-trip for standard CSV; 3 tests pass.

---

## Milestone 4 — Engine Parity (v1.4.0)

**Goal:** Bytecode VM passes all tests that AST VM passes. Predictable behavior on both engines.
**Priority:** HIGH — current divergence makes bytecode VM unreliable.
**Target:** 3 weeks after M3

### Tasks

#### 4.1 — Parity Test Suite `[PRIORITY 1]`
- **Why:** No systematic comparison exists today.
- **What:**
  - Run all 465 tests against bytecode VM (`--engine=bytecode` flag)
  - Catalog all failures (expected: 30–80 failures)
  - Create `tests/parity/` — tests that must pass on both engines

#### 4.2 — Fix Top Divergences `[PRIORITY 2]`
- **Why:** Target zero divergence on core language features.
- **What:** Fix the top divergences found in 4.1, starting with:
  - Lambda capture in bytecode VM
  - `try/catch` signal propagation
  - Generator/yield in bytecode VM
  - Struct method dispatch (`impl` blocks)
- **Done when:** Bytecode VM passes 95%+ of AST VM tests.

#### 4.3 — Runtime Type Enforcement `[PRIORITY 3]`
- **Why:** Types are advisory only — `int x = "hello"` runs fine. Misleads users.
- **What:**
  - On variable assignment with explicit type annotation, validate at runtime
  - On function call, validate typed param types
  - Emit `RuntimeError(E0060)` on type mismatch (not just warning)
  - Flag: `--strict-types` already exists — make it the default
- **Done when:** `store → x: int → "hello"` fails at runtime with clear error; 5 tests pass.

---

## Milestone 5 — Developer XP (v1.5.0)

**Goal:** The editor experience is on par with any modern scripting language.
**Priority:** MEDIUM — needed for adoption, not for functionality.
**Target:** 2 weeks after M4

### Tasks

#### 5.1 — LSP Diagnostics Push `[PRIORITY 1]`
- **Why:** LSP exists but editors show no red squiggles while typing — the #1 user complaint.
- **What:**
  - Implement `textDocument/publishDiagnostics` notification
  - On `textDocument/didChange`, run type checker + linter on the document
  - Push diagnostics back to editor immediately
  - Map `ErrorCode` values to LSP `DiagnosticSeverity`
- **Files:** `src/cli/lsp.rs`
- **Done when:** VS Code shows error underlines for type mismatches and undefined variables in real time.

#### 5.2 — Debugger Step-Through `[PRIORITY 2]`
- **Why:** Current debugger sets breakpoints but has no `step`, `next`, `continue` commands.
- **What:**
  - `step` — execute one statement, pause
  - `next` — execute one statement, skip into function calls
  - `continue` — run until next breakpoint
  - `print <expr>` — evaluate expression in current scope
  - `stack` — print call stack
- **Files:** `src/cli/debug.rs`, `src/runtime/bytecode_vm.rs`
- **Done when:** Can step through a 20-line script line by line in the REPL debugger.

#### 5.3 — Formatter Improvements `[PRIORITY 3]`
- **Why:** Current formatter is basic — doesn't handle nested structures, comments, or alignment.
- **What:**
  - Consistent indentation for all block types
  - Preserve comments (currently stripped)
  - Align map literals
  - `txtcode format --check` exits 1 if file would change (CI use)
- **Done when:** Format is idempotent (format twice = same result); 5 formatter tests pass.

#### 5.4 — Error Messages `[PRIORITY 4]`
- **Why:** Current errors are often cryptic (`index out of bounds` with no location).
- **What:**
  - Every `RuntimeError` includes line number and column
  - Suggestion text for common mistakes: `did you mean X?`, `hint: use ?. for nullable access`
  - Diff between actual and expected types on mismatch
- **Done when:** Top 10 most common errors have line numbers and hints.

---

## Milestone 6 — Platform (v2.0.0)

**Goal:** Txtcode is a real platform: packages install from a live registry, plugins extend the runtime, and the type system is enforced end-to-end.
**Priority:** MEDIUM-LONG — adoption and ecosystem.
**Target:** 6 weeks after M5

### Tasks

#### 6.1 — Deploy Package Registry `[PRIORITY 1]`
- **Why:** `src/bin/registry_server.rs` exists but is not deployed. `txtcode package install` points at nothing live.
- **What:**
  - Deploy registry server (Docker image exists)
  - Point `registry/index.json` at live URL
  - `txtcode package publish` uploads and signs to live registry
  - Basic auth via API token (`txtcode package login`)
- **Done when:** `txtcode package install http-client` downloads and installs from live registry.

#### 6.2 — Plugin / Native Extension System `[PRIORITY 2]`
- **Why:** Users need to call native libraries that don't exist in stdlib.
- **What:**
  - `plugin_load(path)` — loads a `.so`/`.dylib` compiled Rust or C plugin
  - Plugin exports a `register(vm: &mut VM)` function that adds stdlib functions
  - Permission: `sys.ffi` (already exists) + path in allowlist
- **Done when:** A hello-world native plugin adds `hello()` function callable from Txtcode.

#### 6.3 — Full Generics (Monomorphization) `[PRIORITY 3]`
- **Why:** Generics are parsed and type-checked but erased at runtime — `List<int>` == `List`.
- **What:**
  - At call site, record concrete type substitution
  - Enforce generic constraints at runtime (not just check-time)
  - `where T: Comparable` → runtime validation that T supports `<`/`>`
- **Done when:** Passing a non-comparable type to a `<T: Comparable>` function → runtime error.

#### 6.4 — VS Code Extension `[PRIORITY 4]`
- **Why:** TextMate grammar exists but there's no installable VS Code extension.
- **What:**
  - Package `editors/` into a `.vsix` extension
  - Include LSP client that connects to `txtcode lsp`
  - Publish to VS Code Marketplace
- **Done when:** Extension installable from Marketplace; syntax highlighting + diagnostics work.

---

## Priority Matrix

| Task | Impact | Effort | Priority |
|---|---|---|---|
| 1.1 Regex | Very High | Low | **Do first** |
| 1.2 HTTPS client | Very High | Low | **Do first** |
| 1.3 Time/date | High | Low | **Do first** |
| 2.1 Real async (tokio) | High | High | Week 3 |
| 3.1 SQLite | High | Medium | Week 4 |
| 4.1–4.2 Bytecode parity | High | High | Week 5–6 |
| 4.3 Runtime types | High | Medium | Week 6 |
| 5.1 LSP diagnostics | Medium | Medium | Week 7 |
| 6.1 Live registry | Medium | Low | Week 8 |
| 5.2 Debugger step | Medium | Medium | Week 9 |
| 6.2 Plugin system | Medium | High | Week 10+ |
| 6.3 Real generics | Low | Very High | Later |
| 6.4 VS Code ext | Medium | Low | Week 10 |

---

## 2-Week Sprint Plan

### Week 1 — Stdlib Essentials
| Day | Task |
|---|---|
| Mon | Add `regex` crate; implement `regex_match`, `regex_find`, `regex_find_all` |
| Tue | `regex_replace`, `regex_split`; 5 tests; commit |
| Wed | Wire `reqwest` TLS; implement `http_get`/`http_post` with headers + timeout |
| Thu | `http_put`, `http_delete`, `http_patch`; permission check; 4 tests; commit |
| Fri | `time_format`, `time_parse`, `time_add`, `time_diff`; 4 tests; commit |

### Week 2 — Logging + Async Foundation
| Day | Task |
|---|---|
| Mon | Structured logging (`log_info`, `log_warn`, `log_error`, `log_set_level`, `log_to_file`) |
| Tue | 3 logging tests; update docs; commit |
| Wed | Add `tokio` to Cargo.toml; wire real runtime in `VirtualMachine` |
| Thu | `async_run`, `await_all`, `async_sleep`; basic concurrency test |
| Fri | Buffer/review all M1+M2-start changes; update CHANGELOG; tag `v1.1.0-alpha` |

---

## What to Stop Doing

1. **Adding new language features** — the language is complete enough. Ship working stdlib first.
2. **Expanding docs for unimplemented features** — docs ahead of code creates false impressions.
3. **Maintaining two VMs without parity** — every new feature added to the AST VM widens the gap.
4. **Fake async** — stop using `async` terminology for sequential code; it misleads users.
5. **Documenting the registry as live** — it's not deployed; remove live URL claims until it is.

---

## Definition of "Real Usable" (the finish line)

Txtcode is **genuinely usable** when a developer can:

- [ ] Write a script that calls a HTTPS API, parses JSON, stores results in SQLite — **M1+M3**
- [ ] Run 5 API calls in parallel with `async_run` + `await_all` — **M2**
- [ ] Get red squiggles in VS Code for type errors without running the script — **M5**
- [ ] `txtcode package install` a real package from the live registry — **M6**
- [ ] Step through a failing script in the debugger — **M5**

Hit those five checkboxes and Txtcode crosses from *experimental* to *usable*.
