# Txt-code Programming Language

**Txt-code** © 2026 MD POOR

Txtcode is a **deterministic cyber orchestration DSL** - a policy-enforced execution engine designed for security-sensitive automation and experimentation. It provides a safe bridge between AI planning and real-world actions, with built-in audit trails and permission systems.

## License

This project is licensed under the **MIT License**. See [LICENSE](LICENSE) for details.

## Disclaimer

This software is provided "as-is" without warranty of any kind.  
By using Txt-code, you agree to respect the license and acknowledge MD POOR as the original author.

---

**Note:** "Txt-code" is the official name of this programming language.  
Unauthorized use of this name for competing products may infringe on intellectual property rights.

## Features

- **Cyber Orchestration**: Safe control of external pentest tools with permission enforcement
- **Execution Transparency**: Full trace logging and replayable execution graphs
- **Policy Enforcement**: Intent declarations, capability scoping, and rate limiting
- **AI-Safe Design**: Structured error output and deterministic execution for AI agents
- **Security-First**: Built-in audit trails, permission systems, and policy engines
- **Developer Tools**: REPL, formatter, linter, and execution tracer

> **Note**: See [NON-GOALS.md](NON-GOALS.md) for what Txtcode is NOT designed to do.

## Quick Start

```bash
# Build the project
cargo build --release

# Run a Txt-code program
./target/release/txtcode run examples/hello.txt

# Start REPL
./target/release/txtcode repl

# Compile to native binary
./target/release/txtcode compile examples/hello.txt -o hello
```

## Example Txt-code Program

```txtcode
# Hello World in Txt-code
print → "Hello, World!"

# Variables
store → name → "Alice"
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
repeat → 5 times
  print → "Count: " + count
end
```

## Project Structure

- `src/lexer/` - Tokenizer implementation
- `src/parser/` - AST builder
- `src/typecheck/` - Type system and checking
- `src/security/` - Obfuscation and protection
- `src/compiler/` - Code generation (bytecode, native, WASM)
- `src/runtime/` - Virtual machine and memory management
- `src/stdlib/` - Standard library modules
- `src/cli/` - Command-line tools
- `tools/` - Development tools (formatter, linter, debugger, docgen)
- `examples/` - Example programs
- `tests/` - Test suite
- `docs/` - Documentation

## Building from Source

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/txtcode/txtcode.git
cd txtcode

# Build
cargo build --release

# Run tests
cargo test

# Run examples
cargo run -- run examples/hello.txt
```

## Documentation

- [Language Specification](docs/language-spec.md)
- [Syntax Reference](docs/syntax-reference.md)
- [Security Features](docs/security-features.md)
- [Contributing Guide](docs/contributing.md)

## License Details

This project is licensed under the **MIT License**. See [LICENSE](LICENSE) for the full license text.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](docs/contributing.md) for guidelines.

