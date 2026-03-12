# Txt-code Quick Start Guide

## Installation

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build Txt-code
git clone https://github.com/iga2x/txtcode.git
cd txtcode
cargo build --release
```

## Your First Program

Create a file `hello.tc`:

```txtcode
print → "Hello, World!"
```

Run it:

```bash
./target/release/txtcode run hello.tc
```

## Basic Syntax

### Variables
```txtcode
store → name → "Alice"
store → age → 25
store → pi → 3.141_592_653     # number separators
store → path → r"C:\tmp\file"  # raw string (no escape processing)
store → msg → f"Hello, {name}! Age: {age}"  # f-string interpolation
```

### Functions
```txtcode
define → greet → (name)
  return → "Hello, " + name
end

print → greet("World")

# Multi-return (returns array)
define → bounds → (arr)
  return → arr[0], arr[len(arr) - 1]
end

# Destructured map argument
define → area → ({width, height})
  return → width * height
end
print → area({"width": 10, "height": 5})
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

# Compound assignment
store → x → 10
x += 5
x *= 2
```

### Arrays and Spread
```txtcode
store → a → [1, 2, 3]
store → b → [4, 5]
store → c → [...a, ...b]   # [1, 2, 3, 4, 5]
```

### Pipe Operator
```txtcode
define → double → (x)
  return → x * 2
end
store → result → 5 |> double   # 10

# Works with lambdas too
store → upper → "hello" |> (s) -> to_upper(s)
```

## Compilation

```bash
# Compile to bytecode (.tcc file)
txtcode compile program.tc -o program.tcc

# Inspect compiled bytecode
txtcode inspect program.tcc

# Inspect as JSON
txtcode inspect program.tcc --format json
```

> **Note:** Only `bytecode` target is supported. Passing `--target native` or `--target wasm`
> will print an error. Native and WASM compilation are planned for v0.5.

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
txtcode format program.tc --write

# Lint code
txtcode lint program.tc

# Start REPL
txtcode repl
```

## Examples

See the `examples/` directory for complete example programs:
- `hello.tc` - Hello World
- `calculator.tc` - Calculator
- `port_scanner.tc` - Network port scanner
- `file_processor.tc` - File processing
- `security_demo.tc` - Security features
- `web_server.tc` - Web server

## Next Steps

- Read the [Language Specification](language-spec.md)
- Check the [Syntax Reference](syntax-reference.md)
- Learn about [Security Features](security-features.md)
- See [Contributing Guide](contributing.md)

