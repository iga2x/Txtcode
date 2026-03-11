# Txt-code Programming Language v0.4

**Txt-code** © 2026 MD POOR — A security-first scripting language for safe automation, cyber orchestration, and AI-assisted operations.

Txtcode is a **deterministic cyber orchestration DSL** — a policy-enforced execution engine designed for security-sensitive automation and experimentation. It provides a safe bridge between AI planning and real-world actions, with built-in audit trails and permission systems.

> See [NON-GOALS.md](NON-GOALS.md) for what Txtcode is intentionally **not** designed to do.

---

## Features

- **Security-First Runtime** — Built-in audit trails, permission systems, and policy engines
- **Cyber Orchestration** — Safe, permission-controlled execution of external tools
- **Execution Transparency** — Full trace logging and replayable execution graphs
- **Policy Enforcement** — Intent declarations, capability scoping, and rate limiting
- **AI-Safe Design** — Structured error output and deterministic execution for AI agents
- **Developer Tooling** — REPL, formatter, linter, debugger, and execution tracer
- **Package Manager** — Built-in dependency management via `Txtcode.toml`

---

## Goals

- Safe, auditable automation of security-sensitive tasks
- Deterministic execution with predictable, reproducible results
- AI-compatible scripting with structured, machine-readable output
- Transparent policy enforcement with zero silent privilege escalation
- Bridging AI planning with real-world system actions safely

---

## Installation

### One-Line Install (Recommended — no sudo required)

```bash
curl -sSf https://raw.githubusercontent.com/iga2x/txtcode/main/install.sh | sh
```

What this does automatically:
- Detects your OS and CPU architecture
- Downloads a pre-built binary, or builds from source if none is available
- Installs to `~/.local/bin/txtcode` (no root/sudo needed)
- Creates `~/.txtcode/` for config, cache, and logs
- Adds `~/.local/bin` to your `PATH` in `.bashrc` / `.zshrc`

After install, restart your terminal or run `source ~/.bashrc`, then verify:

```bash
txtcode --version
txtcode repl
```

---

### Install from Source (Developers)

Requires [Rust](https://rustup.rs) stable.

```bash
# Install Rust if you don't have it
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/iga2x/txtcode.git
cd txtcode

# Install to ~/.local/bin  (no sudo)
make install-user

# Or install system-wide to /usr/local/bin  (needs sudo)
make install
```

---

### Windows

Download the pre-built binary from [GitHub Releases](https://github.com/iga2x/txtcode/releases) and add it to your `PATH`, or build from source with `cargo build --release`.

---

### Verify Installation

```bash
txtcode --version           # prints version
txtcode self info           # shows binary path, data size, project environments
```

---

## Quick Start

```bash
# Run a Txt-code program
txtcode examples/hello.tc
# or explicitly:
# txtcode run examples/hello.tc

# Start interactive REPL
txtcode repl

# Compile to bytecode
txtcode compile examples/hello.tc -o hello.txtc
# (Experimental / planned) Compile to native or WASM:
# native and WASM backends are not enabled in v0.4 builds yet.
# Future versions will support:
# txtcode compile examples/hello.txt -t native -o hello
```

---

## CLI Commands

```text
# Core execution
txtcode                        Start REPL (no args)
txtcode <file>                 Run a file (shortcut for txtcode run)
txtcode run <file>             Run a Txt-code program (with AST VM, full policy/audit)
txtcode repl                   Start interactive shell

# Compilation
txtcode compile <file> [opts]  Compile to bytecode (.txtc)
                               Note: emits bytecode only; native/WASM backends are planned.

# Formatting & linting
txtcode format <paths> [--write]  Format source files (in-place with --write)
txtcode lint <paths>              Run static analysis

# Debugging
txtcode debug <file>           Launch interactive debugger with breakpoints

# Packages
txtcode package init <name>    Initialize Txtcode.toml
txtcode package add <lib>      Add a dependency
txtcode package install        Install all dependencies
txtcode package update         Update dependencies
txtcode package list           List dependencies

# Projects & maintenance
txtcode init [name]            Initialize a new project scaffold
txtcode test [path]            Run tests (default: tests/)
txtcode doc [files]            Generate docs (default: src → docs/api)
txtcode bench <file>           Benchmark a program
txtcode doctor                 Check environment and ~/.txtcode setup
txtcode migrate [...]          Migrate code between Txt-code versions (dry-run by default)
```

### CLI behavior

- **Version and verbose info**:
  - `txtcode -V` / `txtcode --version` prints the version and exits.
  - `txtcode -v` (with no file/command) prints verbose build/platform info and exits.
- **REPL**:
  - `txtcode` with no arguments starts the REPL.
  - `txtcode repl` is the explicit form.
  - `Ctrl+C`, `Ctrl+D`, `exit`, or `quit` cleanly exit the REPL.
- **Safe mode**:
  - `txtcode --safe-mode run <file>` disables `exec()` and restricts process spawning.
  - `--allow-exec` overrides `--safe-mode` to re-enable process execution explicitly.
- **Compilation target**:
  - `txtcode compile` currently emits bytecode (`.txtc`) only.
  - Native (LLVM) and WASM backends are **planned** and not enabled in v0.4 builds.

> **Engine note:** `txtcode run` and the REPL use the high-level AST VM with full policy enforcement,
> permission checks, and audit logging. The bytecode VM (used internally by `compile`) is
> **experimental** in v0.4 and does not yet have full permission/audit parity. See
> [docs/language-spec.md](docs/language-spec.md) for details.

## Example Program

```txtcode
# Hello World
print → "Hello, World!"

# Variables
store → name → "Alice"
store → age → 25
print → "Hello, " + name

# Functions
define → greet → (name)
  return → "Hello, " + name
end

print → greet("World")

# Control flow (age is defined above)
if → age > 18
  print → "Adult"
else
  print → "Minor"
end

# Loops
store → count → 0
while → count < 5
  store → count → count + 1
  print → "Count: " + count
end
```

---

## Examples

The [`examples/`](examples/) directory contains ready-to-run programs:

| File | Description |
|------|-------------|
| [`hello.tc`](examples/hello.tc) | Hello World, variables, functions, control flow |
| [`calculator.tc`](examples/calculator.tc) | Arithmetic with pattern matching |
| [`file_processor.tc`](examples/file_processor.tc) | File read/write operations |
| [`port_scanner.tc`](examples/port_scanner.tc) | TCP port scanning (authorized hosts only) |
| [`security_demo.tc`](examples/security_demo.tc) | Hashing, encryption, and audit logging |
| [`web_server.tc`](examples/web_server.tc) | Simple HTTP server |

---

## Project Structure

```
src/
 ├── bin/          Entry point
 ├── lexer/        Tokenizer
 ├── parser/       AST builder
 ├── typecheck/    Type system and checking
 ├── security/     Obfuscation and protection
 ├── capability/   Capability and permission model
 ├── policy/       Policy engine
 ├── validator/    Input and runtime validation
 ├── compiler/     Code generation (bytecode; native/WASM planned)
 ├── runtime/      Virtual machine and memory management
 ├── stdlib/       Standard library modules
 ├── tools/        Formatter, linter, debugger, docgen
 └── cli/          Command-line interface

docs/             Language specification and guides
examples/         Example programs
tests/            Unit and integration test suite
release/          Pre-built binaries
```

---

## Uninstall

The uninstall command is built into the binary. Run:

```bash
txtcode self uninstall
```

You will be asked to choose one of three modes:

| Mode | What gets removed |
|------|-------------------|
| **1 — Binary only** (safest) | Just the `txtcode` binary. All your config, cache, and project files are kept. |
| **2 — Binary + global data** | Binary + `~/.txtcode/` (config, cache, logs, global packages). Project `.txtcode-env/` dirs are kept. |
| **3 — Complete wipe** | Everything above + all `.txtcode-env/` directories found under your home folder. Requires typing `DELETE ALL` to confirm. |

The uninstaller also removes the `txtcode` PATH entry from your `.bashrc` / `.zshrc`.

**If the binary is already gone or broken**, use the standalone shell script instead:

```bash
curl -sSf https://raw.githubusercontent.com/iga2x/txtcode/main/uninstall.sh | sh
```

Or if you cloned the repo:

```bash
./uninstall.sh
```

---

## Update

Check for and apply updates:

```bash
txtcode self update
```

This checks the latest release on GitHub and tells you if a newer version is available. To apply the update, re-run the one-line installer — it will replace the existing binary automatically:

```bash
curl -sSf https://raw.githubusercontent.com/iga2x/txtcode/main/install.sh | sh
```

---

## Building from Source

Requires [Rust](https://rustup.rs) (stable).

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/iga2x/txtcode.git
cd txtcode

# Build release binary
cargo build --release

# Run tests
cargo test

# Run an example directly
cargo run -- run examples/hello.tc
```

### Makefile targets

```bash
make install        # build + install to /usr/local/bin  (needs sudo)
make install-user   # build + install to ~/.local/bin    (no sudo)
make uninstall      # remove from /usr/local/bin
make uninstall-user # remove from ~/.local/bin
make test           # run test suite
make lint           # run clippy
make fmt            # run rustfmt
make clean          # remove build artefacts
```

---

## Documentation

- [Quick Start Guide](docs/quick-start.md)
- [Language Specification](docs/language-spec.md)
- [Syntax Reference](docs/syntax-reference.md)
- [Security Features](docs/security-features.md)
- [Contributing Guide](docs/contributing.md)

---

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](docs/contributing.md) before submitting a pull request.

---

## License

This project is licensed under the **MIT License**. See [LICENSE](LICENSE) for details.

---

## Disclaimer

This software is provided **as-is** without warranty of any kind.
By using Txt-code, you agree to respect the license and acknowledge **MD POOR** as the original author.

**"Txt-code"** is the official name of this programming language. The name and branding may not be used for misleading or competing derivative projects.
