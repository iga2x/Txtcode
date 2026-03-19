# Txt-code Programming Language

**Txt-code** © 2026 MD POOR — A security-first scripting language for safe automation, cyber orchestration, and AI-assisted operations.

Txtcode is a **deterministic cyber orchestration DSL** — a policy-enforced execution engine designed for security-sensitive automation and experimentation. It provides a safe bridge between AI planning and real-world actions, with built-in audit trails and permission systems.

---

## Features

- **Security-First Runtime** — Built-in audit trails, permission systems, and policy engines
- **Cyber Orchestration** — Safe, permission-controlled execution of external tools
- **Execution Transparency** — Full trace logging and replayable execution graphs
- **Policy Enforcement** — Intent declarations, capability scoping, and rate limiting
- **AI-Safe Design** — Structured error output and deterministic execution for AI agents
- **Developer Tooling** — REPL, formatter, linter, debugger, LSP server (`txtcode lsp`), TextMate grammar
- **Package Manager** — 20 core packages, `registry/index.json`, `Txtcode.lock` lockfile
- **Async/Await** — thread-based `Value::Future`, `await` in both AST and Bytecode VMs
- **Full Stdlib** — HTTP server/client, datetime (UTC/local), CSV, streaming file I/O, process piping
- **Performance Documented** — see [docs/performance.md](performance.md) for real benchmark numbers

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

After install, restart your terminal or run `source ~/.bashrc`, then verify:

```bash
txtcode --version
txtcode repl
```

### Install from Source (Developers)

Requires [Rust](https://rustup.rs) stable.

```bash
git clone https://github.com/iga2x/txtcode.git
cd txtcode

# Install to ~/.local/bin  (no sudo)
make install-user

# Or install system-wide to /usr/local/bin  (needs sudo)
make install
```

---

## Quick Start

```bash
# Run a Txt-code program
txtcode examples/hello.tc

# Start interactive REPL
txtcode repl

# Compile to bytecode
txtcode compile examples/hello.tc -o hello.txtc
```

See the [Quick Start](quick-start.md) guide for a full walkthrough.

---

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

# Control flow
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

## CLI Commands

```text
txtcode                        Start REPL (no args)
txtcode <file>                 Run a file
txtcode run <file>             Run a Txt-code program (full policy/audit)
  --timeout 30s                Maximum execution time
  --sandbox                    Deny all fs writes, network, exec
  --allow-fs PATH              Permit filesystem access under PATH
  --allow-net HOST             Permit network access to HOST
  --type-check / --strict-types  Static type checker (advisory / hard errors)
  --permissions-report         List privileged calls without running
txtcode repl                   Start interactive shell
txtcode compile <file>         Compile to bytecode (.txtc)
txtcode lsp                    Start LSP server on stdin/stdout
txtcode format <paths>         Format source files (--write for in-place)
txtcode lint <paths>           Run static analysis
txtcode debug <file>           Launch interactive debugger
txtcode test [path]            Run tests (default: tests/)
txtcode bench <file>           Benchmark a program
txtcode doctor                 Check environment setup
txtcode init [name]            Initialize a new project
txtcode package install        Install dependencies
txtcode package install-local  Install from local directory
txtcode package search <query> Search the registry
```

> **Engine note:** Both `txtcode run` (AST VM) and compiled `.txtc` files (Bytecode VM) enforce
> the same full 6-layer security pipeline: intent checking, capability tokens, rate limiting,
> permission grants/denials, audit trail logging, and runtime integrity verification.

---

## Documentation

- [Quick Start Guide](quick-start.md)
- [Language Specification](language-spec.md)
- [Syntax Reference](syntax-reference.md)
- [Permissions Reference](permissions.md)
- [Security Features](security-features.md)
- [Performance Baseline](performance.md)
- [Contributing Guide](contributing.md)

---

## License

MIT License — see [LICENSE](https://github.com/iga2x/txtcode/blob/main/LICENSE) on GitHub.

**"Txt-code"** is the official name of this programming language. © 2026 MD POOR.
