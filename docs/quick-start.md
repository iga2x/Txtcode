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
# Compile to bytecode (.txtc file)
txtcode compile program.tc -o program.txtc

# Inspect compiled bytecode
txtcode inspect program.txtc

# Inspect as JSON
txtcode inspect program.txtc --format json
```

> **Note:** The compiled file extension is `.txtc`.
> The bytecode VM runs with basic permission checking but no audit trail, policy engine,
> or intent checking. Use `txtcode run` on source files for full security enforcement.
> Native and WASM compilation targets are planned for v0.5.

## Package Management

```bash
# Initialize a package
txtcode package init myproject 0.1.0

# Add a dependency
txtcode package add some_lib 1.0.0

# Install dependencies
txtcode package install
```

## Permissions and Security

Privileged operations require explicit grants. Without a grant the runtime
raises a permission error and records it in the audit trail.

```bash
# Allow filesystem reads under /var/log only
txtcode run scan.tc --allow-fs=/var/log

# Allow outbound connections to one host
txtcode run probe.tc --allow-net=192.168.1.1

# Deny all privileged access (sandbox mode)
txtcode run sandbox.tc --sandbox

# Timeout after 30 seconds
txtcode run long.tc --timeout 30s
```

### Declaring permissions in a function

```txtcode
define → recon → (target: string)
  allowed → ["net.connect", "wifi.scan"]
  forbidden → ["sys.exec", "fs.write"]

  store → nets → wifi_scan()
  store → resp → http_get(f"https://{target}/info")
  return → {"wifi": nets, "http": resp}
end
```

`forbidden` violations are caught at **validation time** (before execution).
`allowed` declarations are advisory and logged to the audit trail.

### WiFi and Bluetooth operations

```txtcode
# Requires wifi.scan permission (enforced)
grant_permission("wifi.scan", null)
store → networks → wifi_scan()

# Requires ble.scan permission (enforced)
grant_permission("ble.scan", null)
store → devices → ble_scan()

# BLE connect + read (requires both permissions)
grant_permission("ble.connect", null)
grant_permission("ble.read", null)
store → handle → ble_connect("AA:BB:CC:DD:EE:FF")
store → data → ble_read(handle, "0x2A37")
```

## Development Tools

```bash
# Format code (print result)
txtcode format program.tc

# Format in-place
txtcode format program.tc --write

# CI check: exit non-zero if formatting needed
txtcode format src/ --check

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
- Understand the [Permission and Capability System](permissions.md)
- See [Contributing Guide](contributing.md)

