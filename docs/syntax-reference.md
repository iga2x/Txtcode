# Txt-code Syntax Reference

Quick reference guide for Txt-code v3.0.0 syntax.

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

### Async Functions
```txtcode
# async define spawns an OS thread; await blocks until the result is ready.
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

### Or-Patterns

Match a value against multiple alternatives in a single `case` arm using `|`:

```txtcode
match → status_code
  case → 200 | 201 | 204
    print → "success"
  case → 400 | 401 | 403
    print → "client error"
  case → _
    print → "other"
end
```

### Range Patterns (Inclusive)

Match a value against an inclusive integer range using `..=`:

```txtcode
match → score
  case → 90..=100
    print → "A"
  case → 80..=89
    print → "B"
  case → 70..=79
    print → "C"
  case → _
    print → "F"
end
```

### Literal Patterns

Match against exact literal values (int, float, string, bool):
```txtcode
match → command
  case → "quit"
    print → "Goodbye"
  case → "help"
    print → "Commands: ..."
  case → _
    print → "Unknown command"
end
```

### Rest Pattern

In array destructuring, `...rest` captures remaining elements:
```txtcode
store → [first, ...rest] → [1, 2, 3, 4]
# first == 1, rest == [2, 3, 4]
```

Or-patterns, range patterns, and literal patterns can appear together in the same match expression.
All patterns work in both the AST VM and the bytecode VM.

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

### `?` Error Propagation Operator

Postfix `?` provides concise early-return on error inside a function. It is the
shorthand replacement for manually checking `is_err(r)` and returning the error:

```txtcode
define → load_config → (path)
  store → raw → read_file(path)?   ;; returns Err immediately if read fails
  store → cfg → json_parse(raw)?   ;; returns Err immediately if parse fails
  return → cfg
end

store → result → load_config("config.json")
if is_ok(result)
  print → unwrap(result)
else
  print → f"Failed: {unwrap_err(result)}"
end
```

Semantics:
- `expr?` where `expr` is `Ok(v)` — evaluates to `v` (unwrapped).
- `expr?` where `expr` is `Err(e)` — immediately returns `Err(e)` from the enclosing function.
- `expr?` where `expr` is not a `Result` — passes the value through unchanged.
- Works in both AST VM and bytecode VM.

## Modules
```txtcode
import → "utils"
import → math
import → sqrt, pow from math
import → math as m
```

## Structs, Type Aliases, and impl Blocks

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

### impl Blocks (Struct Methods)

Attach methods to a struct type using an `impl` block. Methods are called as
`obj.method(args)`. The receiver (`self`) is auto-prepended by the runtime.

```txtcode
struct Point(x: int, y: int)

impl → Point
  define → sum → (self)
    return → self.x + self.y
  end

  define → scale → (self, factor)
    return → Point { x: self.x * factor, y: self.y * factor }
  end
end

store → p → Point { x: 3, y: 4 }
print → p.sum()          ;; 7
store → p2 → p.scale(2)  ;; Point { x: 6, y: 8 }
```

Rules:
- `impl` takes the struct name after the arrow: `impl → StructName`.
- Each method is a normal `define` block; the first parameter (`self` by convention) receives the struct instance.
- Methods can call other methods via `self.other_method(args)`.
- Works in both the AST VM and the bytecode VM.

## Permissions and Capabilities

### Granting permissions
```txtcode
grant_permission("fs.read",    "/tmp/*")
grant_permission("net.connect", "*.example.com")
grant_permission("sys.exec",   null)
deny_permission("fs.delete",   null)
```

### Capability tokens (short-lived, revocable)
```txtcode
store → tok → grant_capability("fs", "read", "/var/log/*")
use_capability(tok)
store → data → read_file("/var/log/syslog")
revoke_capability(tok)    # subsequent calls fail immediately
```

### Function-level declarations
```txtcode
define → probe → (host: string)
  intent    → "network reachability probe only"
  allowed   → ["net.connect"]
  forbidden → ["sys.exec", "fs.write"]

  store → result → tcp_connect(f"{host}:80")
  return → is_ok(result)
end
```

`forbidden` is enforced at **validation time** (before execution).
`allowed` and `intent` are logged to the audit trail at runtime.

## Generators (yield)

A function containing `yield` is a generator. Calling it returns an array of all yielded values.

```txtcode
define → squares → (n)
  store → i → 1
  while → i <= n
    yield → i * i
    store → i → i + 1
  end
end

print → squares(4)    # [1, 4, 9, 16]
```

## Protocols

Declare an interface; enforce it with `implements` on a struct.

```txtcode
protocol → Printable
  to_string() → string
end

struct Point(x: int, y: int) implements Printable

impl → Point
  define → to_string → (self) → string
    return → f"({self.x}, {self.y})"
  end
end
```

## Type Aliases and Named Errors

```txtcode
type → UserId → int
type → Hostname → string

error → NotFound → "Resource not found"
error → Unauthorized → "Access denied"

# Use in function signatures:
define → find_user → (id: UserId) → User?
  # returns User or null
end

# Raise a named error:
return → err(NotFound)
```

## Nullable Types

Append `?` to a type to allow `null` as a valid value:

```txtcode
store → name: string? → null
define → find → (id: int) → User?
  # may return User or null
end

# Safe access with ??:
store → display → name ?? "anonymous"
```

## Structured Concurrency (nursery)

```txtcode
async → nursery
  nursery_spawn(() → fetch("https://api.example.com/a"))
  nursery_spawn(() → fetch("https://api.example.com/b"))
end
# both complete (or one fails and all are cancelled) before continuing
```

## Keyword Aliases

These are canonicalized at lex time — both forms are identical:

| Canonical | Alias(es) |
|-----------|----------|
| `store` | `let` |
| `define` | `def` |
| `return` | `ret` |
| `print` | `out` |
| `import` | `use` |
| `elseif` | `elif` |
| `match` | `switch` |
| `for` | `foreach` |
| `intent` | `doc` |
| `hint` | `ai_hint` |

## Examples

See the `examples/` directory for complete example programs.
