# Txt-code Grammar Specification

This document describes the formal grammar for the Txt-code programming language.

## Expression Grammar

**Note:** The arrow operator (`→`) is **NOT** part of expression grammar. It is only used in statements and lambda expressions.

```
expression       → or_expression
or_expression    → and_expression ("or" and_expression)*
and_expression   → null_coalesce ("and" null_coalesce)*
null_coalesce    → equality ("??" equality)*
equality         → comparison (("==" | "!=") comparison)*
comparison       → bitwise_or (("<" | ">" | "<=" | ">=") bitwise_or)*
bitwise_or       → bitwise_xor ("|" bitwise_xor)*
bitwise_xor      → bitwise_and ("^" bitwise_and)*
bitwise_and      → shift ("&" shift)*
shift            → additive (("<<" | ">>") additive)*
additive         → multiplicative (("+" | "-") multiplicative)*
multiplicative   → unary (("*" | "/" | "%") unary)*
unary            → ("not" | "-" | "~" | "++" | "--") unary | power
power            → call ("**" unary)*
call             → primary ("(" arguments? ")" | "[" expression "]" | "." identifier | "?." identifier | "?(" arguments? ")" | "?[" expression "]" | "?")*
primary          → literal
                 | identifier
                 | "(" expression ")"
                 | "[" (expression ("," expression)*)? "]"
                 | "{" (map_entry ("," map_entry)*)? "}"
                 | lambda

arguments        → expression ("," expression)*
map_entry        → (string | identifier) ":" expression
lambda           → "(" parameters? ")" "→" expression
parameters       → parameter ("," parameter)*
parameter        → ("...")? identifier (":" type)? ("=" expression)?
```

## Statement Grammar

```
program          → statement*

statement        → expression_statement
                 | assignment
                 | function_def
                 | return
                 | if_statement
                 | while_statement
                 | do_while_statement
                 | for_statement
                 | repeat_statement
                 | match_statement
                 | break
                 | continue
                 | try_statement
                 | import
                 | export
                 | const
                 | enum
                 | struct
                 | impl_block

expression_statement → expression

assignment       → "store" "→" identifier (":" type)? "→" expression

function_def     → "define" ("<" type_params ">")? "→" identifier "→" "(" parameters? ")" ("→" type)? statement* "end"

type_params      → identifier ("," identifier)*

return           → "return" "→" expression?

if_statement     → "if" "→" expression statement* ("elseif" "→" expression statement*)* ("else" statement*)? "end"

while_statement  → "while" "→" expression statement* "end"

do_while_statement → "do" statement* "while" "→" expression "end"

for_statement    → ("for" | "foreach") "→" identifier "in" expression statement* "end"

repeat_statement → "repeat" "→" expression "times" statement* "end"

match_statement  → ("match" | "switch") "→" expression ("case" ("→")? pattern ("if" "→" expression)? statement*)+ ("case" ("→")? "_" statement*)? "end"

try_statement    → "try" statement* ("catch" ("→")? identifier statement*)? ("finally" statement*)? "end"

import           → "import" "→" identifier ("," identifier)* ("from" "→" (identifier | string))? ("as" "→" identifier)?

export           → "export" "→" identifier ("," identifier)*

impl_block       → "impl" "→" identifier function_def* "end"
```

## Type Grammar

```
type             → "int"
                 | "float"
                 | "string"
                 | "bool"
                 | "array" "[" type "]"
                 | "map" "[" type "]"
                 | identifier
                 | generic

generic          → identifier
```

## Literal Grammar

```
literal          → integer
                 | float
                 | string
                 | boolean
                 | "null"

integer          → digit+ | "0x" hex_digit+ | "0b" binary_digit+
float            → digit+ "." digit+ (("e" | "E") ("+" | "-")? digit+)?
string           → '"' (escape | char)* '"' | "'" (escape | char)* "'"
boolean          → "true" | "false"

escape           → "\\" ("n" | "t" | "r" | "\\" | '"' | "'")
hex_digit        → [0-9a-fA-F]
binary_digit     → [0-1]
digit            → [0-9]
char             → any character except escape sequences and quote
```

## Pattern Grammar

```
pattern          → literal
                 | identifier
                 | "_"
                 | or_pattern
                 | range_pattern
                 | array_pattern
                 | map_pattern

or_pattern       → pattern ("|" pattern)+

range_pattern    → integer "..=" integer

array_pattern    → "[" (pattern ("," pattern)*)? "]"

map_pattern      → "{" (identifier (":" pattern)?)* "}"
```

**Or-pattern** — matches if the value matches any of the listed sub-patterns.
Example: `1 | 2 | 3`

**Range pattern** — matches if the value is within the inclusive integer range `[start, end]`.
Example: `1..=10`

## Operator Precedence (from highest to lowest)

**Note:** The arrow operator (`→`) is **NOT** an expression operator and is **NOT** included in this precedence list. It is a statement-level syntax element only.

1. `()` - Parentheses
2. `**` - Exponentiation (right-associative)
3. `*`, `/`, `%` - Multiplicative
4. `+`, `-` - Additive
5. `<<`, `>>` - Bitwise shifts
6. `&` - Bitwise AND
7. `^` - Bitwise XOR
8. `|` - Bitwise OR
9. `<`, `>`, `<=`, `>=` - Comparison
10. `==`, `!=` - Equality
11. `??` - Null coalesce
12. `and` - Logical AND
13. `or` - Logical OR
14. `++`, `--` - Increment/Decrement (prefix)
15. `?.`, `?()`, `?[]` - Optional chaining
16. `?` (postfix) - Error propagation (unwraps `Ok`, early-returns `Err`)

## Arrow Operator (`→`) Usage

**Important:** The arrow operator (`→`) is **NOT** an expression operator and **CANNOT** be used in expressions. It is a statement-level syntax element only.

The arrow operator (`→`) is used for:
- **Required** in statements: `store →`, `if →`, `while →`, `for →`, `repeat →`, `return →`, `import →`, `export →`, `define →`
- **Optional** in statements: `case →` (for readability), `catch →` (for readability)
- **In lambda expressions**: `(params) → expression` (this is lambda syntax, not an operator)
- **NOT used** as: a logical operator, arithmetic operator, or any expression operator

**Examples:**
- ✅ Correct: `store → x → 10` (statement)
- ✅ Correct: `if → x > 5` (statement)
- ✅ Correct: `(x) → x * 2` (lambda expression)
- ❌ Incorrect: `x → y` (cannot use arrow as expression operator)
- ❌ Incorrect: `a → b → c` (cannot chain arrows in expressions)

## Notes

- The arrow operator (`→`) is required in most statement contexts (see Arrow Operator section above)
- Whitespace and comments are ignored during parsing
- Single-line comments start with `#`
- Multi-line comments are enclosed in `## ... ##`
- Identifiers must start with a letter or underscore, followed by letters, digits, or underscores
- String literals support escape sequences: `\n`, `\t`, `\r`, `\\`, `\"`, `\'`
- Only `f"..."` strings support `{expression}` interpolation; plain `"..."` strings treat `{` literally
- Variadic function parameters use `...args` syntax
- Generic type parameters use `<T, U>` syntax
- Destructuring patterns are supported in assignments: `store → [a, b] → [1, 2]`
- `impl_block` attaches methods to a struct; the struct must be declared before the `impl` block
- Or-patterns (`1 | 2 | 3`) and range patterns (`1..=5`) are only valid inside `match` case arms
- Postfix `?` (error propagation) may only appear inside a function body; using it at the top level is a `RuntimeError`

