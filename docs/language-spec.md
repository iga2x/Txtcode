# Txt-code Language Specification — v0.4.0

> **Status:** v0.4.0 release. Behaviour described here matches the current implementation
> except where explicitly marked as “planned / not yet implemented”.
> Breaking changes between minor versions are documented in [CHANGELOG.md](../CHANGELOG.md).
>
> **New in v0.4:**
> - Virtual environment system (`txtcode env`) with 12 subcommands
> - Bytecode VM: `break`/`continue`, `for x in arr`, `repeat N`, `match`, string interpolation
> - Integer overflow guards (`checked_*`) in both AST VM and bytecode VM
> - Recursion depth limit (50) enforced across all VMs
> - User-defined functions with caller/callee scope isolation in bytecode VM
> - Module imports (`ImportModule`) in bytecode VM
> - `?.` / `?[]` / `?()` optional chaining — both VMs (returns `null` on null target)
> - `do…while` loop in bytecode VM
> - `f”...”` string interpolation prefix; raw strings `r”...”`, number separators `1_000_000`
> - Spread operator `[...arr]` in both VMs
> - Multi-return: `return → a, b` auto-wraps as `[a, b]`
> - Destructured function arguments: `define → f → ({x, y})`
> - `doc →` / `hint →` canonical annotation keywords
> - Pipe operator `|>` with lambda/complex RHS in both VMs
> - Pattern matching: array `[a, b]` and struct `{x, y}` destructuring in `match`
> - `++`/`--` now errors cleanly on non-identifier targets
> - `txtcode inspect` command — disassemble compiled bytecode
> - Call depth unified to 50 across all VMs
> - `async`/`await` runs synchronously (passthrough; true async planned for v0.5)
>
> **Still not fully implemented:**
> - Generic type enforcement at runtime (type params are parsed but erased)
> - `++arr[0]` / `--arr[0]` — use `store → arr[0] → arr[0] + 1` instead
> - Integer overflow guards — ✅ implemented in v0.4 (`checked_*` arithmetic in both VMs; overflow returns `RuntimeError`)

---

## Table of Contents

1. [Syntax](#syntax)
2. [Values and Types](#values-and-types)
3. [Operator Behaviour](#operator-behaviour)
4. [Function Rules](#function-rules)
5. [Module Resolution](#module-resolution)
6. [Runtime Limits](#runtime-limits)
7. [Error Semantics](#error-semantics)

---

## 1. Syntax

### 1.1 Source Encoding

All source files must be valid UTF-8. The file extension for Txt-code source files is `.tc`.

### 1.2 Whitespace and Line Endings

Statements are separated by newlines. Blank lines are ignored. LF and CRLF are both accepted.
Indentation is not significant — it is used only for readability.

### 1.3 Comments

```txtcode
# Single-line comment (rest of line is ignored)

## Multi-line comment
   continues until the next ##
##
```

### 1.4 Syntax Styles

Txt-code supports two equivalent syntax forms. They may be mixed freely.

| Style | Example |
|-------|---------|
| Arrow-based (preferred) | `store → x → 42` |
| Space-based (shorthand) | `store x 42` |

The arrow `→` (Unicode U+2192) and `->` (ASCII two-character sequence) are both accepted as arrow tokens. Space-based style uses a space where the arrow would appear.

`define name (params)` (without arrows) is deprecated. The parser accepts it with a warning.

### 1.5 Identifiers

Identifiers begin with a letter (`[a-zA-Z]`) or underscore (`_`), followed by zero or more letters, digits (`[0-9]`), or underscores. Identifiers are case-sensitive.

Reserved keywords may not be used as identifiers:
`and`, `array`, `assert`, `async`, `await`, `bool`, `break`, `call`, `case`, `catch`,
`char`, `const`, `continue`, `define`, `do`, `else`, `elseif`, `end`, `enum`,
`export`, `false`, `finally`, `float`, `for`, `forbidden`, `if`, `import`, `in`,
`int`, `intent`, `map`, `match`, `not`, `null`, `or`, `permission`, `repeat`,
`return`, `set`, `store`, `struct`, `times`, `true`, `try`, `while`.

### 1.6 Literals

```txtcode
# Integer literals
42          # decimal
0xFF        # hexadecimal
0b1010      # binary
0o17        # octal

# Float literals
3.14
-0.5
1.0e10
1.5e-3

# String literals
"hello"
'world'
"multi\nline"
f"Hello, {name}!"     # interpolated (see §1.10)

# Character literals
'a'         # single Unicode code point

# Boolean literals
true
false

# Null literal
null
```

### 1.7 Statements

The complete set of valid statements:

#### Assignment (variable)
```txtcode
store → name → expression
store → name: type → expression    # with type annotation
let → name → expression            # alias for store
```

Destructuring patterns are supported:
```txtcode
store → [a, b, c] → array_expr          # array destructure
store → {x: field_x, y: field_y} → map  # struct destructure
store → Point(x, y) → point_value       # constructor destructure
```

#### Compound Assignment
```txtcode
store → x → x + 1    # explicit
x += 1               # compound (+=, -=, *=, /=, %=)
```

#### Index Assignment
```txtcode
store → arr[0] → value
store → map["key"] → value
```

#### Constants
```txtcode
const → PI → 3.14159
```
Constants are evaluated at parse time and may not be reassigned.

#### Function Definition
See §4 for full rules.

```txtcode
define → name → (params) → return_type
  body
end
```

#### Conditional
```txtcode
if → condition
  body
elseif → condition
  body
else
  body
end
```

#### While Loop
```txtcode
while → condition
  body
end
```

#### Do-While Loop
```txtcode
do
  body
while → condition
```
The body executes at least once before the condition is tested.

#### For Loop
```txtcode
for → variable in iterable
  body
end
```
`iterable` may be any `array`, `set`, or a range expression `range(start, end)`.

#### Repeat Loop
```txtcode
repeat → N times
  body
end
```
`N` must evaluate to a non-negative integer. `times` is a required keyword.

#### Pattern Matching
```txtcode
match → expression
  case → pattern
    body
  case → pattern if guard_expression
    body
  case → _
    default_body
end
```

#### Try / Catch / Finally
```txtcode
try
  body
catch → error_variable
  body
finally
  body            # always runs, with or without error
end
```
`catch` and `finally` are both optional, but at least one must be present.

#### Import
```txtcode
import → module_name
import → name1, name2 from module_name
import → module_name as alias
```

#### Export
```txtcode
export → name1, name2
```

#### Struct Definition
```txtcode
struct → Point
  x: int
  y: int
end
```

#### Enum Definition
```txtcode
enum → Color
  Red
  Green
  Blue
  Custom → 255    # variant with associated value
end
```

#### Assert
```txtcode
assert → condition
assert → condition, "Failure message"
```
A failing assert raises a `RuntimeError` with the optional message (or a default if omitted).

#### Permission Statement
Declares required capabilities for the enclosing scope:
```txtcode
permission → fs.read
permission → net.connect → "example.com"
```

### 1.8 Expressions

```
expression :=
  | literal
  | identifier
  | expression op expression          # binary operation
  | unary_op expression               # unary operation
  | identifier(args)                  # function call
  | identifier<types>(args)           # generic function call
  | expression.method(args)           # method call
  | expression.field                  # member access
  | expression?.field                 # optional member (null-safe)
  | expression?(args)                 # optional call (null-safe)
  | expression?[index]                # optional index (null-safe)
  | expression[index]                 # index
  | expression[start:end:step]        # slice (any part optional)
  | [elements]                        # array literal
  | {key: value, ...}                 # map literal
  | {| elements |}                    # set literal
  | (params) → body                   # lambda
  | condition ? true_expr : false_expr  # ternary
  | await → expression                # await (inside async only)
  | f"text {expr} text"              # interpolated string
```

### 1.9 Slice Expressions

```txtcode
store → sub → arr[1:4]        # elements 1, 2, 3
store → evens → arr[::2]      # every second element
store → rev → arr[::-1]       # reversed
```

Any of `start`, `end`, `step` may be omitted. `step` defaults to 1.

### 1.10 String Interpolation

```txtcode
store → name → "Alice"
store → greeting → f"Hello, {name}!"    # → "Hello, Alice!"
store → math → f"1 + 1 = {1 + 1}"      # → "1 + 1 = 2"
```

The `f` prefix triggers interpolation. Expressions inside `{}` are fully evaluated.

### 1.11 Optional Chaining

Optional access operators return `null` when the target is `null` rather than raising an error:

```txtcode
store → len → user?.name         # null if user is null
store → val → map?.["key"]       # null if map is null
store → res → func?.()           # null if func is null; calls func if not null
```

All three operators (`?.`, `?[]`, `?()`) are fully implemented in both the AST VM and bytecode VM as of v0.4.

### 1.12 Spread Operator

The `...` spread operator expands an array into the surrounding array literal:

```txtcode
store → a → [1, 2]
store → b → [3, 4]
store → c → [...a, ...b]       # [1, 2, 3, 4]
store → d → [0, ...a, 5]       # [0, 1, 2, 5]
store → e → [...a, ...b, ...a] # [1, 2, 3, 4, 1, 2]
```

Rules:
- Spread elements must evaluate to `array`. Spreading a non-array is a `RuntimeError`.
- Spread is only supported inside array literals (`[...]`). It is not supported in function call arguments in v0.4.

### 1.13 Multi-Return Values

A `return` statement may return multiple comma-separated expressions. They are automatically wrapped in an `array`:

```txtcode
define → minmax → (arr)
  return → arr[0], arr[len(arr) - 1]
end

store → result → minmax([3, 1, 4, 5, 2])
# result == [3, 2]  (first and last element)
```

The caller receives a plain `array`. To unpack: `store → lo → result[0]`.

### 1.14 Destructured Function Arguments

A function parameter wrapped in `{...}` destructures a `map` argument:

```txtcode
define → describe → ({name, age})
  return → f"{name} is {age} years old"
end

print → describe({"name": "Alice", "age": 30})
# Alice is 30 years old
```

Rules:
- Each field name in `{...}` becomes a local variable bound to `arg["field_name"]`.
- If the key is absent in the map, the variable is `null`.
- Destructured parameters are positional — the map is passed as a single argument.

### 1.15 Grammar (Formal BNF)

```
program           := statement*

statement         :=
    | assignment | compound_assignment | index_assignment
    | function_def | return | break | continue
    | if_stmt | while_stmt | do_while_stmt | for_stmt | repeat_stmt
    | match_stmt | try_stmt | assert_stmt
    | import_stmt | export_stmt | const_stmt
    | struct_def | enum_def | permission_stmt
    | expression

assignment        := ("store"|"let") ("→"|WS) pattern (":" type)? ("→"|WS) expression
compound_assignment := identifier op "=" expression    # op: + - * / %
index_assignment  := expression "[" expression "]" ("→"|"=") expression

function_def      := ("async" ("→")?)? "define" ("→"|WS) identifier
                     ("<" type_params ">")? ("→")? "(" params? ")" ("→" type)?
                     (intent_decl | ai_hint_decl | allowed_decl | forbidden_decl)*
                     statement* "end"

params            := param ("," param)*
param             := "..."? identifier (":" type)? ("=" expression)?

intent_decl       := "intent" "→" string_literal
ai_hint_decl      := "ai_hint" "→" string_literal
allowed_decl      := "allowed" "→" "[" cap_string ("," cap_string)* "]"
forbidden_decl    := "forbidden" "→" "[" cap_string ("," cap_string)* "]"

if_stmt           := "if" ("→"|WS) expression statement*
                     ("elseif" ("→"|WS) expression statement*)*
                     ("else" statement*)? "end"

while_stmt        := "while" ("→"|WS) expression statement* "end"
do_while_stmt     := "do" statement* "while" ("→"|WS) expression
for_stmt          := "for" ("→"|WS) identifier "in" expression statement* "end"
repeat_stmt       := "repeat" ("→"|WS) expression "times" statement* "end"

match_stmt        := "match" ("→"|WS) expression
                     ("case" ("→"|WS) pattern ("if" expression)? statement*)+
                     ("case" ("→"|WS) "_" statement*)? "end"

try_stmt          := "try" statement*
                     ("catch" ("→"|WS) identifier statement*)?
                     ("finally" statement*)? "end"

assert_stmt       := "assert" ("→"|WS) expression ("," expression)?
import_stmt       := "import" ("→"|WS) (name_list "from")? identifier ("as" identifier)?
export_stmt       := "export" ("→"|WS) name_list
const_stmt        := "const" ("→"|WS) identifier ("→"|WS) expression

struct_def        := "struct" ("→"|WS) identifier field* "end"
field             := identifier ":" type
enum_def          := "enum" ("→"|WS) identifier variant* "end"
variant           := identifier ("→" expression)?
permission_stmt   := "permission" "→" identifier "." identifier ("→" string_literal)?

type              := "int" | "float" | "string" | "char" | "bool"
                   | "array" "[" type "]" | "map" "[" type "]" | "set" "[" type "]"
                   | "Future" "<" type ">"
                   | identifier           # named struct/enum
                   | identifier           # generic type parameter

pattern           := identifier | "[" pattern_list "]"
                   | "{" (identifier ":" pattern)* ("..." identifier)? "}"
                   | identifier "(" pattern_list ")"
                   | "_"
```

---

## 2. Values and Types

### 2.1 Primitive Types

| Type | Storage | Range / Notes |
|------|---------|---------------|
| `int` | 64-bit signed integer | −9 223 372 036 854 775 808 to 9 223 372 036 854 775 807 |
| `float` | 64-bit IEEE 754 double | ±1.8 × 10^308, 15–17 significant digits |
| `string` | UTF-8 heap string | Arbitrary length |
| `char` | Single Unicode code point | U+0000 – U+10FFFF |
| `bool` | Boolean | `true` or `false` |
| `null` | Absence of value | Only value is `null` |

### 2.2 Composite Types

| Type | Syntax | Notes |
|------|--------|-------|
| `array[T]` | `[1, 2, 3]` | Ordered, zero-indexed, dynamically sized |
| `map[T]` | `{"key": value}` | String keys only, values of type T |
| `set[T]` | `{| 1, 2, 3 |}` | Unordered, unique values |
| `struct Name` | `Name { field: value }` | Named fields, declared with `struct` |
| `enum Name` | `Name.Variant` | Discriminated union, declared with `enum` |
| `function` | `(x) → x + 1` | First-class, captures enclosing environment |

### 2.3 Async Type

| Type | Notes |
|------|-------|
| `Future<T>` | Returned by `async` functions; resolved with `await` |

Calling an async function without `await` yields a `Future<T>`.
Calling `await` on a non-`Future` value is a runtime error.

### 2.4 Type Annotations (Optional)

Type annotations are optional everywhere. The type checker uses Hindley-Milner-style inference.

```txtcode
store → x: int → 42
store → name: string → "Alice"
define → add → (a: int, b: int) → int
  return → a + b
end
```

### 2.5 Type Compatibility

The following implicit coercions apply (checked at runtime for dynamic paths):

| From | To | Rule |
|------|----|------|
| `int` | `float` | Widening: exact value preserved |
| `float` | `int` | Narrowing: **not** implicit — explicit `int(f)` required |
| `char` | `string` | Lossless: single-character string |
| `string` | `char` | Only if string has exactly one code point |
| `T` | `string` | Via `+` operator: `str(T)` conversion applied automatically |

Type compatibility for parameterised types is covariant on the element type.
`array[int]` is compatible with `array[float]` because `int` is compatible with `float`.

### 2.6 Default Values

Variables declared without assignment are `null`.
Functions with no explicit `return` return `null`.

### 2.7 Equality Semantics

- `int == int`: bitwise equality
- `float == float`: within `f64::EPSILON` (approximately 2.2 × 10^−16)
- `string == string`: byte-for-byte equality
- `char == char`: Unicode code-point equality
- `bool == bool`: value equality
- `null == null`: always `true`
- Cross-type `==`: always `false` (no coercion for equality)
- `array`, `map`, `set`, `struct`, `function`: reference equality (same instance)

---

## 3. Operator Behaviour

### 3.1 Arithmetic Operators

| Operator | Left | Right | Result | Notes |
|----------|------|-------|--------|-------|
| `+` | `int` | `int` | `int` | — |
| `+` | `float` | `float` | `float` | — |
| `+` | `int` | `float` | `float` | int widened to float |
| `+` | `float` | `int` | `float` | int widened to float |
| `+` | `string` | `string` | `string` | Concatenation |
| `+` | `char` | `char` | `string` | Both chars concatenated |
| `+` | `string` | `char` | `string` | Char appended |
| `+` | `char` | `string` | `string` | Char prepended |
| `+` | `string` | `any` | `string` | Right auto-converted via `str()` |
| `+` | `any` | `string` | `string` | Left auto-converted via `str()` |
| `+` | other | other | **RuntimeError** | Invalid operands |
| `-` | `int` | `int` | `int` | — |
| `-` | `float` | `float` | `float` | — |
| `-` | `int` | `float` | `float` | — |
| `-` | `float` | `int` | `float` | — |
| `-` | other | other | **RuntimeError** | — |
| `*` | `int` | `int` | `int` | — |
| `*` | `float` | `float` | `float` | — |
| `*` | `int` | `float` | `float` | — |
| `*` | `float` | `int` | `float` | — |
| `*` | other | other | **RuntimeError** | — |
| `/` | `int` | `int` | `int` | Truncating (floor toward zero). **Error** if divisor is 0 |
| `/` | `float` | `float` | `float` | IEEE 754. **Error** if divisor is 0.0 |
| `/` | `int` | `float` | `float` | **Error** if divisor is 0.0 |
| `/` | `float` | `int` | `float` | **Error** if divisor is 0 |
| `%` | `int` | `int` | `int` | **Error** if divisor is 0 |
| `%` | other | other | **RuntimeError** | Modulo requires both `int` |
| `**` | `int` | `int` | `int` | **Error** if exponent is negative |
| `**` | `float` | `float` | `float` | — |
| `**` | `int` | `float` | `float` | — |
| `**` | `float` | `int` | `float` | — |
| `**` | other | other | **RuntimeError** | — |

**Integer division** truncates toward zero: `7 / 2 == 3`, `-7 / 2 == -3`.

### 3.2 Unary Operators

| Operator | Operand | Result | Notes |
|----------|---------|--------|-------|
| `-` (negate) | `int` | `int` | Two's complement negation |
| `-` (negate) | `float` | `float` | IEEE 754 negation |
| `not` | `bool` | `bool` | Logical complement |
| `~` | `int` | `int` | Bitwise NOT |
| `++` (prefix) | `int` | `int` | Implemented in v0.4. |
| `--` (prefix) | `int` | `int` | Implemented in v0.4. |

### 3.3 Comparison Operators

Comparison is defined between values of **compatible types only**. Comparing incompatible types raises a `RuntimeError`.

| Types | Operators | Semantics |
|-------|-----------|-----------|
| `int` ↔ `int` | `== != < > <= >=` | Numeric |
| `float` ↔ `float` | `== != < > <= >=` | IEEE 754 (`==` uses epsilon tolerance) |
| `int` ↔ `float` | `== != < > <= >=` | int widened to float |
| `string` ↔ `string` | `== != < > <= >=` | Lexicographic (byte order) |
| `char` ↔ `char` | `== != < > <= >=` | Unicode code-point order |
| `bool` ↔ `bool` | `== !=` | Value equality only |
| `null` ↔ `null` | `== !=` | Always `null == null` is `true` |

### 3.4 Logical Operators

Short-circuit evaluation applies to `and` and `or`.

| Operator | Behaviour |
|----------|-----------|
| `and` | Returns `false` as soon as left operand is falsy; otherwise evaluates right |
| `or` | Returns `true` as soon as left operand is truthy; otherwise evaluates right |
| `not` | Negates a boolean value |

**Truthiness rules** (used by `if`, `while`, logical ops):

| Value | Truthiness |
|-------|-----------|
| `false` | falsy |
| `null` | falsy |
| `0` (int) | falsy |
| `0.0` (float) | falsy |
| `""` (empty string) | falsy |
| `[]` (empty array) | falsy |
| `{}` (empty map) | falsy |
| everything else | truthy |

### 3.5 Bitwise Operators

Require `int` operands on both sides. Any other type raises a `RuntimeError`.

| Operator | Operation |
|----------|-----------|
| `&` | Bitwise AND |
| `\|` | Bitwise OR |
| `^` | Bitwise XOR |
| `<<` | Left shift |
| `>>` | Right shift (arithmetic, sign-extending) |
| `~` | Bitwise NOT (unary) |

### 3.6 Null Coalescing

```txtcode
store → result → value ?? default
```

Returns `value` if it is not `null`, otherwise returns `default`. The right operand is only evaluated if needed.

> **v0.4 status:** `??` is fully implemented in the bytecode VM (`NullCoalesce` instruction). Works in both AST VM and bytecode mode.

### 3.7 Ternary

```txtcode
store → abs → x >= 0 ? x : -x
```

### 3.8 Operator Precedence (highest to lowest)

| Level | Operators |
|-------|-----------|
| 1 | `()` parentheses, `[]` index, `.` member, `?.` `?[]` `?()` optional |
| 2 | `++` `--` (prefix), `-` (unary), `not`, `~` |
| 3 | `**` |
| 4 | `*` `/` `%` |
| 5 | `+` `-` |
| 6 | `<<` `>>` |
| 7 | `&` |
| 8 | `^` |
| 9 | `\|` |
| 10 | `<` `>` `<=` `>=` |
| 11 | `==` `!=` |
| 12 | `and` |
| 13 | `or` |
| 14 | `??` (null coalesce) |
| 15 | `? :` (ternary) |
| 16 | `→` (arrow, assignment/call) |

---

## 4. Function Rules

### 4.1 Basic Definition

```txtcode
define → name → (params) → return_type
  body
end
```

All parts except `define`, the name, and the parameter list are optional.
`return_type` is an annotation only; the runtime does not enforce it except via the type checker.

### 4.2 Parameters

Each parameter supports:

| Feature | Syntax | Notes |
|---------|--------|-------|
| Plain | `name` | Positional |
| Typed | `name: type` | Type annotation |
| Default value | `name = expr` | Used when argument is omitted |
| Typed with default | `name: type = expr` | Both combined |
| Variadic | `...name` | Collects remaining args into an array; must be last |

```txtcode
define → greet → (name: string, title = "Mr")
  return → "Hello, " + title + " " + name
end

define → sum_all → (...nums: int) → int
  store → total → 0
  for → n in nums
    store → total → total + n
  end
  return → total
end
```

**Constraints:**
- A variadic parameter must be the last parameter.
- A variadic parameter may not have a default value.
- Parameters with default values must appear after parameters without defaults.

### 4.3 Generic Functions

```txtcode
define → identity<T> → (x: T) → T
  return → x
end
```

Type parameters are listed after the function name inside `<>` and may be used in parameter types and return types.

> **v0.4 note:** Type parameters are parsed and stored in the AST but are **type-erased at runtime**. No generic specialisation or type-checking against `T` occurs. All type annotations are advisory and validated by the type-checker tool only, not by the runtime.

### 4.4 Return

```txtcode
return → expression       # returns single value
return → a, b, c          # multi-return: auto-wraps as [a, b, c]
return                    # returns null
```

A `return → a, b` statement wraps all expressions in a plain `array` at the call site. The caller receives a regular array value.

Execution stops at the first `return` encountered. A function that reaches `end` without a `return` returns `null`.

### 4.5 First-Class Functions and Closures

Functions are values. They capture their enclosing environment at definition time (closure semantics).

```txtcode
define → make_adder → (n: int)
  return → (x: int) → x + n    # captures n
end

store → add5 → make_adder(5)
print → add5(3)    # 8
```

The captured environment is a snapshot: mutations to `n` after `make_adder` returns do not affect existing closures.

### 4.6 Async Functions (synchronous mode in v0.4)

> **v0.4 note:** `async`/`await` syntax is fully parsed and accepted. In the current
> implementation both VMs execute async functions **synchronously** — `await` evaluates
> the expression and returns its value immediately without any blocking or parallelism.
> Full Tokio-backed async I/O is planned for v0.5.

```txtcode
async → define → fetch → (url: string) → string
  # In v0.4 this runs synchronously — http_get blocks
  store → body → await → http_get(url)
  return → body
end

store → result → fetch("https://example.com")
```

- `async → define` is accepted; the function runs synchronously.
- `await → expr` evaluates `expr` and returns the result directly.
- No `Future<T>` type at runtime — the value is returned as-is.
- `await_all` is not built-in; model parallel execution with sequential calls for now.

### 4.7 Capability Declarations

Functions may declare which system capabilities they require or prohibit:

```txtcode
define → read_config → (path: string) → string
  allowed → ["fs.read"]
  forbidden → ["net.connect", "sys.exec"]
  return → read_file(path)
end
```

**Capability format:** `"resource.action"` or `"resource.action:scope"`

| Resource | Actions |
|----------|---------|
| `fs` | `read`, `write`, `delete`, `*` |
| `net` | `connect`, `listen`, `*` |
| `sys` | `exec`, `env`, `*` |
| `tool` | `tool:name` or `tool:*` |

Wildcard forms: `"fs.*"` (any fs action), `"*.read"` (read on any resource), `"*.*"` (unrestricted).

Scoped form: `"fs.read:/tmp/*"` restricts the action to a specific path/host pattern.

The runtime enforces capability declarations. Violating a `forbidden` capability raises a `RuntimeError`.

### 4.8 Doc and Hint Annotations

These are metadata-only annotations that do not affect runtime behaviour but are emitted by `txtcode doc`:

```txtcode
define → parse_json → (input: string) → map
  doc → "Parse a JSON string into a map"
  hint → "Input must be valid JSON; raises on parse error"
  return → json_decode(input)
end
```

- `doc →` (canonical) / `intent →` (legacy alias) — human-readable description
- `hint →` (canonical) / `ai_hint →` (legacy alias) — AI/tooling guidance

Both legacy names are still accepted and automatically canonicalized by the parser.

### 4.9 Scope and Variable Lookup

Variables are looked up in scope order: local → enclosing closures → module-level globals.
Re-assigning a variable in an inner scope creates a new binding; it does not modify the outer scope.

```txtcode
store → x → 10
define → f → ()
  store → x → 20    # new binding, does not modify outer x
  return → x
end
print → f()   # 20
print → x     # 10
```

---

## 5. Module Resolution

### 5.1 Search Algorithm

When the interpreter encounters `import → name`, it resolves the module path as follows:

1. **Relative import** — if `name` starts with `./` or `../`:
   - Resolve relative to the directory containing the current source file.
   - Requires a current file context; raises an error otherwise.

2. **Absolute import** — any other name:
   - Search each path in the **search path list** in order.
   - For each directory `dir`, try:
     1. `dir/name.tc`
     2. `dir/name` (exact, no extension added)
   - Use the first match found.
   - If no match: raise `RuntimeError: Module 'name' not found in search paths`.

### 5.2 Default Search Paths

1. The current working directory (`cwd`) at interpreter startup.
2. Each colon-separated path in the `TXTCODE_MODULE_PATH` environment variable (if set).

```sh
TXTCODE_MODULE_PATH=/usr/lib/txtcode:/home/user/modules txtcode run main.tc
```

Additional paths may be added programmatically via the `ModuleResolver` API.

### 5.3 Import Syntax

```txtcode
# Import entire module; access via module_name.symbol
import → math

# Import specific names into local scope
import → sqrt, pow from math

# Import with alias
import → math as m
store → x → m.sqrt(16.0)
```

### 5.4 Circular Import Detection

The resolver tracks the current import chain. If a module being loaded is already in the chain, a `RuntimeError` is raised:

```
RuntimeError: Circular import detected: a.tc -> b.tc -> a.tc
Hint: Remove the circular dependency between modules.
```

### 5.5 Module Caching

Each module is loaded and parsed **at most once** per interpreter session. Subsequent imports of the same resolved path return the cached `Program` AST directly.

### 5.6 Version Compatibility

Module source files may include a version header:

```txtcode
## @version 0.1.0
```

If the module's declared version is older than the current runtime version, the AST is automatically migrated via the compatibility layer before execution.
If the version is declared **incompatible**, loading raises a `RuntimeError` with the reason.

### 5.7 Export

Modules explicitly export symbols using `export`:

```txtcode
# math.tc
define → add → (a: int, b: int) → int
  return → a + b
end
export → add
```

Importing a module that does not export a symbol raises a `RuntimeError` at runtime.

---

## 6. Runtime Limits

### 6.1 Source File Size

The CLI enforces a **10 MB** maximum source file size. Files exceeding this limit are rejected before parsing with:

```
Error: File 'path' is too large (N bytes). Maximum allowed: 10485760 bytes
```

### 6.2 Execution Timeout

A timeout policy is available and configurable. There is **no default timeout** — programs run to completion unless a timeout is explicitly set via API or future CLI flag.

When a timeout is configured and exceeded, the runtime raises:

```
RuntimeError: Maximum execution time exceeded: N seconds (max: M seconds)
```

### 6.3 Call Stack

The call stack depth limit is enforced at 50 in all VMs as of v0.4. Deep recursion beyond this limit returns a `RuntimeError` instead of exhausting the host process stack.

**Recommended limit for well-behaved programs:** ≤ 10 000 nested calls.

### 6.4 Memory

Memory is managed by Rust's ownership system combined with reference counting for shared values. There is **no explicit memory limit** in v0.4. The process is bounded by the host OS.

`MemoryManager` (stub in v0.4) is a placeholder for future GC integration.

### 6.5 Integer Overflow

Integer arithmetic uses Rust `i64`. All arithmetic (`+`, `-`, `*`, `**`) in both the AST VM and bytecode VM uses Rust's `checked_*` methods as of v0.4, returning a `RuntimeError` on overflow instead of wrapping or panicking.

### 6.6 Float Semantics

Float arithmetic follows IEEE 754 strictly. The runtime does not raise errors for:
- `Infinity` (result of e.g. `1.0 / 0.0` in float context)
- `-Infinity`
- `NaN`

These propagate silently. Use `math.is_nan()` and `math.is_finite()` to check.

### 6.7 Rate Limiting (Capability System)

When capabilities include rate limits (e.g. `"net.connect:100/hour"`), the runtime enforces them via the policy engine. Exceeding the rate limit raises a `RuntimeError` with details:

```
RuntimeError: Rate limit exceeded for net.connect: 100 per 3600s
```

### 6.8 Safe Mode

When launched with `--safe-mode`, the `exec()` stdlib function is disabled.
`--allow-exec` re-enables it and overrides `--safe-mode`.

---

## 7. Error Semantics

### 7.1 Error Structure

All runtime errors are instances of `RuntimeError`:

```
RuntimeError {
  message:     string     -- human-readable description
  hint:        string?    -- optional resolution hint
  stack_trace: CallFrame* -- ordered list of call frames (innermost last)
}
```

`CallFrame` contains:
- `function_name`: name of the function, or `"<main>"` for top-level code
- `line`: 1-based line number in source
- `column`: 1-based column number

### 7.2 Error Display

```
RuntimeError: Division by zero
  (Hint: Check divisor before dividing)

Stack trace:
  3: divide at line 5, column 12
  2: calculate at line 12, column 8
  1: <main> at line 20, column 1
```

### 7.3 Try / Catch / Finally

```txtcode
try
  store → result → divide(10, 0)
catch → err
  print → err          # prints error message string
  print → "Failed"
finally
  print → "Always runs"
end
```

- The `catch` variable is bound to the **error message string** (not the full `RuntimeError` object).
- Access `err` as a string inside the catch block.
- `finally` runs regardless of whether an error occurred, and regardless of whether the catch block itself raises.
- If no `catch` is present and an error occurs, `finally` runs, then the error propagates.
- Re-raising from inside `catch` is done by not catching the new error: any error raised in `catch` propagates normally.

### 7.4 Propagation Rules

1. An uncaught `RuntimeError` propagates up the call stack until a `try/catch` handles it.
2. If it reaches the top-level without being caught, the interpreter prints it to `stderr` and exits with code `1`.
3. Errors from async functions propagate when the `Future` is `await`-ed.

### 7.5 Standard Error Conditions

| Condition | Message |
|-----------|---------|
| Undefined variable | `"Undefined variable: name"` |
| Division by zero | `"Division by zero"` |
| Modulo by zero | `"Modulo by zero"` |
| Index out of bounds | `"Index out of bounds"` |
| Key not found in map | `"Key not found: key"` |
| Undefined function | `"Function not found: name"` |
| Type mismatch in operator | `"Invalid operands for [operation]"` |
| Negative int exponent | `"Negative exponent not supported for integers"` |
| Module not found | `"Module 'name' not found in search paths"` |
| Circular import | `"Circular import detected: a -> b -> a"` |
| Assert failure | `"Assertion failed"` or custom message |
| Capability violation | `"Capability [cap] is forbidden"` |
| Timeout exceeded | `"Maximum execution time exceeded"` |
| File too large | `"File 'path' is too large"` |
| Stack overflow (OS) | Process abort — not catchable in v0.4 |

### 7.6 Assert

```txtcode
assert → x > 0
assert → x > 0, "x must be positive"
```

Equivalent to:
```txtcode
if → not (x > 0)
  # raise RuntimeError("Assertion failed") or RuntimeError(message)
end
```

Assert is always evaluated — there is no release-mode stripping in v0.4.

### 7.7 Result Pattern (Idiomatic)

While there is no language-level `Result<T, E>` syntax type in v0.4, the convention is:

```txtcode
define → safe_divide → (a: int, b: int)
  if → b == 0
    return → {"ok": false, "error": "Division by zero"}
  end
  return → {"ok": true, "value": a / b}
end

store → r → safe_divide(10, 2)
if → r["ok"]
  print → r["value"]
else
  print → "Error: " + r["error"]
end
```

A native `Result<T, E>` type is planned for a future release.

---

---

## Appendix F — Security Guarantees (v0.4)

This section documents what the v0.4 runtime does and does not enforce, so users can make
informed decisions about running Txt-code scripts.

### F.1 Enforced in v0.4

| Guarantee | Mechanism | Notes |
|-----------|-----------|-------|
| **Permission declarations** | `allowed`/`forbidden` in functions checked by AST VM | Capability violations raise RuntimeError |
| **Safe mode** (`--safe-mode`) | Disables `exec()` stdlib call | Can be overridden by `--allow-exec` |
| **Rate limiting** | Policy engine applies capability rate limits | e.g. `"net.connect:100/hour"` |
| **Audit trail** | All major events logged to `~/.txtcode/logs/` | Includes call targets, timestamps, errors |
| **Capability scoping** | `permission → fs.read:/tmp/*` restricts to path glob | Checked per-call in AST VM |
| **Circular import detection** | Raises RuntimeError with full chain | Prevents infinite module loops |
| **Division/modulo by zero** | Always raises RuntimeError | Both AST VM and bytecode VM |
| **Source file size limit** | 10 MB max, rejected before parsing | Prevents resource exhaustion |
| **Permission checking in stdlib** | `call_function_with_combined_traits` checks net/IO/sys/exec | Before call via PermissionChecker trait |

### F.2 Not Yet Enforced (v0.4 status)

| Gap | Impact | Status |
|-----|--------|--------|
| **Memory limits** | No explicit heap limit; bounded by host OS only | Future |
| **`?()` optional call on non-null non-function** | Raises RuntimeError in bytecode VM | v0.5 |
| **Source comments in migrate** | `#` comments are not preserved through AST printer | v0.4 |
| **`return →` inside nested blocks** | Does not propagate through `match`, `if`, or `for` to caller | v0.5 |

### F.2a Known Runtime Limitation — `return →` in Nested Blocks

In v0.4, `return →` only propagates to the caller when it appears **at the top level of the
function body**. A `return →` inside a `match` case, `if` branch, `for` loop, or `try` block
is silently swallowed — execution continues after the enclosing block, and the function returns
whatever its final top-level expression evaluates to (`null` if nothing else).

**Does not work:**
```txtcode
define → f → (x)
  if → x > 0
    return → "positive"   ## BUG: not propagated to caller
  end
  return → "other"
end
```

**Workaround — store then return at top level:**
```txtcode
define → f → (x)
  store → result → "other"
  if → x > 0
    store → result → "positive"
  end
  return → result           ## OK: top-level return
end
```

The same pattern applies to `match` cases and `try`/`catch` bodies. Always accumulate the
result in a variable and place a single `return → result` as the last statement of the function.

### F.3 Safe Mode Guarantees

When `--safe-mode` is active:
- `exec()`, `spawn()`, and `kill()` stdlib calls return RuntimeError before executing.
- `sys.exec` capability is treated as denied regardless of `permission` declarations.
- All other capabilities (fs, net, tool) remain subject to their declared permissions.

Safe mode **does not** restrict:
- Network access (use `forbidden → ["net.*"]` in function declarations for that)
- File system reads/writes (use `forbidden → ["fs.*"]` for that)
- Loading and executing modules

### F.4 Capability Declaration vs Permission Checker

Two independent mechanisms enforce permissions in v0.4:

1. **Capability declarations** (`allowed`/`forbidden`) — declared in function bodies, enforced
   by the AST VM at runtime when the function executes.
2. **PermissionChecker** — a Rust trait implemented by the VM executor, checked by
   `StdLib::call_function_with_combined_traits` before routing to stdlib modules.

Both must allow an operation for it to proceed in the AST VM. Either mechanism alone is
sufficient to block a call. The bytecode VM does not implement either mechanism.

---

## Appendix E — Migration Tooling Reference

### E.1 Overview

`txtcode migrate` assists with updating Txt-code source files when syntax or semantics change between versions.

### E.2 Supported Transformations (v0.1 → v0.2)

| Change | From (v0.1) | To (v0.2) | Auto-migrated? |
|--------|-------------|-----------|----------------|
| Deprecated space syntax | `define name (params)` | `define → name → (params)` | Warning emitted |
| Module version header | `## @version 0.1.0` | `## @version 0.2.0` | Reported |

No breaking syntax changes were introduced between v0.1 and v0.2. Migration is advisory.

### E.3 Usage

```bash
# Validate files without modifying them (default: dry-run is ON)
txtcode migrate --files main.tc lib.tc

# Migrate an entire directory
txtcode migrate --directory src/

# Specify versions explicitly
txtcode migrate --files main.tc --from 0.1.0 --to 0.2.0

# Write migrated source back to file (v0.4+)
txtcode migrate --files main.tc --dry-run=false
```

### E.4 Current Limitations (v0.4)

- **Source code regeneration** is now implemented via the AST printer. Files are written when
  `--dry-run=false`. The written source uses canonical syntax (`store →`, `define →`, etc.)
  and may differ in formatting from the original.
- **Version auto-detection** reads `# version: X.Y.Z` from the first 20 lines of the file.
  If not present, defaults to 0.1.0. Add a `# version: 0.2.0` header to your files for
  accurate detection without needing `--from`.
- **Source comments** (lines starting with `#`) are not preserved through the AST printer,
  as the AST does not store comment nodes. Back up your files before migrating.

The AST-to-source printer is implemented as of v0.4, enabling actual file transformation via `--dry-run=false`.

---

## Appendix D — Execution Engine Reference

### D.1 Engine Overview (v0.4)

Txt-code has two execution engines. Understanding which one is active is important for security and
compatibility guarantees.

| Engine | Entry point | Policy/audit | Permission checks | Status |
|--------|-------------|--------------|-------------------|--------|
| **AST VM** (`VirtualMachine`) | `txtcode run`, `txtcode repl`, `txtcode <file>` | Full | Full | Production |
| **Bytecode VM** (`BytecodeVM`) | `txtcode compile` output, debugger, benchmarks | None | None | Experimental |

### D.2 AST VM — Production Engine

`VirtualMachine` (in `src/runtime/vm.rs`) is the primary execution engine used by `txtcode run` and the REPL.

- All permission declarations (`allowed`, `forbidden`) are enforced at the call site.
- All stdlib calls for net/IO/sys go through `PermissionChecker` before execution.
- Every executed statement is recorded in the audit trail (`src/runtime/audit.rs`).
- Policy constraints (rate limits, timeouts) are applied by the policy engine.
- Intent and AI-hint annotations are visible to the audit log.

This engine is the **only** execution path with production security guarantees in v0.4.

### D.3 Bytecode VM — Experimental Engine

`BytecodeVM` (in `src/runtime/bytecode_vm.rs`) is a stack-based interpreter for compiled `.txtc` files.

**v0.4 status:** Significant improvements shipped — `break`/`continue`, `for x in arr`, `repeat N`,
`match`, `++`/`--`, string interpolation, user-defined functions, and `ImportModule` are all
implemented. Remaining limitations:
- No permission enforcement — all stdlib calls bypass `PermissionChecker`.
- No audit logging — executed instructions are not recorded.
- No capability scoping — `allowed`/`forbidden` declarations are ignored.

**Recommended use in v0.4:** benchmarking (`txtcode bench`), debugger step-through, and
offline bytecode inspection. **Do not** use compiled `.txtc` output in production environments
where permission enforcement or audit logging is required.

**Planned for v0.5:** BytecodeVM permission and audit parity with AST VM.

### D.4 Choosing an Engine

```
┌─────────────────────────────────────────────────────────────┐
│ Use case                       Recommended command          │
├─────────────────────────────────────────────────────────────┤
│ Production script execution    txtcode run <file>           │
│ Interactive development        txtcode repl                 │
│ Security-critical automation   txtcode run --safe-mode      │
│ Offline inspection / bench     txtcode compile + bench      │
│ Debugging with breakpoints     txtcode debug <file>         │
└─────────────────────────────────────────────────────────────┘
```

---

## Appendix A — Complete Operator Quick Reference

```
Arithmetic:   +  -  *  /  %  **
Unary:        -  not  ~
              ++  --  (syntax reserved; not yet implemented in all backends — use x+1/x-1)
Comparison:   ==  !=  <  >  <=  >=
Logical:      and  or  not
Bitwise:      &  |  ^  <<  >>  ~
Null-safe:    ??  ?.  ?[]  ?()  (AST VM only; bytecode VM emits Nop — see §1.11, §3.6)
Ternary:      ? :
Arrow:        →  (alias: ->)
```

## Appendix B — Type System Quick Reference

```
Primitives:    int  float  string  char  bool  null
Collections:   array[T]  map[T]  set[T]
User-defined:  struct Name  enum Name
Callable:      function (params) → T
Async:         Future<T>
Generic:       define name<T, U>(...)
```

## Appendix C — Capability Reference

```
fs.read                  Read filesystem
fs.write                 Write filesystem
fs.delete                Delete files
fs.*                     All filesystem operations
net.connect              Outbound TCP/HTTP connections
net.listen               Bind a port
net.*                    All network operations
sys.exec                 Execute external processes
sys.env                  Read environment variables
sys.*                    All system calls
tool:name                Execute a specific tool
tool:*                   Execute any tool
*.read                   Read on any resource
*.*                      Unrestricted
resource.action:scope    Scoped (e.g. fs.read:/tmp/*)
```
