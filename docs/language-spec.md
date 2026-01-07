# Txt-code Language Specification

## Overview

Txt-code is a simple, memorable, security-focused programming language designed for both cybersecurity tools and general development. It features a hybrid syntax that supports both arrow-based (`action → data`) and space-based (`action data`) patterns, making it easy to learn and remember.

## Design Principles

1. **Simplicity**: Minimal syntax with consistent patterns
2. **Memorability**: Repetitive patterns that are easy to remember
3. **Security**: Built-in obfuscation and protection features
4. **Flexibility**: Support for both functional and imperative programming
5. **Safety**: Memory-safe with garbage collection and type checking

## Syntax Overview

Txt-code supports a hybrid syntax style:
- Arrow-based: `action → data` (preferred for clarity)
- Space-based: `action data` (shorthand alternative)

Both styles are equivalent and can be mixed in the same program.

## Data Types

### Primitive Types

- **Integer** (`int`): 64-bit signed integers
  - Examples: `42`, `-10`, `0xFF`, `0b1010`
  
- **Float** (`float`): 64-bit IEEE 754 floating-point numbers
  - Examples: `3.14`, `-0.5`, `1.0e10`
  
- **String** (`string`): UTF-8 encoded strings
  - Examples: `"Hello"`, `'World'`, `"Multi\nline"`
  - Escape sequences: `\n`, `\t`, `\r`, `\\`, `\"`, `\'`
  
- **Boolean** (`bool`): Logical values
  - Values: `true`, `false`
  
- **Null** (`null`): Absence of value
  - Value: `null`

### Composite Types

- **Array** (`array`): Ordered collection of values
  - Example: `[1, 2, 3]`, `["a", "b", "c"]`
  - Zero-indexed, dynamically sized
  
- **Map** (`map`): Key-value pairs
  - Example: `{"key": "value", "num": 42}`
  - Keys must be strings
  - Values can be any type

## Variables

Variables are dynamically typed but can have optional type annotations.

```txtcode
# Type inference
store → x → 42
store → name → "Alice"
store → active → true

# Explicit typing
store → count: int → 10
store → price: float → 19.99
store → items: array → [1, 2, 3]
```

Variable names must start with a letter or underscore, followed by letters, digits, or underscores.

## Operators

### Arithmetic Operators
- `+` Addition
- `-` Subtraction
- `*` Multiplication
- `/` Division
- `%` Modulo
- `**` Exponentiation

### Comparison Operators
- `==` Equal
- `!=` Not equal
- `<` Less than
- `>` Greater than
- `<=` Less than or equal
- `>=` Greater than or equal

### Logical Operators
- `and` Logical AND
- `or` Logical OR
- `not` Logical NOT

### Bitwise Operators
- `&` Bitwise AND
- `|` Bitwise OR
- `^` Bitwise XOR
- `<<` Left shift
- `>>` Right shift
- `~` Bitwise NOT

### Operator Precedence (highest to lowest)
1. `()` (parentheses)
2. `**` (exponentiation)
3. `*`, `/`, `%` (multiplicative)
4. `+`, `-` (additive)
5. `<<`, `>>` (bitwise shifts)
6. `&` (bitwise AND)
7. `^` (bitwise XOR)
8. `|` (bitwise OR)
9. `<`, `>`, `<=`, `>=` (comparison)
10. `==`, `!=` (equality)
11. `and` (logical AND)
12. `or` (logical OR)
13. `→` (arrow operator, for function calls and assignments)

## Control Flow

### Conditional Statements

```txtcode
# Simple if
if → condition
  print → "True"
end

# If-else
if → age > 18
  print → "Adult"
else
  print → "Minor"
end

# If-elseif-else
if → score >= 90
  print → "A"
elseif → score >= 80
  print → "B"
else
  print → "C"
end
```

### Loops

```txtcode
# Repeat N times
repeat → 5 times
  print → "Hello"
end

# While loop
while → count < 10
  print → count
  store → count → count + 1
end

# For loop (range)
for → i in range(1, 10)
  print → i
end

# For loop (array)
for → item in items
  print → item
end

# Break and continue
while → true
  if → condition
    break
  end
  if → skip_condition
    continue
  end
end
```

### Pattern Matching

```txtcode
match → value
  case → 0
    print → "Zero"
  case → n if n > 0
    print → "Positive: " + n
  case → _ → "Negative"
end
```

## Functions

### Function Definition

```txtcode
# Simple function
define → greet → (name)
  return → "Hello, " + name
end

# Function with type annotations
define → add → (a: int, b: int) → int
  return → a + b
end

# Function with multiple parameters
define → create_user → (name: string, age: int, active: bool)
  return → {"name": name, "age": age, "active": active}
end
```

### Function Calls

```txtcode
# Call function
store → result → greet("Alice")
store → sum → add(5, 3)

# Call with arrow syntax
call → greet → ("Bob")
call → add → (10, 20)
```

### Anonymous Functions (Lambdas)

```txtcode
store → square → (x) → x * x
store → doubled → [1, 2, 3].map((x) → x * 2)
```

## Modules

### Module Definition

```txtcode
# In math.txt
define → add → (a, b)
  return → a + b
end

define → multiply → (a, b)
  return → a * b
end
```

### Module Import

```txtcode
# Import entire module
import → math

# Import specific functions
import → add, multiply from math

# Import with alias
import → math as m

# Use imported functions
store → result → math.add(5, 3)
store → product → multiply(4, 7)
```

## Comments

```txtcode
# Single-line comment

## Multi-line comment
   Can span multiple lines
##

# Comments can appear anywhere
store → x → 42  # This is a comment
```

## Error Handling

```txtcode
# Try-catch
try
  store → result → divide(10, 0)
catch → error
  print → "Error: " + error.message
end

# Result type (optional)
store → result → safe_divide(10, 0)
if → result.is_ok()
  print → result.value()
else
  print → "Error: " + result.error()
end
```

## Security Features

### Built-in Obfuscation

All Txt-code programs are automatically obfuscated when compiled:
- Variable name mangling
- Control flow flattening
- String encryption
- Dead code insertion

### Secure Operations

```txtcode
# Encryption
store → encrypted → encrypt(data, key)
store → decrypted → decrypt(encrypted, key)

# Hashing
store → hash → sha256(data)
store → hmac → hmac_sha256(data, key)

# Digital signatures
store → signature → sign(data, private_key)
store → valid → verify(data, signature, public_key)
```

## Standard Library

### Core Functions

- `print(value)` - Print to stdout
- `input(prompt)` - Read from stdin
- `len(collection)` - Get length
- `type(value)` - Get type name
- `str(value)` - Convert to string
- `int(value)` - Convert to integer
- `float(value)` - Convert to float
- `bool(value)` - Convert to boolean

### Array Operations

- `array.append(item)` - Add item to end
- `array.insert(index, item)` - Insert at index
- `array.remove(index)` - Remove at index
- `array.map(func)` - Transform elements
- `array.filter(func)` - Filter elements
- `array.reduce(func, initial)` - Reduce to single value

### String Operations

- `string.split(delimiter)` - Split string
- `string.join(array)` - Join array
- `string.replace(old, new)` - Replace substring
- `string.contains(substring)` - Check containment
- `string.starts_with(prefix)` - Check prefix
- `string.ends_with(suffix)` - Check suffix

### Math Operations

- `math.abs(x)` - Absolute value
- `math.min(a, b)` - Minimum
- `math.max(a, b)` - Maximum
- `math.sqrt(x)` - Square root
- `math.sin(x)`, `math.cos(x)`, `math.tan(x)` - Trigonometry
- `math.log(x)`, `math.exp(x)` - Logarithms

## Examples

### Hello World

```txtcode
print → "Hello, World!"
```

### Calculator

```txtcode
define → calculate → (a, op, b)
  match → op
    case → "+"
      return → a + b
    case → "-"
      return → a - b
    case → "*"
      return → a * b
    case → "/"
      return → a / b
    case → _
      return → null
  end
end

store → result → calculate(10, "+", 5)
print → result
```

### File Operations

```txtcode
# Read file
store → content → read_file("data.txt")
print → content

# Write file
write_file("output.txt", "Hello, World!")

# Check if file exists
if → file_exists("data.txt")
  print → "File exists"
end
```

### Network Operations

```txtcode
# HTTP GET
store → response → http_get("https://api.example.com/data")
print → response.body()

# TCP connection
store → conn → tcp_connect("example.com", 80)
conn.send("GET / HTTP/1.1\r\n\r\n")
store → data → conn.receive()
conn.close()
```

## Version Compatibility

Txt-code supports running code written in older versions:
- Semantic versioning (major.minor.patch)
- Automatic migration tools
- Backward compatibility guarantees
- Deprecation warnings

## Grammar (BNF-like)

```
program := statement*

statement := 
  | assignment
  | function_def
  | if_statement
  | while_statement
  | for_statement
  | repeat_statement
  | match_statement
  | return_statement
  | break_statement
  | continue_statement
  | try_statement
  | import_statement
  | expression

assignment := "store" ("→" | WS) identifier (":" type)? ("→" | WS) expression

function_def := "define" ("→" | WS) identifier ("→" | WS) "(" params? ")" ("→" type)? statement* "end"

if_statement := "if" ("→" | WS) expression statement* ("elseif" ("→" | WS) expression statement*)* ("else" statement*)? "end"

while_statement := "while" ("→" | WS) expression statement* "end"

for_statement := "for" ("→" | WS) identifier "in" expression statement* "end"

repeat_statement := "repeat" ("→" | WS) expression "times" statement* "end"

match_statement := "match" ("→" | WS) expression ("case" ("→" | WS) pattern ("if" expression)? statement*)+ ("case" ("→" | WS) "_" statement*)? "end"

expression := 
  | literal
  | identifier
  | function_call
  | binary_op
  | unary_op
  | "(" expression ")"
  | array_literal
  | map_literal

literal := integer | float | string | boolean | null

identifier := [a-zA-Z_][a-zA-Z0-9_]*

type := "int" | "float" | "string" | "bool" | "array" | "map" | identifier
```

## Implementation Notes

- All source code is automatically obfuscated during compilation
- Bytecode is encrypted and protected
- Runtime includes anti-debugging mechanisms
- Memory is managed by garbage collector
- Type checking is optional but recommended

