# Txt-code Syntax Reference

Quick reference guide for Txt-code v0.4 syntax.

## Basic Syntax

### Comments
```txtcode
# Single-line comment

## Multi-line comment
   Can span multiple lines
##
```

### Variables
```txtcode
store → x → 42
store → name: string → "Alice"
store → active → true
```

### String Literals
```txtcode
# Regular string
store → s → "Hello, World!"

# F-string (interpolated) — embed expressions with { }
store → name → "Alice"
store → greeting → f"Hello, {name}!"

# Raw string — no escape processing
store → path → r"C:\Users\Alice\Documents"
store → regex → r"\d+\.\d+"

# Multiline string
store → text → """
Line one
Line two
"""
```

### Number Literals
```txtcode
# Integer with digit separators (underscore ignored)
store → million → 1_000_000
store → hex → 0xFF
store → bin → 0b1010_1010

# Float
store → pi → 3.141_592_653
```

### Functions
```txtcode
define → greet → (name)
  return → "Hello, " + name
end

define → add → (a: int, b: int) → int
  return → a + b
end

# Destructured map argument
define → show_coords → ({x, y})
  return → f"({x}, {y})"
end
store → pt → {"x": 10, "y": 20}
print → show_coords(pt)

# Multi-return (auto-wraps as array)
define → minmax → (arr)
  return → arr[0], arr[len(arr) - 1]
end
store → bounds → minmax([3, 1, 4, 1, 5])
```

### Async Functions (synchronous mode)
```txtcode
# async/await runs synchronously in v0.4 — no blocking or parallelism yet.
# True async I/O is planned for v0.5.
async → define → fetch → (url: string)
  store → body → await → http_get(url)
  return → body
end

store → result → fetch("https://example.com")
```

### Doc and Hint Annotations
```txtcode
define → compute → (x: int) → int
  doc → "Doubles the input value"
  hint → "Pure function, no side effects"
  return → x * 2
end
```

## Control Flow
```txtcode
# If / elseif / else
if → score >= 90
  print → "A"
elseif → score >= 80
  print → "B"
else
  print → "C"
end

# While loop
while → count < 10
  print → count
  store → count → count + 1
end

# For loop
for → item in items
  print → item
end

# Repeat loop
repeat → 5 times
  print → "Hello"
end

# Do-while loop
do
  store → x → x + 1
while → x < 10
end
```

## Pattern Matching
```txtcode
match → value
  case → 0
    print → "Zero"
  case → n if n > 0
    print → "Positive"
  case → _
    print → "Other"
end

# Array destructuring in match
match → coords
  case → [x, y]
    print → f"x={x} y={y}"
  case → _
    print → "not a 2-element array"
end
```

## Operators

### Arithmetic
- `+` Addition
- `-` Subtraction
- `*` Multiplication
- `/` Division
- `%` Modulo
- `**` Exponentiation

### Compound Assignment
```txtcode
store → x → 10
x += 5    # x = 15
x -= 3    # x = 12
x *= 2    # x = 24
x /= 4    # x = 6
x **= 2   # x = 36
x %= 7    # x = 1
```

### Increment / Decrement
```txtcode
++x   # x = x + 1 (identifier only)
--x   # x = x - 1 (identifier only)
# Note: ++arr[0] is not supported — use arr[0] = arr[0] + 1
```

### Comparison
- `==` Equal, `!=` Not equal
- `<` `>` `<=` `>=`

### Logical
- `and`, `or`, `not`

### Bitwise
- `&` `|` `^` `<<` `>>` `~`

### Special Operators
```txtcode
# Null coalescing
store → val → maybe_null ?? "default"

# Optional chaining (returns null instead of error)
store → field → obj?.key
store → elem  → arr?[0]

# Ternary
store → label → score > 50 ? "pass" : "fail"

# Pipe operator
store → result → 5 |> double
store → upper  → "hello" |> to_upper

# Spread in arrays
store → a → [1, 2]
store → b → [3, 4]
store → c → [...a, ...b]       # [1, 2, 3, 4]
store → d → [0, ...a, 5]       # [0, 1, 2, 5]
```

## Data Types

| Type     | Description                        | Example              |
|----------|------------------------------------|----------------------|
| `int`    | 64-bit signed integer              | `42`, `1_000_000`    |
| `float`  | 64-bit floating-point              | `3.14`, `1.5e10`     |
| `string` | UTF-8 string                       | `"hello"`, `f"{x}"` |
| `bool`   | Boolean                            | `true`, `false`      |
| `char`   | Single Unicode character           | `'a'`                |
| `array`  | Ordered list                       | `[1, 2, 3]`          |
| `map`    | Key-value pairs (string keys)      | `{"x": 1, "y": 2}`  |
| `set`    | Unique values                      | `{1, 2, 3}`          |
| `null`   | Absent value                       | `null`               |

## Error Handling
```txtcode
try
  store → data → json_parse(raw)
catch err
  print → f"Parse error: {err}"
finally
  print → "done"
end

# Result type
store → r → ok(42)
store → e → err("not found")
if is_ok(r)
  print → unwrap(r)
end
```

## Modules
```txtcode
import → "utils"
import → math
import → sqrt, pow from math
import → math as m
```

## Structs and Type Aliases

```txtcode
# Struct definition — parens form (canonical)
struct Point(x: int, y: int)

# Block form (also accepted)
struct → Rectangle
  width: float
  height: float
end

# Type alias
type → UserId → int
type → Hostname → string

# Named error constant
error → NotFound → "Resource not found"
error → Unauthorized → "Access denied"
```

## Permissions and Capabilities

### Granting permissions
```txtcode
grant_permission("fs.read",    "/tmp/*")
grant_permission("net.connect", "*.example.com")
grant_permission("wifi.scan",  null)
grant_permission("ble.scan",   null)
```

### Capability tokens (short-lived, revocable)
```txtcode
store → tok → grant_capability("wifi.capture", null)
use_capability(tok)
store → frames → wifi_capture("wlan0")
revoke_capability(tok)
```

### Function-level declarations
```txtcode
define → probe → (host: string)
  intent   → "network probe only"
  allowed  → ["net.connect", "wifi.scan"]
  forbidden → ["sys.exec", "fs.write"]

  store → result → tcp_connect(f"{host}:80")
  return → is_ok(result)
end
```

`forbidden` is enforced at validation time (before execution).
`allowed` and `intent` are logged to the audit trail.

## WiFi / Bluetooth Functions

All `wifi_*` and `ble_*` functions require the corresponding permission grant.

```txtcode
# WiFi — requires wifi.<action>
wifi_scan()                     # passive scan
wifi_capture("wlan0")           # raw frame capture (monitor mode)
wifi_deauth("wlan0", "AA:BB:CC:DD:EE:FF")  # deauth (auth required)
wifi_inject("wlan0", frame_bytes)           # inject (auth required)

# Bluetooth LE — requires ble.<action>
ble_scan()                               # device discovery
store → h → ble_connect("AA:BB:CC:DD:EE:FF")  # GATT connect
store → v → ble_read(h, "0x2A37")             # read characteristic
ble_write(h, "0x2A06", 0x01)                  # write characteristic
ble_fuzz(h, "0x2A06", 32)                     # fuzz (auth required)
```

## Examples

See the `examples/` directory for complete example programs.
