# Async Programming in Txtcode

This guide covers every async feature available in Txtcode, from simple background tasks to structured concurrency, async streams, and timeouts.

---

## Overview

Txtcode's async model is **thread-based and cooperative**.  There is no Tokio or event-loop dependency — async functions execute in OS threads managed by Txtcode's `FutureHandle` primitive (a `Mutex`+`Condvar` pair).  This keeps the model simple and predictable: an async call always runs on a real thread, and `await` simply blocks the calling thread until the result is ready.

Key concepts:

| Concept | Keyword/Function | Returns |
|---------|-----------------|---------|
| Async function | `async define` | `Value::Future` |
| Await future | `await expr` | resolved value |
| Structured concurrency | `async → nursery` | — |
| Spawn task | `nursery_spawn(fn)` | — |
| Await many | `await_all(arr)` | array of results |
| First-one-wins | `await_any(arr)` | first result |
| Timeout guard | `with_timeout(ms, fn)` | `Result` |
| Async file read | `async_read_file(path)` | `Future<String>` |
| Async file write | `async_write_file(path, content)` | `Future<null>` |
| Async generator | `async define` + `yield` | `Future<Array>` |
| Async iteration | `async → for x in gen()` | — |

---

## 1. Async Functions

Mark any function definition with `async` to make it run in a background thread when called.  The call returns immediately with a `Future` value.

```
async define → fetch_data → ()
  store → result → http_get("https://example.com/api")
  return → result
end

store → future → fetch_data()
// other work happens here while the network request runs ...
store → data → await future      // blocks until done
print(data)
```

`await` can appear anywhere an expression is valid.  You can also inline it:

```
store → data → await fetch_data()
```

---

## 2. Awaiting Futures

`await expr` blocks the current thread until `expr` (a `Future`) resolves and returns its value.  If the async function threw an error, `await` re-raises it.

```
store → f → slow_compute()
// ... other work ...
store → result → await f
```

You can await any value; if it is not a `Future`, it is returned as-is.

---

## 3. `await_all` — Parallel Fan-out

Run multiple async functions concurrently and collect all results in order.

```
async define → task → (n)
  return → n * n
end

store → futures → [task(1), task(2), task(3)]
store → results → await_all(futures)
// results == [1, 4, 9]
print(results)
```

`await_all` waits for every future and returns an array in the same order as the input.  If any future errors, the first error is raised.

---

## 4. `await_any` — First-One-Wins

Return the result of whichever future resolves first (in evaluation order with the current thread-based implementation).

```
async define → fast → ()
  return → "fast"
end

async define → slow → ()
  sleep(500)
  return → "slow"
end

store → winner → await_any([fast(), slow()])
print(winner)    // "fast"
```

---

## 5. Structured Concurrency — Nursery

A `nursery` block provides structured concurrency: all tasks spawned inside it are **joined automatically** when the block exits.  If any child task fails, its error propagates after all tasks finish.

```
async → nursery
  nursery_spawn(() → heavy_work_a())
  nursery_spawn(() → heavy_work_b())
  nursery_spawn(() → heavy_work_c())
end
// All three tasks have completed (or one has failed) here.
```

### Rules

- `nursery_spawn(fn)` registers a zero-argument lambda to run as a background task.
- Tasks **cannot share mutable state** with the parent VM; they run in isolated child VMs.
- If any task raises an error, the nursery re-raises the **first** error after all tasks complete.
- The nursery body can do ordinary synchronous work between `nursery_spawn` calls.

### Error propagation

```
async → nursery
  nursery_spawn(() →
    store → x → 1 / 0
  )
end
// RuntimeError: division by zero  ← propagated from child task
```

---

## 6. Async Generators / Streams

An `async define` that also uses `yield` becomes an **async generator**.  Calling it returns a `Future` that resolves to an array of all yielded values.

```
async define → counter → (n)
  store → i → 0
  while → i < n
    yield → i
    i += 1
  end
end

store → stream → counter(5)
store → values → await stream
// values == [0, 1, 2, 3, 4]
```

### `async → for` — Streaming Iteration

Use `async → for` to iterate over an async generator without manually awaiting.  Txtcode auto-resolves the future before the loop body runs.

```
async define → numbers → ()
  yield → 10
  yield → 20
  yield → 30
end

store → total → 0
async → for → n in numbers()
  total += n
end
print(total)   // 60
```

`async → for` is syntactic sugar — internally the loop awaits the generator's Future and then iterates the resulting array.

---

## 7. Timeout and Cancellation

### `with_timeout(ms, fn)`

Runs a zero-argument lambda and returns a `Result`:
- `ok(value)` if the function completes within `ms` milliseconds.
- `err("timeout")` if the deadline is exceeded.

```
store → outcome → with_timeout(200, () →
  sleep(100)
  return → "done"
)

if → is_ok(outcome)
  print(unwrap(outcome))    // "done"
else
  print("timed out")
end
```

### Handling timeout errors

```
store → result → with_timeout(50, () →
  sleep(500)
  return → "never"
)

if → is_err(result)
  print("operation timed out — using default")
  store → result → ok("default_value")
end
```

### `sleep(ms)`

Pauses the current thread (or async task) for the given number of milliseconds.  Safe to use inside async functions and nursery tasks.

```
async define → delayed → ()
  sleep(100)
  return → "woke up"
end
```

---

## 8. Async File I/O

### `async_read_file(path)` → `Future<String>`

Reads a file on a background thread.  Permission checks run synchronously on the calling thread before spawning.

```
store → future → async_read_file("/etc/hostname")
store → content → await future
print(content)
```

### `async_write_file(path, content)` → `Future<null>`

Writes content to a file on a background thread.

```
store → w → async_write_file("/tmp/output.txt", "hello")
await w
print("write complete")
```

### Async write-then-read

```
await async_write_file("/tmp/data.txt", "42")
store → data → await async_read_file("/tmp/data.txt")
print(data)   // "42"
```

The synchronous equivalents `read_file` / `write_file` remain available for non-async contexts.

---

## 9. What Is NOT Async-Safe

The following operations should **not** be called concurrently from multiple nursery tasks or async functions targeting the same resources:

| Category | Concern |
|----------|---------|
| `write_file` / `append_file` (sync) | No locking — concurrent writes may interleave |
| `setenv` | Modifies process-global environment |
| `exec` / `exec_*` | Subprocess handles are not shared across VM boundaries |
| FFI (`ffi_load`, `ffi_call`) | Native library state is not isolated per child VM |
| `http_serve` | One server per process; only call from the main VM |
| Shared in-memory state | Child VMs start with a **snapshot** of the parent scope; mutations in child VMs do NOT propagate back to the parent |

For concurrent file access, use dedicated locking (`async → nursery` with sequenced writes) or a work-queue pattern.

---

## 10. Internals (for advanced users)

### FutureHandle

Futures are `Arc<(Mutex<Option<Result<Value, String>>>, Condvar)>`.  The spawned thread stores its result in the `Mutex`; the awaiting thread wakes via the `Condvar`.  No Tokio runtime is involved.

### Async generators under the hood

When an `async define` body also contains `yield`, the spawned thread activates `GENERATOR_COLLECTOR` (a thread-local `Vec`).  Each `yield` pushes a value into the collector.  When the function returns, the collected values are wrapped in `Value::Array` and sent via the `FutureSender`.

### Nursery internals

`NURSERY_HANDLES` is a thread-local `Vec<FutureHandle>`.  `nursery_spawn(fn)` appends a new handle.  On nursery exit, the VM drains the list and calls `resolve()` on each handle sequentially; the first error is collected and re-raised after all tasks complete.

---

## Quick-Reference Cheat Sheet

```
// Async function
async define → name → (params)
  ...
  return → value
end

// Await
store → x → await some_future

// Parallel fan-out
store → results → await_all([f1(), f2(), f3()])

// First result
store → winner → await_any([fast(), slow()])

// Structured concurrency
async → nursery
  nursery_spawn(() → task_a())
  nursery_spawn(() → task_b())
end

// Timeout
store → r → with_timeout(500, () →
  return → compute()
)

// Async generator
async define → gen → ()
  yield → 1
  yield → 2
end

// Async for
async → for → item in gen()
  print(item)
end

// Async I/O
store → content → await async_read_file("file.txt")
await async_write_file("out.txt", content)
```
