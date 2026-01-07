# Txt-code Syntax Reference

Quick reference guide for Txt-code syntax.

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

### Functions
```txtcode
define → greet → (name)
  return → "Hello, " + name
end

define → add → (a: int, b: int) → int
  return → a + b
end
```

### Control Flow
```txtcode
# If statement
if → condition
  print → "True"
else
  print → "False"
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
```

### Pattern Matching
```txtcode
match → value
  case → 0
    print → "Zero"
  case → n if n > 0
    print → "Positive"
  case → _
    print → "Negative"
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

### Comparison
- `==` Equal
- `!=` Not equal
- `<` Less than
- `>` Greater than
- `<=` Less than or equal
- `>=` Greater than or equal

### Logical
- `and` Logical AND
- `or` Logical OR
- `not` Logical NOT

### Bitwise
- `&` Bitwise AND
- `|` Bitwise OR
- `^` Bitwise XOR
- `<<` Left shift
- `>>` Right shift
- `~` Bitwise NOT

## Data Types

- `int` - 64-bit integers
- `float` - 64-bit floating-point
- `string` - UTF-8 strings
- `bool` - Boolean values
- `array` - Arrays
- `map` - Key-value maps
- `null` - Null value

## Examples

See the `examples/` directory for complete example programs.

