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
> Native and WASM compilation backends are planned for v0.6/v0.8.

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

## New in v0.5.0 — Stdlib Highlights

### Streaming File I/O
```txtcode
store → h → file_open("data.txt", "r")
store → line → file_read_line(h)
while → line != null
  print → line
  store → line → file_read_line(h)
end
file_close(h)
```

### Datetime
```txtcode
store → today → format_datetime(now(), "%Y-%m-%d", "UTC")
store → tomorrow → datetime_add(now(), 1, "days")
store → diff → datetime_diff(tomorrow, now(), "hours")
```

### CSV Write
```txtcode
csv_write("/tmp/report.csv", [["name","score"],["Alice",95],["Bob",87]])
```

### Process Piping
```txtcode
store → result → exec_pipe(["echo hello world", "tr a-z A-Z"])
```

### Async/Await
```txtcode
async define → fetch_data → (url)
  return → http_get(url)
end
store → handle → fetch_data("https://api.example.com/data")
store → result → await handle
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

# Start LSP server (for VS Code / Neovim / Zed integration)
txtcode lsp

# Run static type checker
txtcode run program.tc --type-check

# Type errors as hard errors
txtcode run program.tc --strict-types
```

## Package Management

```bash
# Initialize a package
txtcode package init myproject 0.1.0

# Add a dependency
txtcode package add npl-math 0.1.0

# Install all dependencies
txtcode package install

# Install a local package directory
txtcode package install-local packages/npl-math

# Search the registry
txtcode package search math

# Show package details
txtcode package info npl-math
```

### Using packages in scripts
```txtcode
from "npl-math/math" import is_prime, factorial, fib
from "npl-strings/strings" import pad_left, truncate
from "npl-collections/collections" import zip, chunk, range
from "npl-datetime/datetime" import today, relative_time

print(is_prime(17))                      ## true
print(factorial(10))                     ## 3628800
print(today())                           ## "2026-03-19"
print(range(0, 5))                       ## [0, 1, 2, 3, 4]
```

## Examples

See the `examples/` directory for complete example programs:
- `hello_world.tc` - Hello World, variables, functions, control flow
- `calculator.tc` - Arithmetic with pattern matching
- `file_processing.tc` - CSV parsing and stats
- `log_analyzer.tc` - Log classification
- `pipeline.tc` - Sequential task runner
- `audit_trail.tc` - File I/O with permission model
- `security_demo.tc` - SHA-256, AES-GCM, base64
- `metrics_report.tc` - Map grouping and aggregation

## Next Steps

- Read the [Language Specification](language-spec.md)
- Check the [Syntax Reference](syntax-reference.md)
- Learn about [Security Features](security-features.md)
- Understand the [Permission and Capability System](permissions.md)
- See [Contributing Guide](contributing.md)

