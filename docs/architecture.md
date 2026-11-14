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
               Semantic Analysis            implemented: names and scopes
                        │                   (docs/semantic.md)
           ┌────────────┴────────────┐
           ▼                         ▼
      Interpreter              Type Checker         planned
      implemented                    │
  (docs/interpreter.md)              ▼
                                  Dijk IR           planned
                                     │
                                     ▼
                              Code Generation       planned
                                     │
                                     ▼
                              Native Program        planned
```

The pipeline branches after semantic analysis. The interpreter runs the
checked AST directly; this is how `dijk run` executes programs today. It
was built first, so that *execution* can be understood before code
generation is introduced. The compilation branch will add a type checker
(making today's name-only semantic analysis a full static check), an
intermediate representation, and code generation. Eventually the
interpreter and the compiler will run the same checked programs.

Each stage only runs if the stages before it succeeded. A program with
lexical or parse errors is never analyzed, and a program with semantic
errors is never executed. Each stage reports its own class of error
through one shared renderer (`src/diagnostics.rs`): `error:` for lexical
and parse errors, `semantic error:`, and `runtime error:`.

## Source layout

```text
src/
├── main.rs            CLI: one function per command
├── diagnostics.rs     source-anchored error rendering, shared by all stages
├── lexer/             source text → tokens
├── parser/            tokens → AST
├── ast/               the AST itself, plus a Visitor for walking it
├── pretty.rs          AST → printed tree (dijk parse / dijk ast)
├── semantic/          AST → semantic errors (names, scopes, functions)
└── interpreter/       checked AST → runtime value (values, environment,
                       evaluation, call frames)
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
dijk lex        implemented   "What are the tokens?"
dijk parse      implemented   "Is this syntactically valid?"
dijk ast        implemented   "What does the program look like structurally?"
dijk run        implemented   "What does the program do?"
dijk typecheck  planned
dijk ir         planned
dijk asm        planned
dijk build      planned
```

Alongside these, `dijk --version`, `dijk --help` and `dijk doctor` report on
the toolchain itself.
