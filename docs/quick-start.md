# Txt-code Quick Start Guide

## Installation

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build Txt-code
git clone https://github.com/txtcode/txtcode.git
cd txtcode
cargo build --release
```

## Your First Program

Create a file `hello.txt`:

```txtcode
print → "Hello, World!"
```

Run it:

```bash
./target/release/txtcode run hello.txt
```

## Basic Syntax

### Variables
```txtcode
store → name → "Alice"
store → age → 25
store → active → true
```

### Functions
```txtcode
define → greet → (name)
  return → "Hello, " + name
end

print → greet("World")
```

### Control Flow
```txtcode
if → age > 18
  print → "Adult"
else
  print → "Minor"
end

repeat → 5 times
  print → "Count: " + count
end
```

## Compilation

```bash
# Compile to bytecode
txtcode compile program.txt -o program.txtc

# Compile with obfuscation
txtcode compile program.txt --obfuscate

# Compile with encryption
txtcode compile program.txt --encrypt

# Compile to native binary (requires LLVM)
txtcode compile program.txt -t native -o program
```

## Package Management

```bash
# Initialize a package
txtcode package init myproject 0.1.0

# Add a dependency
txtcode package add some_lib 1.0.0

# Install dependencies
txtcode package install
```

## Development Tools

```bash
# Format code
txtcode format program.txt --write

# Lint code
txtcode lint program.txt

# Start REPL
txtcode repl
```

## Examples

See the `examples/` directory for complete example programs:
- `hello.txt` - Hello World
- `calculator.txt` - Calculator
- `port_scanner.txt` - Network port scanner
- `file_processor.txt` - File processing
- `security_demo.txt` - Security features
- `web_server.txt` - Web server

## Next Steps

- Read the [Language Specification](language-spec.md)
- Check the [Syntax Reference](syntax-reference.md)
- Learn about [Security Features](security-features.md)
- See [Contributing Guide](contributing.md)

