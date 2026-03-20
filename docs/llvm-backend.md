# Txtcode Native Compilation Backend — Design & Planning

**Version:** Design for v0.9 prototype, v1.0 release
**Decision date:** 2026-03-19
**Status:** Planning (no code yet)

---

## Evaluation: inkwell (LLVM) vs Cranelift

### inkwell (LLVM Rust bindings)

| Criteria | Assessment |
|----------|------------|
| Maturity | High — LLVM is battle-tested, used by Clang/Rust/Swift |
| Performance | Excellent — best optimizing backend available |
| Rust integration | `inkwell` crate wraps LLVM C API via `llvm-sys` |
| Build complexity | **HIGH** — requires LLVM toolchain installed (version-pinned) |
| Binary size | Large — links in LLVM (~50–100 MB) |
| CI complexity | Requires LLVM build artifacts per platform |
| Debug builds | Slow — LLVM IR compilation itself is slow |

### Cranelift (Rust-native code generator)

| Criteria | Assessment |
|----------|------------|
| Maturity | Medium-High — used by Wasmtime, SpiderMonkey, Bytecode Alliance |
| Performance | Good — fast compilation, code quality ≈70–80% of LLVM at -O2 |
| Rust integration | Pure Rust — no C FFI, no external toolchain required |
| Build complexity | **LOW** — `cargo add cranelift-codegen cranelift-frontend cranelift-jit` |
| Binary size | Moderate (~5–10 MB) |
| CI complexity | Normal cargo build — works on all platforms |
| Debug builds | Fast — designed for near-instant JIT compilation |

---

## Recommendation: **Cranelift**

Cranelift is the correct choice for Txtcode for the following reasons:

1. **Zero external dependency**: Pure Rust. `cargo build` works out of the box without system LLVM.
2. **Alignment with ecosystem**: Cranelift powers Wasmtime (our WASM runtime) — unified backend.
3. **Developer ergonomics**: No version-pinning headaches, simpler CI, easier cross-compilation.
4. **JIT + AOT**: Cranelift supports both JIT (fast REPL execution) and AOT (standalone binaries).
5. **Performance target is achievable**: 10× improvement over bytecode VM requires eliminating interpretation overhead, not maximal optimization — Cranelift's code quality suffices.

LLVM may be revisited for v1.1+ if Txtcode needs auto-vectorization or aggressive inlining across module boundaries. At that point, the `NativeCompiler` trait abstraction allows swapping backends without changing calling code.

---

## Architecture: `src/compiler/native.rs`

```
txtcode source
     │
     ▼
  Lexer + Parser → Program (AST)
     │
     ▼
  BytecodeCompiler → Bytecode
     │
     ▼
  NativeCompiler (Cranelift) → machine code
     │
     ├── JIT path: executes in-process (for REPL acceleration)
     └── AOT path: writes ELF/Mach-O/PE binary (for standalone deployment)
```

### Core types

```rust
// src/compiler/native.rs

pub trait NativeBackend {
    fn compile(&mut self, bytecode: &Bytecode) -> Result<CompiledModule, NativeError>;
}

pub struct NativeCompiler {
    backend: Box<dyn NativeBackend>,
}

pub struct CraneliftBackend {
    isa: Arc<dyn cranelift_codegen::isa::TargetIsa>,
    module: cranelift_jit::JITModule,  // JIT path
    // or cranelift_object::ObjectModule for AOT
}

pub struct CompiledModule {
    pub entry_ptr: *const u8,   // JIT: function pointer to main
    pub binary: Option<Vec<u8>>, // AOT: object file bytes
}
```

### Instruction mapping (Bytecode → Cranelift IR)

| Bytecode Instruction | Cranelift IR |
|---------------------|--------------|
| `PushConstant(Integer(n))` | `ins.iconst(I64, n)` |
| `PushConstant(Float(f))` | `ins.f64const(f)` |
| `Add` | `ins.iadd(lhs, rhs)` |
| `Subtract` | `ins.isub(lhs, rhs)` |
| `Multiply` | `ins.imul(lhs, rhs)` |
| `Divide` | `ins.sdiv(lhs, rhs)` |
| `LoadVar(name)` | load from stack slot by variable index |
| `StoreVar(name)` | store to stack slot |
| `Call(name, n)` | `ins.call(func_ref, args)` |
| `Jump(ip)` | `ins.jump(block, &[])` |
| `JumpIfFalse(ip)` | `ins.brz(cond, false_block, &[])` |
| `Return` / `ReturnValue` | `ins.return_(&[value])` |

---

## v1.0 Scope

The first release of the native backend will support:

### Supported
- **Integers** (`i64`): arithmetic, comparison, bitwise
- **Floats** (`f64`): arithmetic, comparison
- **Strings**: heap-allocated via `libtxtcode_rt.a` runtime — passed as `(ptr, len)` pairs
- **Function calls**: direct (no dynamic dispatch), recursive
- **Basic control flow**: `if/while/for` (via Jump/JumpIfFalse)
- **Local variables**: Cranelift stack slots, indexed by name at compile time
- **Stdlib calls**: via C FFI into `libtxtcode_rt.a` runtime library

### Out of scope for v1.0
- Dynamic dispatch (method calls on unknown types)
- Closures / captured environments
- Arrays, Maps, Sets (require GC integration)
- Pattern matching / match expressions
- Exception handling (try/catch)
- Module imports

---

## Performance Target

**Goal**: 10× faster than bytecode VM for compute-heavy scripts.

Baseline (bytecode VM): `fib(35)` ≈ 850ms (from `docs/performance.md`).
Target (native): `fib(35)` ≤ 85ms — achievable with direct machine code elimination of interpretation overhead.

Compute-heavy benchmarks expected to benefit most:
- Fibonacci (function call overhead)
- Array sorting (loop + comparison overhead)
- String concatenation is NOT expected to improve (heap allocation dominates)

---

## Runtime Library: `libtxtcode_rt.a`

Stdlib functions that can't be inlined (I/O, networking, crypto) will be exposed
as a C-compatible static library. The native compiler links against it.

```c
// libtxtcode_rt.h (generated header)
typedef struct TcValue TcValue;  // opaque tagged union

TcValue tc_print(TcValue s);
TcValue tc_string_len(TcValue s);
TcValue tc_array_push(TcValue arr, TcValue elem);
TcValue tc_http_get(TcValue url);
// ...
```

---

## Timeline

| Milestone | Version | Deliverable |
|-----------|---------|-------------|
| Planning | v0.8.0 | This document |
| Prototype | v0.9.0 | `src/compiler/native.rs` JIT for integers + arithmetic + function calls |
| Beta | v0.9.5 | Strings, control flow, local vars; `txtcode compile --target native` |
| Release | v1.0.0 | Full integer/float/string support; `libtxtcode_rt.a` for stdlib |

---

## Decision Record

This decision has been recorded in CHANGELOG.md under the v0.8.0 entry:

> **Native compilation backend**: Selected Cranelift over LLVM for the v1.0 native
> compilation backend. Cranelift is pure Rust (zero C toolchain dependency),
> powers Wasmtime (our existing WASM runtime), and provides sufficient code quality
> for the 10× speedup target on compute-heavy workloads. LLVM may be evaluated
> for v1.1+ if vectorization or cross-module inlining becomes a requirement.
