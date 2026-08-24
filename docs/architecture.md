# Architecture

This document describes the intended compiler architecture. Nothing below
is implemented yet — it is the plan the implementation will be built
against, and is expected to be refined as implementation proceeds.

## Pipeline

```text
                    Dijk Source
                        │
                        ▼
                     Lexer
                        │
                        ▼
                     Parser
                        │
                        ▼
                       AST
                        │
                        ▼
               Semantic Analysis
                        │
                        ▼
                   Type Checker
                        │
                        ▼
                     Dijk IR
                        │
                        ▼
                  Code Generation
                        │
                        ▼
                  Native Program
```

An interpreter will be implemented before or alongside native compilation,
so that execution can be understood before code generation is introduced.

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
dijk lex
dijk parse
dijk typecheck
dijk ir
dijk asm
dijk run
dijk build
```

None of these subcommands exist yet. `dijk --version` is currently the only
implemented behavior.
