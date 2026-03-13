# Contributing to Txt-code

Thank you for your interest in contributing to Txt-code!

## Building from Source

```bash
# Clone the repository
git clone https://github.com/iga2x/txtcode.git
cd txtcode

# Build the project
cargo build --release

# Run tests
cargo test

# Run examples
cargo run -- run examples/hello.tc
```

## Architecture Overview

Txt-code is built in Rust and follows a modular architecture:

- **Lexer**: Tokenizes source code
- **Parser**: Builds Abstract Syntax Tree (AST)
- **Type Checker**: Performs type checking and inference
- **Compiler**: Compiles AST to bytecode (`.txtc`); native and WASM targets planned for v0.5
- **Runtime**: Executes programs
- **Standard Library**: Core functions and modules

## Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Use meaningful variable and function names
- Add comments for complex logic
- Write tests for new features
- Update documentation

## Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

## Submitting Changes

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Update documentation
6. Submit a pull request

## Development Roadmap

### v0.2 (released) — Security & stdlib hardening
- ✅ Virtual environment system (`.txtcode-env/`)
- ✅ Permission allowlists (`--allow-fs`, `--allow-net`)
- ✅ Extended stdlib: net (PUT/DELETE/PATCH/headers/status/timeout), io (read_lines/csv/temp_file/watch_file), sys (env_list/signal_send/pipe_exec/which/cpu_count/memory/disk_space), crypto (HMAC/UUID/PBKDF2/ed25519), math (clamp/lerp/gcd/lcm/factorial/combinations/random)
- ✅ Ternary, pipe operator, compound assignment, struct literals, type aliases, named errors
- ✅ Bytecode VM: permissions, module imports, closures, try-catch

### v0.3 (released) — Language completeness & quality
- ✅ f-string prefix support (`f"Hello {name}"`)
- ✅ Raw strings (`r"\n"` — no escape processing)
- ✅ Number separators (`1_000_000`)
- ✅ do-while loop in bytecode VM
- ✅ Optional chaining (`?.` `?[]` `?()`) in both VMs
- ✅ Ternary operator (`cond ? a : b`)
- ✅ Pipe operator (`x |> func`) — including lambda/complex RHS
- ✅ Spread operator (`[...arr]`) in both VMs
- ✅ Multi-return values (`return → a, b` — auto-wraps as array)
- ✅ Destructured function arguments (`define → f → ({x, y})`)
- ✅ `doc →` and `hint →` as canonical names for `intent →` / `ai_hint →`
- ✅ Pattern matching: array destructuring `[a, b]` and struct patterns `{x, y}`
- ✅ `++`/`--` prefix increment/decrement (identifier targets only)
- ✅ AST-to-source printer (migration file writing)
- ✅ Feature-gated stdlib: `zip`, `quick-xml`, `serde_yaml` (`--features full-stdlib`)
- ✅ `txtcode inspect file.txtc` — disassemble compiled bytecode
- ✅ `--target` validation (errors on unsupported native/wasm targets)
- ✅ Call depth aligned to 50 in all VMs
- ✅ async/await runs synchronously (non-blocking passthrough)

### v0.4 (released) — Virtual environments & bytecode completeness
- ✅ Virtual environment system (`txtcode env`) — 12 subcommands
- ✅ Bytecode VM: `break`/`continue`, `for x in arr`, `repeat N`, `match`, string interpolation
- ✅ Integer overflow guards in both VMs
- ✅ Recursion depth limit (50) in all VMs
- ✅ User-defined functions with scope isolation in bytecode VM
- ✅ Module imports (`ImportModule`) in bytecode VM

### v0.4.1 (released) — Security hardening & WiFi/BLE enforcement
- ✅ `PermissionResource::WiFi` and `PermissionResource::Bluetooth` — fully enforced in all check paths
- ✅ `wifi_*` / `ble_*` stdlib functions gated by permission system, audit trail, and validator
- ✅ Capability-adaptive `RuntimeSecurity`: Platform detection, SecurityLevel (None/Basic/Standard/Full)
- ✅ Anti-debug: 5-technique Linux stack (TracerPid + wchan + parent-process-name + timing + env scan)
- ✅ `security/auth.rs` — Ed25519 script signing/verification (ScriptAuth, ScriptSignature, KeyStore)
- ✅ `security/encryptor.rs` — PBKDF2-HMAC-SHA256 passphrase key derivation
- ✅ Source integrity SHA-256 hash verified at startup via `RuntimeSecurity`
- ✅ `.txtc` bytecode files now executable via `txtcode run` (routes to bytecode VM)
- ✅ Validator wired into `txtcode run`, `txtcode compile`, `txtcode check`
- ✅ `docs/permissions.md` — full permission and capability reference
- ✅ `docs/security-features.md` — accurate feature documentation (replaced fabricated content)
- ✅ All Clippy `-D warnings` issues resolved

### v0.5+ (planned)
- True async/await with Tokio runtime integration
- Native binary compilation (`-t native`) via LLVM
- WebAssembly compilation target
- WebSocket stdlib (`websocket_connect`)
- Bytecode VM: audit trail, policy engine, intent checking parity with AST VM
- Generic type enforcement at runtime
- AST identifier obfuscation (Obfuscator currently a no-op stub)
- macOS / Windows OS-level anti-debug checks

## Questions?

Feel free to open an issue or contact the maintainers.

