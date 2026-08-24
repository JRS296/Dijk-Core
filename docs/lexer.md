# Lexical Specification

This document defines the lexical grammar of Dijk Core: the rules by which
raw source text is divided into tokens. It is written before the lexer
implementation, and the implementation should be judged against it.

The lexer answers one question only:

> **What are the pieces of this source code?**

It does not answer how those pieces are structured (the parser's job) or
what they mean (semantic analysis's job). A token stream produced by the
lexer for:

```dijk
let x: i32 = 42;
```

contains `IDENTIFIER("x")` and `IDENTIFIER("i32")` as two syntactically
identical tokens. The lexer does not know that `x` is a variable, or that
`i32` is a type in this position — both are simply identifiers. That
distinction is introduced later, by semantic analysis.

## Token categories

### Identifiers

An identifier begins with an ASCII letter or underscore, followed by any
number of ASCII letters, digits, or underscores.

```text
foo
variable
my_function
User
_private
```

Unicode identifiers are explicitly out of scope for the initial lexer.

### Integer literals

A contiguous run of ASCII digits.

```text
0
42
12345
```

Hexadecimal, binary, octal forms, and numeric suffixes are out of scope for
the initial lexer.

### String literals

Characters between a pair of unescaped double quotes, on a single line.

```dijk
"Hello, world!"
```

Escape sequences are out of scope for the initial lexer. An unterminated
string (no closing `"` before the end of the line or file) is a lexical
error.

### Keywords

The following identifiers are reserved and produce a dedicated token kind
instead of `IDENTIFIER`:

```text
fn
let
mut
if
else
while
for
return
struct
enum
true
false
```

This list is expected to grow only as constructs using these keywords are
actually implemented. Primitive type names (`i32`, `bool`, ...) are
deliberately **not** keywords at this stage — see the note above.

### Operators

```text
+   Plus
-   Minus
*   Star
/   Slash
%   Percent
==  EqualEqual
!=  BangEqual
<   Less
>   Greater
<=  LessEqual
>=  GreaterEqual
=   Equal
!   Bang
&   Amp
|   Pipe
```

Multi-character operators (`==`, `!=`, `<=`, `>=`) are matched greedily
before falling back to their single-character prefix.

### Punctuation

```text
(   LeftParen
)   RightParen
{   LeftBrace
}   RightBrace
[   LeftBracket
]   RightBracket
,   Comma
:   Colon
;   Semicolon
.   Dot
->  Arrow
```

### Comments

```dijk
// a line comment, extending to the end of the line
```

Line comments are discarded and produce no token. Block comments are out
of scope for the initial lexer.

### Whitespace

Spaces, tabs, carriage returns, and newlines separate tokens and are
otherwise discarded. Dijk Core is not whitespace-sensitive.

### End of file

The lexer always terminates the token stream with a single `Eof` token, so
that a parser never needs to special-case running out of input.

## Explicitly deferred

The initial lexer does not support:

```text
- Unicode identifiers
- string escape sequences
- block comments
- hexadecimal / binary / octal integer literals
- floating-point literals
- macros or string interpolation
- indentation-sensitive syntax
```

These may be introduced later, once a reason to need them exists.

## Token representation

```text
Token
├── kind     — which category of token this is
├── lexeme   — the exact source text this token was built from
└── span     — the byte range [start, end) this token occupies
```

Byte offsets are sufficient for now. Line/column information for
diagnostics is derived from the offset and the source text on demand,
rather than stored on every token.

## Errors

The lexer does not panic on invalid input. An unrecognized character or an
unterminated string produces a lexical error carrying a span, which is
rendered as a source-anchored diagnostic, e.g.:

```text
error: unexpected character '@'
 --> example.dk:1:12
  |
1 | let x = 42 @ 10;
  |            ^
```

Lexing continues past an error where possible, so that a single pass can
report more than one problem.
