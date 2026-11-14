# Architecture

This document describes the compiler architecture: the stages a Dijk
program passes through, which of them exist today, and the plan for the
rest. The plan is expected to be refined as implementation proceeds.

## Pipeline

```text
                    Dijk Source
                        │
                        ▼
                     Lexer                  implemented  (docs/lexer.md)
                        │
                        ▼
                     Parser                 implemented  (docs/parser.md)
                        │
                        ▼
                       AST                  implemented  (docs/ast.md)
                        │
                        ▼
               Semantic Analysis            implemented: names, scopes,
                        │                   mutability (docs/semantic.md)
                        ▼
                  Type Checker              implemented: static types
                        │                   (docs/types.md)
           ┌────────────┴────────────┐
           ▼                         ▼
      Interpreter                 Dijk IR            planned
      implemented                    │
  (docs/interpreter.md)              ▼
                              Code Generation        planned
                                     │
                                     ▼
                              Native Program         planned
```

Every program goes through the same front end: lexing, parsing,
semantic analysis and type checking. Only then does the pipeline
branch. The interpreter runs the checked AST directly; this is how
`dijk run` executes programs today. It was built first, so that
*execution* can be understood before code generation is introduced. The
compilation branch will add an intermediate representation and code
generation, working from the same checked program. Eventually the
interpreter and the compiler will run the same checked programs.

Each stage only runs if the stages before it succeeded. A program with
lexical or parse errors is never analyzed, a program with semantic
errors is never type-checked, and a program with type errors is never
executed. Each stage reports its own class of error through one shared
renderer (`src/diagnostics.rs`): `error:` for lexical and parse errors,
`semantic error:`, `type error[E…]:` (with a stable code per rule), and
`runtime error:`.

## Dependency direction

The type checker is a distinct pass, not part of the interpreter.
Both read the AST, and neither changes it:

```text
              AST
               │
       ┌───────┴───────┐
       ▼               ▼
 Type checker     Interpreter
  src/types/      src/interpreter/
       │               ▲
       └───────────────┘
         TypedProgram
```

- `types` depends only on the AST (and the lexer's `Span`). It never
  depends on the interpreter, so it can be reused unchanged by the
  compiler branch later.
- The type checker's *results* are kept in a `TypedProgram` (function
  signatures, the type of every variable, and the type of every
  expression, keyed by span) rather than thrown away. The interpreter
  consumes it in exactly one place: to build an integer literal as an
  `i32` or an `i64` value. An IR generator will need the same
  information.
- The interpreter does not decide whether a program is type-correct.
  Its own checks on the kinds of values only guard against programs that
  skipped the checker.

## Source layout

```text
src/
├── main.rs            CLI: one function per command
├── diagnostics.rs     source-anchored error rendering, shared by all stages
├── lexer/             source text → tokens
├── parser/            tokens → AST
├── ast/               the AST itself, plus a Visitor for walking it
├── pretty.rs          AST → printed tree (dijk parse / dijk ast), and the
│                      typing environment (dijk check --types)
├── semantic/          AST → semantic errors (names, scopes, functions,
│                      mutability)
├── types/             AST → TypedProgram or type errors (types, rules,
│                      type environment, checker)
└── interpreter/       checked AST + TypedProgram → runtime value (values,
                       environment, evaluation, call frames)
```

## Implementation language

The first implementation will be written in Rust. Rust's algebraic data
types and pattern matching are well suited to representing an AST and IR,
and its safety guarantees are valuable for the compiler's own
implementation. Rust is the implementation language, not the language being
taught by this project.

## Backend

The initial compiler will likely use LLVM as a backend once the language
reaches native compilation. This is not committed to as a permanent
constraint — it is the practical starting point.

## Observability

The compiler's intermediate stages should be exposed through tooling where
practical, so the compiler itself is part of the curriculum rather than a
black box:

```text
dijk lex             implemented   "What are the tokens?"
dijk parse           implemented   "Is this syntactically valid?"
dijk ast             implemented   "What does the program look like structurally?"
dijk check           implemented   "Is this program well-typed?" (without running it)
dijk check --types   implemented   "What type did each variable get?"
dijk run             implemented   "What does the program do?"
dijk ir              planned
dijk asm             planned
dijk build           planned
```

Alongside these, `dijk --version`, `dijk --help` and `dijk doctor` report on
the toolchain itself.
