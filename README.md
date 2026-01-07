# Txt-code Programming Language

Txt-code is a simple, memorable, security-focused programming language designed for both cybersecurity tools and general development. It features built-in obfuscation and reverse-engineering protection while maintaining an easy-to-learn syntax.

## Features

- **Simple Syntax**: Hybrid syntax supporting both `action → data` and `action data` patterns
- **Security-First**: Built-in obfuscation, bytecode encryption, and anti-debugging protection
- **Memory Safe**: Garbage-collected runtime with secure memory management
- **Cross-Platform**: Compile to native code, WebAssembly, or bytecode
- **Rich Standard Library**: Core utilities, cryptography, networking, I/O, and system operations
- **Developer Tools**: REPL, formatter, linter, debugger, and documentation generator

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

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](docs/contributing.md) for guidelines.

