# The Abstract Syntax Tree

This document describes Dijk's AST (`src/ast/`): the compiler's structured
representation of a program. The parser (`src/parser/parse.rs`) constructs
these nodes directly — there is no separate "syntax representation" stage
anymore (see `docs/parser.md`'s note on that history).

The AST answers one question:

> **What did the programmer write?**

It deliberately does not answer:

> **Is this program valid?**

```dijk
let x: i32 = "hello";
```

is syntactically a `Let` binding a string literal to a variable annotated
`i32`. The AST represents exactly that, without complaint — nothing has
checked that a string can be assigned where an `i32` was declared. That
check belongs to semantic analysis, a later milestone. Keeping this
boundary explicit is why the compiler's pipeline stays:

```text
Parser
    ↓
AST
    ↓
Semantic analysis
    ↓
Type checker
```

rather than folding validation into the AST itself.

## Parse tree vs. AST

A parser could, in principle, produce a *concrete parse tree*: a structure
containing every grammatical detail used to derive it, including
parentheses, which production rule matched, and so on. The AST is not
that. It keeps only **meaningful program structure**.

```dijk
a + b * c
```

parses through several grammar rules (`equality`, `comparison`, `term`,
`factor`, ...; see `docs/parser.md`), but the AST for it is just:

```text
Binary(+)
├── Identifier(a)
└── Binary(*)
    ├── Identifier(b)
    └── Identifier(c)
```

The grammar machinery used to *arrive* at that shape is not itself part of
the shape.

The clearest illustration of this is parentheses. `(a + b) * c` and an
expression where `a + b` was simply written already-nested under `*`
produce the **exact same tree**:

```text
Binary(*)
├── Binary(+)
│   ├── Identifier(a)
│   └── Identifier(b)
└── Identifier(c)
```

There is no `Grouping` node anywhere in `src/ast/expression.rs`. Once
precedence has been resolved, parentheses have done their job and leave no
trace — they are concrete syntax, not program structure. (Contrast this
with `dijk-0.0.2`'s provisional `syntax.rs`, which kept a `Grouping` node;
dropping it here is the concrete payoff of formalizing that provisional
representation into a real AST.)

```text
Concrete syntax
      ↓
Parse tree      (every grammar detail — Dijk Core never actually builds this)
      ↓
AST             (meaningful structure only — src/ast/)
      ↓
Useful compiler representation
```

## Node shapes

```text
Program
└── declarations: Vec<Declaration>

Declaration
└── Function(Function)

Function
├── name: String
├── params: Vec<Param>
├── return_type: Option<Type>
├── body: Vec<Statement>
└── span: Span

Param
├── name: String
├── ty: Type
└── span: Span

Type
├── name: String     (an identifier; nothing has checked it names a real type)
└── span: Span
```

There is no `Block` node. A body is just `Vec<Statement>` — a block was
never more than "a sequence of statements with a span," and nothing
downstream needs that span once parsing has finished (see
`Parser::parse_block` in `src/parser/parse.rs`, which computes and then
discards it).

```text
Statement
├── Let         { name, type_annotation: Option<Type>, value: Expression, span }
├── Return      { value: Option<Expression>, span }
├── Expression  { expr: Expression, span }
├── If          { condition, then_branch: Vec<Statement>,
│                 else_branch: Option<Vec<Statement>>, span }
└── While       { condition, body: Vec<Statement>, span }
```

`else if` is not a distinct case. `if a {} else if b {} else {}` parses
`else if` as an `else_branch` containing exactly one statement, itself an
`If`. There is nothing about "else if" that needs its own representation —
it is surface syntax over the general "else runs this block" shape, and
collapsing it is another instance of this milestone's central lesson.

```text
Expression
├── Integer(i64, Span)
├── String(String, Span)
├── Boolean(bool, Span)
├── Identifier(String, Span)
├── Binary   { left: Box<Expression>, operator: BinaryOperator,
│              right: Box<Expression>, span }
├── Unary    { operator: UnaryOperator, operand: Box<Expression>, span }
└── Assign   { name: String, value: Box<Expression>, span }
```

`Integer` holds a parsed `i64`, not the source lexeme — a lexeme like
`"99999999999999999999"` that doesn't fit in 64 bits is a parse error
(`ParseErrorKind::InvalidIntegerLiteral`), not something the AST needs to
represent. Converting digits to a value is still a syntactic act, distinct
from asking whether that value is *valid where it appears* (an `i32`
parameter given a value that doesn't fit — that question is semantic
analysis's, not yet asked here).

## Deviations from a "typical" example AST

An AST sketched from first principles (e.g. Step 3's own illustrative
Rust enums) often includes more than Dijk Core's grammar (`docs/parser.md`)
actually supports. Two things were deliberately left out, rather than
added speculatively:

- **No `Call` expression.** Dijk Core's grammar has no function-call
  syntax yet. Adding a `Call` node now would be a variant the parser can
  never construct — dead code advertising a feature that doesn't exist.
- **No `Grouping` expression.** Covered above — this one is dropped on
  purpose, not merely because the grammar lacks it.

Both may be reconsidered once the grammar actually grows to need them.

## Source spans

Every node that originated from source code carries a `span: Span`
(`crate::lexer::token::Span` — the same type the lexer and parser already
use, not a duplicate). This is what will let the type checker eventually
report:

```text
error: cannot add `i32` and `string`

 --> main.dk:4:13
  |
4 | let x = a + "hello";
  |             ^^^^^^^
```

pointing at the *specific* offending expression, not just "somewhere in
this function." `Statement::span()` and `Expression::span()` compute the
right span for whichever variant they're called on.

## Traversal

`src/ast/visit.rs` provides a `Visitor` trait: one method per node kind,
each with a default implementation that walks into that node's children
via a matching `walk_*` function. A consumer overrides only the node kinds
it cares about; overriding nothing just walks the whole tree.

```text
visit_program
    ↓
visit_declaration
    ↓
visit_function
    ↓
visit_statement
    ↓
visit_expression
```

This is deliberately not an elaborate visitor framework (no generic
return values, no fold/map variants) — just enough for semantic analysis,
type checking, and later passes to share one walking mechanism instead of
each writing its own recursive match. `dijk ast`'s node-count summary
(`crate::pretty::NodeCounts`) is the first real consumer.

## Invariants

These are design principles this AST is expected to uphold, not (yet)
things the Rust type system enforces on their own:

- Every `Binary` expression has exactly two operands; every `Unary`,
  exactly one. (The type system *does* enforce this one, via `Box<Expression>`
  fields rather than a `Vec`.)
- Every function declaration has a non-empty name (guaranteed by the
  grammar: `parse_function` requires an `IDENTIFIER` token).
- Every AST node that originated from source code has a valid `span`.
- The parser never constructs an AST node for syntactically invalid
  input — a construct that fails to parse produces a `ParseError`
  instead, never a partially-valid node standing in for it.

## What's next

The AST doesn't know what `x` refers to, whether types agree, or what a
program *does*. Those questions — name resolution, type checking, and
execution — are semantic analysis and the interpreter, not yet built.
`docs/roadmap.md` has the current plan.
