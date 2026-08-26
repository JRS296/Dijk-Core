# Parsing

This document defines Dijk Core's initial grammar and the parser that
implements it. It is written before extending either, and the
implementation should be judged against it — see `docs/lexer.md` for the
same convention applied to the lexer.

The parser answers one question:

> **How do these tokens relate to each other?**

The lexer already answered "what are the pieces" (`docs/lexer.md`). The
parser determines whether a token sequence conforms to Dijk's grammar and
constructs Dijk's AST (`docs/ast.md`) directly from it. It does not decide
what that structure *means* — that is semantic analysis's job, still to
come.

```text
Tokens
  ↓
Parser
  ↓
AST            (docs/ast.md)
  ↓
Semantic analysis
```

### A retired intermediate step

The very first version of this parser (`dijk-0.0.2`) built a separate,
deliberately provisional "syntax representation" (`src/parser/syntax.rs`)
rather than the real AST — the reasoning at the time was that parsing and
representing a program are related but distinct problems, and a student
should be able to answer "did this parse correctly?" before being asked to
design a representation meant to support everything semantic analysis,
IR generation, and beyond would eventually need. `dijk-0.0.3` formalized
that provisional representation into the real AST and the parser was
rewritten to build it directly; `syntax.rs` no longer exists. This history
is worth knowing mainly because it's a reasonable example of deliberately
building something provisional first — see `docs/ast.md` for what changed
in the formalization (most notably: parenthesized grouping, kept as its
own node in the provisional version, is gone entirely in the real AST).

## Grammar

EBNF-style, covering only what Dijk Core actually parses today. `?` means
optional, `*` means zero or more, `|` means alternatives.

```text
program     ::= function*

function    ::= "fn" identifier "(" parameters? ")" return_type? block

parameters  ::= parameter ("," parameter)*

parameter   ::= identifier ":" type

return_type ::= "->" type

type        ::= identifier

block       ::= "{" statement* "}"

statement   ::= let_statement
              | return_statement
              | if_statement
              | while_statement
              | expression_statement

let_statement ::= "let" identifier (":" type)? "=" expression ";"

return_statement ::= "return" expression? ";"

if_statement ::= "if" expression block ("else" (if_statement | block))?

while_statement ::= "while" expression block

expression_statement ::= expression ";"
```

### Expressions, by ascending precedence

```text
expression  ::= assignment

assignment  ::= identifier "=" assignment
              | equality

equality    ::= comparison (("==" | "!=") comparison)*

comparison  ::= term (("<" | ">" | "<=" | ">=") term)*

term        ::= factor (("+" | "-") factor)*

factor      ::= unary (("*" | "/") unary)*

unary       ::= ("-" | "!") unary
              | primary

primary     ::= INTEGER
              | STRING
              | "true"
              | "false"
              | identifier
              | "(" expression ")"
```

Everything not in this grammar (structs, enums, arrays, references, `for`,
modules, ...) is out of scope for this milestone; see `docs/language.md`
for where each of those is expected to land eventually.

## Precedence

Listed highest (binds tightest) to lowest:

```text
Highest
│
├── Unary          (-x, !x)
├── Multiplication (*, /)
├── Addition        (+, -)
├── Comparison      (<, >, <=, >=)
├── Equality        (==, !=)
└── Assignment      (=)
│
Lowest
```

`a + b * c` parses as `a + (b * c)`, not `(a + b) * c`, because
multiplication is parsed at a tighter precedence level than addition.
Parenthesized expressions (`"(" expression ")"`) let a program override
this — but produce no node of their own in the AST once precedence is
resolved; see `docs/ast.md`'s "parse tree vs. AST" section for why.

Assignment is right-associative (`x = y = 1` parses as `x = (y = 1)`) and
its left-hand side must currently be a plain identifier — anything else
(`1 = 2;`) is an `invalid assignment target` error.

## Parsing strategy

A hand-written recursive-descent parser, deliberately not a parser
generator or a Pratt parser. Each grammar rule above maps onto one parsing
function (`parse_function`, `parse_block`, `parse_statement`,
`parse_expression` and its precedence chain, ...), so the mapping from
grammar to code stays direct and inspectable — see `src/parser/parse.rs`.

Expression precedence is handled by recursive descent through one function
per precedence level (`parse_equality` → `parse_comparison` → `parse_term`
→ `parse_factor` → `parse_unary` → `parse_primary`), rather than a Pratt
parser. This is more verbose but keeps the connection between the
precedence table above and the code fully explicit, which matters more
here than compactness.

## Diagnostics

Like the lexer, the parser does not panic on invalid input. Parse errors
carry a span and render as the same kind of source-anchored diagnostic
(`crate::diagnostics::render`, shared with the lexer):

```text
error: expected parameter or ')', found '{'
 --> example.dk:1:10
  |
1 | fn main( {
  |          ^
```

## Error recovery

Recovery is basic, not a fully general error-recovery system:

- **Inside a block**, if a statement fails to parse, the parser records
  the error and skips tokens until the next `;` (consumed) or the block's
  closing `}` (left for the block to see), then keeps parsing subsequent
  statements. This means one broken statement does not prevent later
  statements in the same block from being checked.
- **At the top level**, if a function declaration fails to parse, the
  parser records the error and skips tokens until the next `fn` or the end
  of the file, then keeps parsing subsequent functions.

This is enough to demonstrate the underlying principle — **a compiler
should diagnose programs, not merely reject them** — without attempting
the more sophisticated recovery a production parser would eventually need.
