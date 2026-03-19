# Txtcode Performance Baseline

> Measured on: Linux x86-64, release build (`cargo bench`)
> Date: 2026-03-19
> Txtcode version: 0.5.0
> CPU: x86-64 (single-threaded, no parallelism)

All times are median of 100 samples collected by [Criterion.rs](https://github.com/bheisler/criterion.rs).

---

## Lexer & Parser

| Benchmark | Time |
|-----------|------|
| `lexer/fib` — tokenise fibonacci program (~10 tokens) | **1.95 µs** |
| `lexer/loop` — tokenise loop program (~8 tokens) | **1.58 µs** |
| `parser/complex` — parse 100-iteration loop + 2 functions | **8.71 µs** |

**Takeaway:** The front-end (lex + parse) is fast. For a 1,000-line program, parsing takes ~1–2 ms.

---

## AST VM Execution

The AST VM is the tree-walking interpreter used by `txtcode repl`, `txtcode run`, and all tests.

| Benchmark | Time | Notes |
|-----------|------|-------|
| `vm/ast_loop` — while loop 1,000 iterations | **327 µs** | ~327 ns/iteration |
| `vm/ast_fib20` — recursive fibonacci(20) | **50.4 ms** | ~21,891 recursive calls |
| `vm/ast_array_ops` — map+filter+reduce on 100-element array | **138 µs** | ~1.4 µs per stdlib HOF call |
| `vm/ast_string_concat` — string `+` in loop × 500 | **282 µs** | ~564 ns/concat |
| `vm/ast_json_ops` — json_encode+decode loop × 100 | **318 µs** | ~3.2 µs per encode+decode round-trip |
| `vm/ast_gc_alloc_10k` — 10,000 map allocations | **5.76 ms** | ~576 ns/allocation |

### Rough per-operation costs (AST VM)

| Operation | Cost |
|-----------|------|
| Simple while iteration (load + compare + add + store) | ~327 ns |
| Function call overhead | ~2–5 µs (estimated) |
| Recursive call (fibonacci leaf) | ~2.3 µs |
| stdlib HOF (map/filter on one element) | ~1.4 µs |
| String concatenation | ~564 ns |
| JSON encode+decode (small object) | ~3.2 µs |
| Map literal allocation | ~576 ns |

---

## Bytecode VM Execution

The Bytecode VM (`txtcode compile` → `.txtc`) executes pre-compiled instruction streams.
It is currently **experimental** and not yet the default for `txtcode run`.

> Note: Bytecode benchmarks require `--features bytecode`.
> No bytecode numbers are included in this baseline; see roadmap below.

---

## Memory Management

Txtcode uses **Rust's RAII** for memory management — values are freed when they go out of scope,
with no separate garbage collection pause.

`AllocationTracker` in `src/runtime/gc.rs` counts live allocations and calls `collect()` periodically
as a hook point. Currently `collect()` is a **no-op suggestion**: it signals the tracker to reset its
counter, but Rust's ownership model handles the actual deallocation automatically.

**Implications:**
- No GC pause — memory is freed deterministically when scopes end
- 10,000 map allocations take **5.76 ms** total → **576 ns per allocation**, which includes the
  Txtcode interpreter overhead (variable lookup, scope push/pop) in addition to the Rust allocation itself
- Large programs with deep call stacks may stack-allocate more Rust frames; `MAX_CALL_DEPTH = 50`
  prevents stack overflow from unbounded recursion

---

## Scaling Estimates

Based on the benchmark data:

| Task | Estimated time |
|------|---------------|
| Parse 1,000-line program | ~1–2 ms |
| Run tight loop × 100,000 | ~33 ms |
| Recursive fib(25) | ~560 ms |
| Recursive fib(30) | ~6 s (impractical; use iterative) |
| Process 10,000 array elements (map) | ~1.4 ms |
| JSON encode 10,000 objects | ~32 ms |

---

## Roadmap: Performance Improvements

### v0.6 — Bytecode VM as Default
- `txtcode run` will use the Bytecode VM by default
- Expected speedup: **5–20×** for computation-heavy programs (loop iterations, arithmetic)
- Recursive functions may see smaller gains (call overhead dominates)

### v0.8 — `txtcode compile` → `.txtc` (Bytecode-Only Execution)
- Pre-compiled bytecode files skip the lex+parse step entirely
- Target: cold-start for a typical 500-line script < **5 ms** including VM setup
- AST VM kept for: REPL, debugging, `--type-check` mode

### Future
- String interning (reduce allocation pressure for repeated string keys in maps)
- Scope implemented as flat array instead of `HashMap<String, Value>` (reduces lookup cost)
- Tail-call optimization for recursive functions

---

## Reproducing These Results

```bash
# Run all benchmarks (release mode, 100 samples each)
cargo bench --bench benchmarks

# Run a specific benchmark
cargo bench --bench benchmarks -- vm/ast_fib20

# Save results to file
cargo bench --bench benchmarks 2>&1 | grep "time:" > bench_results.txt
```

Criterion saves HTML reports to `target/criterion/`. Open `target/criterion/report/index.html`
for interactive plots.
