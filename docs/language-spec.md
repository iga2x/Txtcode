# Txt-code Language Specification — v3.0.0

> **Status:** v3.0.0 stable. The public API, permission model, stdlib function names, and core syntax will not have breaking changes until v4.0.
> See [CHANGELOG](https://github.com/iga2x/txtcode/blob/main/CHANGELOG.md) for full version history.
>
> **Known limitations (v3.0.0):**
> - Generic type enforcement is parse-only — type params are stored in AST but erased at runtime
> - `++arr[0]` / `--arr[0]` on non-identifier targets unsupported — use `store → arr[0] → arr[0] + 1`
> - macOS/Windows OS-level anti-debug checks not yet implemented (Linux only)
> - Bytecode VM is experimental (`--features bytecode`) — float literals unimplemented
> - LLVM/native backend deferred — see `docs/dev-plan.md` §Deferred

---

## Table of Contents

1. [Syntax](#1-syntax)
2. [Values and Types](#2-values-and-types)
3. [Operator Behaviour](#3-operator-behaviour)
4. [Function Rules](#4-function-rules)
5. [Module Resolution](#5-module-resolution)
6. [Runtime Limits](#6-runtime-limits)
7. [Error Semantics](#7-error-semantics)

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

| Keyword | Purpose |
|---------|---------|
| `store` / `let` | Variable assignment (`let` is an alias) |
| `const` | Immutable constant |
| `print` / `out` | Print to stdout (`out` is an alias) |
| `define` / `def` | Function definition (`def` is an alias) |
| `return` / `ret` | Return from function (`ret` is an alias) |
| `yield` | Emit a value from a generator function |
| `if` | Conditional |
| `elseif` / `elif` | Else-if branch (`elif` is an alias) |
| `else` | Else branch |
| `end` | Close a block |
| `while` | While loop |
| `do` | Do-while loop |
| `for` / `foreach` | For loop (`foreach` is an alias) |
| `in` | For-loop iteration keyword |
| `repeat` / `times` | Counted loop |
| `break` | Exit loop |
| `continue` | Skip to next loop iteration |
| `match` / `switch` | Pattern match (`switch` is an alias) |
| `case` | Match arm |
| `try` | Error handling block |
| `catch` | Catch handler |
| `finally` | Always-run cleanup block |
| `and` / `or` / `not` | Logical operators |
| `true` / `false` / `null` | Literal values |
| `enum` | Enum type declaration |
| `struct` | Struct type declaration |
| `impl` | Attach methods to a struct |
| `protocol` | Protocol (interface) declaration |
| `implements` | Struct protocol list (inside `struct` declaration) |
| `import` / `use` | Import a module (`use` is an alias) |
| `from` | Import source: `from "mod" import X` |
| `as` | Import alias: `import X as Y` |
| `export` | Export names from a module |
| `async` | Async function or nursery |
| `await` | Await a future |
| `nursery` | Structured concurrency block |
| `intent` / `doc` | Function intent annotation (`doc` is an alias) |
| `hint` / `ai_hint` | Function hint annotation (`ai_hint` is an alias) |
| `allowed` | Capability whitelist inside function |
| `forbidden` | Capability blacklist inside function |
| `permission` | Permission statement |
| `assert` | Runtime assertion |
| `int` / `float` / `string` / `bool` / `char` | Type keywords |
| `array` / `map` / `set` | Collection type keywords |
| `step` / `to` / `then` | Loop / conditional helper keywords |

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
end
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

Guard expressions (`case → pattern if condition`) are evaluated only when the pattern matches. Variables bound by the pattern are in scope inside the guard expression. A case arm is only entered when both the pattern matches **and** the guard evaluates to `true`.

**Or-patterns** — match any of several alternatives in a single arm:
```txtcode
match → x
  case → 1 | 2 | 3
    print → "low"
  case → _
    print → "other"
end
```

**Range patterns** (inclusive) — match a value inside an integer range:
```txtcode
match → score
  case → 90..=100
    print → "A"
  case → 80..=89
    print → "B"
  case → _
    print → "other"
end
```

Both or-patterns and range patterns work in the AST VM and the bytecode VM.

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
Two equivalent syntaxes:
```txtcode
# Parens form (canonical — preferred)
struct Point(x: int, y: int)

# Block form (also accepted)
struct → Point
  x: int
  y: int
end
```

**Struct literal** — create an instance:
```txtcode
store → p → Point { x: 3, y: 4 }
```
All declared fields must be provided.

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

#### Yield (Generator Functions)
Inside a `define` body that contains at least one `yield`, the function becomes a generator. Calling it returns an array of all yielded values.
```txtcode
define → counter → (n)
  store → i → 0
  while → i < n
    yield → i
    store → i → i + 1
  end
end

store → nums → counter(3)   # [0, 1, 2]
```
`yield` may only appear inside a `define` body. Mixing `yield` and `return` in the same function is allowed; `return` stops iteration early.

#### Protocol Declaration
Declares a named interface that structs can implement.
```txtcode
protocol → Printable
  to_string() → string
end

protocol → Comparable
  compare(other) → int
end
```
Protocol bodies list method signatures: `method_name(param_types) → return_type`. Parameter names are optional.

#### Struct with Protocol Implementation
```txtcode
struct Point(x: int, y: int) implements Printable

impl → Point
  define → to_string → (self) → string
    return → f"Point({self.x}, {self.y})"
  end
end
```
The `implements` keyword lists protocols after the struct declaration. The type checker verifies that all protocol methods are implemented in the corresponding `impl` block.

#### Type Alias
Creates an alias for an existing type. Purely documentary — the runtime treats the alias as the underlying type.
```txtcode
type → UserId → int
type → Hostname → string
type → Matrix → array[array[float]]
```

#### Named Error
Declares a named error constant. A named error is a **string value** at runtime — `error → NotFound → "Resource not found"` binds `NotFound` to the string `"Resource not found"`. It can be passed to `err()` or compared with `==`.

```txtcode
error → NotFound → "Resource not found"
error → Unauthorized → "Access denied"

# Raise using the name:
return → err(NotFound)

# Pattern match by value (string comparison):
match → unwrap_err(result)
  case → "Resource not found"
    print → "not found"
  case → _
    print → "other error"
end
```

Named errors are **not distinct types** — the runtime does not distinguish `err(NotFound)` from `err("Resource not found")`. They are a documentation and readability convention.

#### Async For Loop
`async → for` iterates over an async generator (or any `Future<Array>`). The future is resolved automatically before the loop body executes.
```txtcode
async → for → n in numbers()
  total += n
end
```
This is syntactic sugar: the runtime awaits the generator's `Future` and then iterates the resulting array.

#### Async Nursery (Structured Concurrency)
A nursery block spawns tasks and waits for all of them to complete before proceeding. If any task raises an error, all remaining tasks are cancelled and the error is re-raised.
```txtcode
async → nursery
  nursery_spawn(() → fetch_data("https://api.example.com/a"))
  nursery_spawn(() → fetch_data("https://api.example.com/b"))
end
# both tasks have completed (or one failed) when execution reaches here
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
  | StructName "{" field: expr, ... "}"   # struct literal
  | expression "?"                        # propagate error (see §1.17)
```

### 1.9 Slice Expressions

```txtcode
store → sub → arr[1:4]        # elements at indices 1, 2, 3 (end is exclusive)
store → evens → arr[::2]      # every second element (step=2)
store → rev → arr[::-1]       # reversed (negative step)
store → tail → arr[-3:]       # last 3 elements (negative index)
store → s → "hello"[1:4]     # "ell" (char-based, not byte-based)
store → s → "hello"[::2]     # "hlo" (every other char, step on strings supported)
```

Syntax: `target[start:end:step]`. Any of `start`, `end`, `step` may be omitted.

- **`step`** defaults to 1. `step = 0` is a runtime error.
- **Negative `step`**: iterates in reverse. `start` defaults to the last index; `end` defaults to index 0.
- **Negative `start`/`end`**: count from the end — `-1` is the last element, `-2` is second-to-last, etc.
- **Out-of-bounds indices**: raise a runtime error (no silent clamping).
- **Strings**: slices are char-based (Unicode code points, not bytes). `step` is supported on strings.
- Slices work on arrays and strings. Maps and sets do not support slicing.

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

All three operators (`?.`, `?[]`, `?()`) are fully implemented in both the AST VM and bytecode VM.

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
- Spread is only supported inside array literals (`[...]`). It is not supported in function call arguments 

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
    | function_def | return | break | continue | yield_stmt
    | if_stmt | while_stmt | do_while_stmt | for_stmt | repeat_stmt
    | match_stmt | try_stmt | assert_stmt
    | import_stmt | export_stmt | const_stmt
    | struct_def | enum_def | permission_stmt
    | impl_def | protocol_def | type_alias | named_error | nursery_stmt | async_for_stmt
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
do_while_stmt     := "do" statement* "while" ("→"|WS) expression "end"
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

struct_def        := "struct" ("→"|WS) identifier ("(" field ("," field)* ")")?
                     ("implements" identifier ("," identifier)*)? (field* "end")?
field             := identifier ":" type
enum_def          := "enum" ("→"|WS) identifier variant* "end"
variant           := identifier ("→" expression)?
permission_stmt   := "permission" "→" identifier "." identifier ("→" string_literal)?

impl_def          := "impl" ("→"|WS) identifier function_def* "end"

protocol_def      := "protocol" ("→"|WS) identifier method_sig* "end"
method_sig        := identifier "(" type_list? ")" ("→" type)?
type_list         := type ("," type)*

type_alias        := "type" ("→"|WS) identifier ("→"|WS) type

named_error       := "error" ("→"|WS) identifier ("→"|WS) string_literal

yield_stmt        := "yield" ("→"|WS) expression

nursery_stmt      := "async" ("→"|WS) "nursery" statement* "end"
async_for_stmt    := "async" ("→"|WS) "for" ("→"|WS) identifier "in" expression statement* "end"

struct_literal    := identifier "{" (identifier ":" expression ("," identifier ":" expression)*)? "}"

type              := "int" | "float" | "string" | "char" | "bool"
                   | "array" "[" type "]" | "map" "[" type "]" | "set" "[" type "]"
                   | "Future" "<" type ">"
                   | type "?"             # nullable type
                   | identifier           # named struct/enum or generic param

constraint        := "Comparable" | "Numeric" | "Printable"
type_param        := identifier (":" constraint)?

pattern           := identifier | "[" pattern_list "]"
                   | "{" (identifier ":" pattern)* ("..." identifier)? "}"
                   | identifier "(" pattern_list ")"
                   | "_"
                   | or_pattern
                   | range_pattern
                   | literal              # literal pattern (int, float, string, bool)
                   | "..." identifier     # rest pattern (in array destructure)

or_pattern        := pattern ("|" pattern)+
range_pattern     := integer "..=" integer
```

### 1.16 impl Blocks (Struct Methods)

The `impl` statement attaches methods to a previously declared struct type.

```txtcode
struct → StructName
  # fields ...
end

impl → StructName
  define → method_name → (self, param1, param2)
    # body; self is the struct instance
    return → self.field + param1
  end

  define → other_method → (self)
    return → self.method_name(0, 0)  ;; call another method via self
  end
end
```

**Method call syntax:** `obj.method_name(arg1, arg2)` — the runtime automatically prepends `obj` as the first argument (`self`).

**Semantics:**
- The `impl` block registers all enclosed method definitions in a per-struct method registry (`struct_methods: HashMap<struct_name, HashMap<method_name, function_value>>`).
- When `obj.method(args)` is evaluated: the runtime looks up `type_of(obj)` in the method registry, retrieves the function, and calls it with `[obj, args...]`.
- `self` is a conventional parameter name — it is not a reserved keyword.
- Methods can call other methods on the same struct via `self.other()`.
- Works in both the AST VM and the bytecode VM.

### 1.17 `?` Error Propagation Operator

Postfix `?` provides concise early-return on error inside a function body:

```txtcode
define → load → (path: string)
  store → raw → read_file(path)?    ;; returns Err if read_file fails
  store → data → json_parse(raw)?   ;; returns Err if json_parse fails
  return → data                     ;; only reached if both steps succeeded
end
```

**Semantics:**
- `expr?` evaluates `expr`.
  - If the result is `Value::Result(true, val)` (Ok): evaluates to `val` (the unwrapped inner value).
  - If the result is `Value::Result(false, err)` (Err): immediately returns `Value::Result(false, err)` from the current function (early return).
  - If the result is any other value: passes through unchanged.
- `?` is a postfix operator with higher precedence than any binary operator; it binds tightly to the expression immediately to its left.
- Works in both the AST VM (`Expression::Propagate`) and the bytecode VM (`Instruction::Propagate`).

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
| `map[T]` | `{"key": value}` | String keys only, values of type T; iteration order is insertion-order  |
| `set[T]` | `{| 1, 2, 3 |}` | Unordered, unique values |
| `struct Name` | `Name { field: value }` | Named fields, declared with `struct`; methods attached via `impl → Name` |
| `enum Name` | `Name.Variant` | Discriminated union, declared with `enum` |
| `function` | `(x) → x + 1` | First-class, captures enclosing environment |

### 2.3 Async Type

| Type | Notes |
|------|-------|
| `Future<T>` | Returned by `async` functions; resolved with `await` |

Calling an async function without `await` yields a `Future<T>`.
Calling `await` on a non-`Future` value returns the value as-is (identity pass-through).

### 2.4 Type Annotations (Optional)

Type annotations are optional everywhere. The type checker uses Hindley-Milner-style inference.

**Default mode — advisory:** `txtcode run` always runs the type checker. Errors appear as `[WARNING] type: ...` messages but do **not** halt execution. Use `--strict-types` to make type errors fatal (exit 2). Use `--no-type-check` to disable the checker entirely.

```txtcode
store → x: int → 42
store → name: string → "Alice"
define → add → (a: int, b: int) → int
  return → a + b
end
```

**Nullable types** — append `?` to allow `null` as a valid value:
```txtcode
store → user: string? → null        # can be string or null
define → find → (id: int) → User?
  # returns User or null
end
```

**Generic type parameters with constraints:**
```txtcode
define → max<T: Comparable> → (a: T, b: T) → T
  return → a > b ? a : b
end
```

Available constraints:

| Constraint | Applies to |
|-----------|-----------|
| `Comparable` | Types that support `<`, `>`, `<=`, `>=` |
| `Numeric` | Types that support `+`, `-`, `*`, `/` |
| `Printable` | Types that can be converted to `string` |

> Type parameters and constraints are parsed and stored in the AST but **type-erased at runtime**. The type checker enforces them in advisory or strict mode; the VM does not.

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
| `++` (prefix) | `int` | `int` | Implemented  |
| `--` (prefix) | `int` | `int` | Implemented  |

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

> `??` is fully implemented in the bytecode VM (`NullCoalesce` instruction). Works in both AST VM and bytecode mode.

### 3.7 Ternary

```txtcode
store → abs → x >= 0 ? x : -x
```

### 3.8 Operator Precedence (highest to lowest)

Higher level number = lower precedence (binds more loosely).
`L` = left-associative · `R` = right-associative · `—` = postfix/prefix (non-binary).

| Level | Operators | Assoc | Notes |
|-------|-----------|-------|-------|
| 1 | `()` `[]` `.` `?.` `?[]` `?()` | L | call, index, member access, optional-chain |
| 2 | postfix `?` | — | error propagation; binds to immediate left operand |
| 3 | prefix `++` `--` `-` `not` `~` | — | unary; `not` is logical NOT, `~` is bitwise NOT |
| 4 | `**` | **R** | power; right-associative: `2**3**2` = `2**(3**2)` = 512 |
| 5 | `*` `/` `%` | L | multiply, floor-divide, floor-modulo |
| 6 | `+` `-` | L | add/concat, subtract |
| 7 | `<<` `>>` | L | bitwise shift |
| 8 | `&` | L | bitwise AND |
| 9 | `^` | L | bitwise XOR |
| 10 | `\|` | L | bitwise OR |
| 11 | `<` `>` `<=` `>=` | L | comparison |
| 12 | `==` `!=` | L | equality |
| 13 | `and` | L | logical AND (short-circuit) |
| 14 | `or` | L | logical OR (short-circuit) |
| 15 | `??` | L | null-coalesce: returns left if non-null, else right |
| 16 | `? :` | R | ternary conditional |
| 17 | `\|>` | L | pipe: `x \|> f` desugars to `f(x)` |

**Key disambiguation examples:**

```txtcode
2 + 3 * 4        ;; 14  (* before +)
2 ** 3 * 4       ;; 32  (** before *, left-assoc mult: (2**3)*4)
2 ** 3 ** 2      ;; 512 (right-assoc **: 2**(3**2))
not true and false  ;; false  (not before and: (not true) and false)
1 | 2 & 3        ;; 3   (& before |: 1|(2&3))
a ?? b ? c : d   ;; (a??b) ? c : d  (?? before ternary)
x |> f |> g      ;; g(f(x))  (left-assoc pipe)
```

**Integer division and modulo use floor semantics** (result has the same sign as the divisor):
```txtcode
7 / 2      ;; 3   (not 3.5 — integer floor division)
-7 / 2     ;; -4  (floor toward −∞, not truncation toward 0)
-7 % 3     ;; 2   (floor modulo: same sign as divisor 3)
7 % -3     ;; -2  (floor modulo: same sign as divisor -3)
```

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

# With constraint:
define → max<T: Comparable> → (a: T, b: T) → T
  return → a > b ? a : b
end
```

Type parameters are listed after the function name inside `<>`. Constraints follow a colon: `<T: Comparable>`. Multiple type params: `<T, U>`. Multiple constraints per param are not supported.

Available constraints: `Comparable`, `Numeric`, `Printable` (see §2.4).

> Type parameters are **type-erased at runtime**. The type checker enforces constraints in advisory/strict mode only.

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

### 4.6 Async Functions

> `async define → fn → (args)` now spawns a real OS thread when called without `await`.
> The return value is `Value::Future`. Calling `await future` blocks the calling thread until the
> spawned task finishes. Non-future values passed to `await` are returned unchanged (identity).
> The child thread receives a snapshot of the parent's global scope.

```txtcode
async → define → fetch → (url: string) → string
  # async define spawns an OS thread — http_get runs concurrently
  store → body → await → http_get(url)
  return → body
end

store → result → fetch("https://example.com")
```

- `async define → fn → (args)` spawns an OS thread; returns `Value::Future`.
- `await expr` — if `expr` is a `Future`, blocks until the thread finishes; otherwise identity.
- `await → expr` evaluates `expr` and returns the result directly.
- No `Future<T>` type at runtime — the value is returned as-is.
- **`await_all(futures_array)`** — stdlib function that blocks until all futures in the array resolve; returns an array of results in the same order. Non-`Future` values are passed through unchanged.
- **`await_any(futures_array)`** — stdlib function that returns the value of the first future in the array to resolve. Non-`Future` values are returned immediately.

```txtcode
async → define → fetch → (url: string) → string
  return → http_get(url)
end

store → f1 → fetch("https://example.com/a")
store → f2 → fetch("https://example.com/b")
store → results → await_all([f1, f2])   ;; blocks for both; results[0] and results[1]
store → fastest → await_any([f1, f2])   ;; returns whichever resolves first
```

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

| Resource | Actions | Notes |
|----------|---------|-------|
| `fs` | `read`, `write`, `delete`, `*` | Filesystem I/O |
| `net` | `connect`, `listen`, `*` | HTTP, TCP, UDP, DNS |
| `sys` | `exec`, `env`, `*` | Process execution, environment |

> **Architecture note:** The built-in permission namespaces are `fs`, `net`, `sys`, and `process`. Hardware-specific capabilities (WiFi, BLE, raw radio, etc.) are **not built-in** — they must be accessed through the plugin/FFI system or OS-level tools via `sys.exec`. Passing `wifi.*` or `ble.*` strings to `grant_permission` raises a `RuntimeError`.

Wildcard forms: `"fs.*"` (any fs action), `"*.*"` (unrestricted).

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

The call stack depth limit is enforced at **500** across all VMs (raised from 50 in v3.0 via `stacker::maybe_grow`). Recursion or mutual calls that exceed this depth return a `RuntimeError`:

```
RuntimeError: Maximum call depth exceeded (500)
```

Design functions to stay within this budget. Deeply recursive algorithms should be rewritten iteratively.

**Tail-call optimization (TCO)** — the runtime automatically optimizes direct self-recursive tail calls (a function whose last statement is `return → fn_name(...)`). TCO replaces the call with a loop, so properly tail-recursive functions do not consume call-stack depth. The TCO loop enforces its own limit of **100,000** iterations to prevent truly infinite loops. TCO applies only to direct self-calls, not mutual recursion or calls through variables.

### 6.4 Memory

Memory is managed by Rust's ownership system combined with reference counting for shared values. There is **no explicit memory limit** by default. The process is bounded by the host OS.

`AllocationTracker` (in `src/runtime/gc.rs`) tracks allocations and enforces a configurable soft memory limit. A full arena-allocator-based GC is deferred — see docs/dev-plan.md.

### 6.5 Integer Overflow

Integer arithmetic uses Rust `i64`. All arithmetic (`+`, `-`, `*`, `**`) in both the AST VM and bytecode VM uses Rust's `checked_*` methods, returning a `RuntimeError` on overflow instead of wrapping or panicking.

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

### 7.4a `?` and `try/catch` Interaction

`?` and `try/catch` are independent mechanisms with no interaction:

| Mechanism | Scope | Interacts with `try/catch`? |
|-----------|-------|------------------------------|
| `?` | **Function boundary** — early-return from current function | **No** |
| `throw` / VM error | **Block boundary** — caught by innermost enclosing `try` | **Yes** |

**Rule 1 — `?` propagates via function return, not via catch.**
Using `?` inside a `try` block does NOT trigger the `catch` handler.
`?` exits the current *function* by emitting a `ReturnValue` signal, which bypasses all `try/catch` blocks.

```txtcode
define → risky → ()
  try
    store → r → err("oops")?   ;; ? early-returns from risky(), does NOT hit catch
    return → ok("never reached")
  catch e
    return → ok(f"caught: {e}")  ;; this block is NEVER executed when ? fires
  end
end

store → result → risky()
# result is err("oops"), NOT ok("caught: oops")
```

**Rule 2 — `throw` inside a function IS caught by a caller's `try/catch`.**
```txtcode
define → explode → ()
  throw → "boom"
end

try
  explode()         ;; throw propagates normally and is caught here
catch e
  print → e         ;; prints "boom"
end
```

**Rule 3 — `?` at the top level (outside any function) is a runtime error [E0034].**
```txtcode
store → r → err("bad")
r?   ;; RuntimeError [E0034]: ? operator used outside of a function body
```

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
| Capability / forbidden violation | `"Capability [cap] is forbidden in function [fn]"` |
| Permission not granted | `"Permission not granted: net.connect"` |
| Permission explicitly denied | `"Permission denied: fs.write"` |
| Intent violation | `"intent.violation.net.connect logged in audit trail"` |
| Timeout exceeded | `"Maximum execution time exceeded"` |
| Call depth exceeded | `"Maximum call depth exceeded (500)"` |
| File too large | `"File 'path' is too large"` |
| Stack overflow (OS) | Process abort — not catchable |

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

Assert is always evaluated — there is no release-mode stripping 

### 7.7 Result Pattern (Idiomatic)

Txt-code has a built-in `Result` value type (`Value::Result(bool, value)`). Use `ok(v)` and `err(e)` to construct results, and `is_ok(r)` / `is_err(r)` / `unwrap(r)` / `unwrap_err(r)` to inspect them.

```txtcode
define → safe_divide → (a: int, b: int)
  if → b == 0
    return → err("Division by zero")
  end
  return → ok(a / b)
end

store → r → safe_divide(10, 2)
if → is_ok(r)
  print → unwrap(r)
else
  print → "Error: " + unwrap_err(r)
end
```

### 7.8 `?` Error Propagation

The `?` postfix operator provides ergonomic early-return on error (see §1.17 for the full specification):

```txtcode
define → load_and_parse → (path: string)
  store → raw → read_file(path)?    ;; early-return Err if read_file fails
  store → data → json_parse(raw)?   ;; early-return Err if parse fails
  return → ok(data)
end

store → result → load_and_parse("config.json")
if → is_ok(result)
  print → unwrap(result)
else
  print → f"Error: {unwrap_err(result)}"
end
```

`?` is equivalent to:
```txtcode
store → __tmp → expr
if → is_err(__tmp)
  return → __tmp
end
store → value → unwrap(__tmp)
```

---

---

## Appendix F — Security Guarantees

This section documents what the runtime does and does not enforce, so users can make
informed decisions about running Txt-code scripts.

### F.1 Enforced

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

### F.2 Not Yet Enforced

| Gap | Impact | Status |
|-----|--------|--------|
| **Memory limits** | No explicit heap limit; bounded by host OS only | 
| **`?()` optional call on non-null non-function** | Raises RuntimeError in bytecode VM | 
| **Source comments in migrate** | `#` comments are not preserved through AST printer | 
| **`return →` inside nested blocks** | Does not propagate through `match`, `if`, or `for` to caller | 

### F.2a Known Runtime Limitation — `return →` in Nested Blocks

`return →` only propagates to the caller when it appears **at the top level of the
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

Two independent mechanisms enforce permissions:

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

### E.2 Supported Transformations

| Change | Before | After | Auto-migrated? |
|--------|-------------|-----------|----------------|
| Deprecated space syntax | `define name (params)` | `define → name → (params)` | Warning emitted |
| Module version header | `## @version 0.1.0` | `## @version 0.2.0` | Reported |

No breaking syntax changes were introduced between early versions. Migration is advisory.

### E.3 Usage

```bash
# Validate files without modifying them (default: dry-run is ON)
txtcode migrate --files main.tc lib.tc

# Migrate an entire directory
txtcode migrate --directory src/

# Specify versions explicitly
txtcode migrate --files main.tc --from 0.1.0 --to 0.2.0

# Write migrated source back to file
txtcode migrate --files main.tc --dry-run=false
```

### E.4 Current Limitations

- **Source code regeneration** is now implemented via the AST printer. Files are written when
  `--dry-run=false`. The written source uses canonical syntax (`store →`, `define →`, etc.)
  and may differ in formatting from the original.
- **Version auto-detection** reads `# version: X.Y.Z` from the first 20 lines of the file.
  If not present, defaults to 0.1.0. Add a `# version: 0.2.0` header to your files for
  accurate detection without needing `--from`.
- **Source comments** (lines starting with `#`) are not preserved through the AST printer,
  as the AST does not store comment nodes. Back up your files before migrating.

The AST-to-source printer is implemented, enabling actual file transformation via `--dry-run=false`.

---

## Appendix D — Execution Engine Reference

### D.1 Engine Overview

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

This engine is the **only** execution path with production security guarantees 

### D.3 Bytecode VM — Experimental Engine

`BytecodeVM` (in `src/runtime/bytecode_vm.rs`) is a stack-based interpreter for compiled `.txtc` files.

Full feature and security parity with the AST VM. Implemented:
`break`/`continue`, `for x in arr`, `repeat N`, `match`, `++`/`--`, string interpolation,
user-defined functions, `ImportModule`, and.2, the complete 6-layer security pipeline:

- Permission enforcement — all stdlib calls pass through `PermissionChecker` (grant/deny).
- Audit logging — all permission checks logged with AI metadata via `AuditTrail`.
- Intent checking — per-function `allowed`/`forbidden` action enforcement via `IntentChecker`.
- Capability scoping — time-bound authorisation tokens enforced via `CapabilityManager`.
- Policy engine — rate limiting, AI control, and max execution time via `PolicyEngine`.
- Runtime security — anti-debug, bytecode integrity hash, platform detection via `RuntimeSecurity`.

Recommendation: all execution paths, including production. Both `txtcode run <file>`
(AST VM) and `txtcode run <file.txtc>` (bytecode VM) enforce the same security guarantees.

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

---

## Appendix G — Stdlib Function Reference

### Named Functions (always available)

**Type / conversion:**
`len`, `type`, `string`, `int`, `float`, `bool`, `str`

**I/O:**
`print`, `input`, `log`

**Math:**
`max`, `min`, `abs`, `floor`, `ceil`, `round`, `sqrt`, `pow`, `log` (math variant), `sin`, `cos`, `tan`

**Collections:**
`map`, `filter`, `reduce`, `find`, `sort`, `reverse`, `concat`, `range`, `enumerate`, `zip`, `chain`

**Result type:**
`ok(v)`, `err(v)`, `is_ok(r)`, `is_err(r)`, `unwrap(r)`, `unwrap_or(r, default)`

**Encoding:**
`base64_encode`, `base64_decode`, `base32_encode`, `base32_decode`, `html_escape`

**Crypto:**
`md5`, `encrypt`, `decrypt`, `hmac_sha256`, `uuid_v4`, `secure_compare`, `pbkdf2`,
`bcrypt_hash`, `bcrypt_verify`, `ed25519_sign`, `ed25519_verify`,
`rsa_generate`, `rsa_sign`, `rsa_verify`, `jwt_sign`, `jwt_verify`, `jwt_decode`

**Async / concurrency:**
`async_run(fn)`, `async_run_scoped(fn)`, `async_run_timeout(ms, fn)`,
`await_all(futures)`, `await_any(futures)`,
`async_cancel_token()`, `async_cancel(id)`, `is_cancelled(id)`,
`async_sleep(ms)`, `async_read_file(path)`, `async_write_file(path, data)`

**Process:**
`exec(cmd)`, `exec_json(cmd)`, `exec_lines(cmd)`, `exec_status(cmd)`, `exec_pipe(cmds)`,
`spawn(cmd)`, `kill(pid)`, `signal_send(pid, sig)`, `wait(pid)`

**System info:**
`cpu_count()`, `memory_available()`, `disk_space(path)`, `platform()`, `arch()`,
`os_name()`, `os_version()`, `pid()`, `user()`, `home()`, `uid()`, `gid()`, `is_root()`

**Environment:**
`getenv(key)`, `setenv(key, val)`, `env_list()`, `args()`, `cwd()`, `chdir(path)`

**Time / date:**
`now()`, `now_utc()`, `now_local()`, `sleep(ms)`,
`format_datetime(ts, fmt, tz)`, `datetime_add(ts, n, unit)`, `datetime_diff(a, b, unit)`,
`parse_datetime(s, fmt)`

**Compression:**
`gzip_compress(data)`, `gzip_decompress(bytes)`,
`gzip_compress_string(s)`, `gzip_decompress_string(bytes)`

**Error constructors (return `Result::Err`):**
`FileNotFoundError(msg)`, `PermissionError(msg)`, `NetworkError(msg)`, `ParseError(msg)`,
`TypeError(msg)`, `ValueError(msg)`, `IndexError(msg)`, `TimeoutError(msg)`

**Capability tokens:**
`grant_capability(resource, action, scope?, expires_in?)` — issue a token (`expires_in`: `"30s"`, `"5m"`, `"1h"`, integer seconds, or `null`/omit for no TTL),
`use_capability(token_id)`, `revoke_capability(token_id)`, `capability_valid(token_id) → bool`

**Test assertions:**
`assert(cond)`, `assert_eq(a, b)`, `assert_ne(a, b)`,
`assert_error(fn)`, `assert_type(v, type_str)`,
`assert_contains(haystack, needle)`, `assert_approx(a, b, delta)`, `expect_error(fn, msg)`

**Plugin / FFI / WASM:**
`plugin_load(path)`, `plugin_functions(handle)`, `plugin_call(handle, fn, args)`,
`ffi_load(path)`, `ffi_call(handle, fn, args)`, `ffi_close(handle)`,
`wasm_load(path)`, `wasm_call(handle, fn, args)`, `wasm_close(handle)`

---

### Prefix-Routed Functions

Functions whose names begin with any of these prefixes are routed to the corresponding stdlib module. Exact function lists are in `packages/README.md`.

| Prefix | Module | Examples |
|--------|--------|---------|
| `str_` | String operations | `str_pad_left`, `str_truncate`, `str_to_upper`, `str_split_lines` |
| `math_` | Math | `math_random`, `math_random_float`, `math_clamp`, `math_lerp` |
| `array_` | Array operations | `array_flatten`, `array_unique`, `array_chunk`, `array_slice` |
| `set_` | Set operations | `set_union`, `set_intersection`, `set_difference` |
| `http_` | HTTP client | `http_get`, `http_post`, `http_put`, `http_delete`, `http_request` |
| `ws_` | WebSocket | `ws_connect`, `ws_send`, `ws_recv`, `ws_close`, `ws_serve` |
| `tcp_` / `udp_` | Raw sockets | `tcp_connect`, `tcp_send`, `tcp_recv`, `udp_send` |
| `db_` | Database | `db_connect`, `db_query`, `db_execute`, `db_commit`, `db_rollback`, `db_transaction` |
| `json_` | JSON | `json_parse`, `json_stringify`, `json_validate`, `json_path` |
| `xml_` | XML | `xml_parse`, `xml_stringify`, `xml_encode` |
| `yaml_` | YAML | `yaml_parse`, `yaml_stringify` |
| `toml_` | TOML | `toml_parse`, `toml_stringify` |
| `csv_` | CSV | `csv_read`, `csv_write`, `csv_stream_reader`, `csv_read_row` |
| `path_` | File paths | `path_join`, `path_dirname`, `path_basename`, `path_exists` |
| `regex_` | Regex | `regex_match`, `regex_find_all`, `regex_replace`, `regex_compile` |
| `url_` | URL encoding | `url_encode`, `url_decode`, `url_parse` |
| `sha_` / `crypto_` | Hashing | `sha256`, `sha512`, `crypto_random_bytes` |
| `async_` | Async helpers | `async_run`, `async_sleep`, `async_cancel_token` |
| `log_` | Logging | `log_info`, `log_warn`, `log_error`, `log_debug` |
| `test_` | Test helpers | `test_run`, `test_suite`, `test_case` |
| `time_` | Time utilities | `time_parse`, `time_format`, `time_unix` |

> Unknown prefix functions raise `RuntimeError: unknown function 'xyz'`. All prefix-routed functions are subject to the same permission checks as named functions.

---

## Appendix H — Error Code Reference

Error codes appear in diagnostic messages to uniquely identify the error class. They are informational and intended for tooling (linters, LSP, CI).

| Code | Category | Message / Condition |
|------|----------|---------------------|
| E0012 | Runtime | Division or modulo by zero |
| E0016 | Type | Struct field type mismatch: `'Point.x' expected Int, got string` |
| E0034 | Runtime | `?` operator used outside of a function body |
| E0051 | Advisory | `async` function executes synchronously (shown as `[WARNING]`) |
| E0060 | Type (planned) | Annotated assignment type violation (`store → x: int → "hello"`) — not yet enforced at runtime; see `docs/dev-plan.md` §4.6 |
