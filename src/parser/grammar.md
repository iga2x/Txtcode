# Txt-code Grammar Specification

This document describes the formal grammar for the Txt-code programming language.

## Expression Grammar

```
expression       → or_expression
or_expression    → and_expression (("or" | "→") and_expression)*
and_expression   → equality (("and" | "→") equality)*
equality         → comparison (("==" | "!=") comparison)*
comparison       → bitwise_or (("<" | ">" | "<=" | ">=") bitwise_or)*
bitwise_or       → bitwise_xor ("|" bitwise_xor)*
bitwise_xor      → bitwise_and ("^" bitwise_and)*
bitwise_and      → shift ("&" shift)*
shift            → additive (("<<" | ">>") additive)*
additive         → multiplicative (("+" | "-") multiplicative)*
multiplicative   → unary (("*" | "/" | "%") unary)*
unary            → ("not" | "-" | "~") unary | power
power            → call ("**" unary)*
call             → primary ("(" arguments? ")" | "[" expression "]" | "." identifier)*
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
parameter        → identifier (":" type)?
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
                 | for_statement
                 | repeat_statement
                 | match_statement
                 | break
                 | continue
                 | try_statement
                 | import

expression_statement → expression

assignment       → "store" ("→")? identifier (":" type)? ("→")? expression

function_def     → "define" ("→")? identifier ("→")? "(" parameters? ")" ("→" type)? statement* "end"

return           → "return" ("→")? expression?

if_statement     → "if" ("→")? expression statement* ("elseif" ("→")? expression statement*)* ("else" statement*)? "end"

while_statement  → "while" ("→")? expression statement* "end"

for_statement    → "for" ("→")? identifier "in" expression statement* "end"

repeat_statement → "repeat" ("→")? expression "times" statement* "end"

match_statement  → "match" ("→")? expression ("case" ("→")? pattern ("if" expression)? statement*)+ ("case" ("→")? "_" statement*)? "end"

try_statement    → "try" statement* ("catch" ("→")? identifier statement*)? "end"

import           → "import" ("→")? identifier ("," identifier)* ("from" (identifier | string))? ("as" identifier)?
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
```

## Operator Precedence (from highest to lowest)

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
11. `and` - Logical AND
12. `or` - Logical OR
13. `→` - Arrow (for function calls and assignments)

## Notes

- The arrow operator (`→`) is optional in many contexts and can be used for clarity
- Whitespace and comments are ignored during parsing
- Single-line comments start with `#`
- Multi-line comments are enclosed in `## ... ##`
- Identifiers must start with a letter or underscore, followed by letters, digits, or underscores
- String literals support escape sequences: `\n`, `\t`, `\r`, `\\`, `\"`, `\'`

