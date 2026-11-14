# Roadmap

This roadmap is provisional. Phases, ordering, and scope are expected to
change based on implementation experience and research findings — this
project is not committed to executing this list unchanged.

```text
Phase 0  — Language design
Phase 1  — Lexer
Phase 2  — Parser
Phase 3  — AST
Phase 4  — Interpreter
Phase 5  — Type system
Phase 6  — Memory model
Phase 7  — IR
Phase 8  — Native compiler
Phase 9  — Core standard library
Phase 10 — Modules
Phase 11 — Concurrency
Phase 12 — Networking
Phase 13 — Dijk+
Phase 14 — Application
Phase 15 — Tooling
Phase 16 — Self-hosting
```

## Current phase

**Phase 5 — Type system** (`dijk-0.0.5`). Phase 0 (language design)
produced the charter in [philosophy.md](philosophy.md),
[language.md](language.md), and [architecture.md](architecture.md).
Phase 1 implemented lexical analysis per [lexer.md](lexer.md): `dijk lex
<file>` turns source text into a token stream. Phase 2 implemented
parsing per [parser.md](parser.md). Phase 3 formalized the parser's
output into Dijk's actual AST per [ast.md](ast.md), exposed by `dijk
parse`/`dijk ast <file>`, with a `Visitor`-based traversal
(`src/ast/visit.rs`) for later passes to build on.

Phase 4 (`dijk-0.0.4`) made Dijk executable: `dijk run <file>` runs a
program on a tree-walking interpreter per [interpreter.md](interpreter.md),
with runtime values, environments, call frames, return propagation,
recursion, and runtime errors. The list above has no separate phase for
it, but Phase 4 also introduced the first, deliberately small, **semantic
analysis** per [semantic.md](semantic.md): name resolution, lexical
scopes, and function resolution, so that no program with an unknown name
is ever run.

Phase 5 (`dijk-0.0.5`) adds the **static type system** per
[types.md](types.md): the types `i32`, `i64`, `bool`, `string` and
`unit`, typing rules for every construct, local type inference, function
signatures, return-path analysis, and stable diagnostic codes. `dijk
check <file>` runs every stage up to type checking without executing
anything, `dijk check --types` shows the type of every variable, and
`dijk run` executes only well-typed programs. Semantic analysis also
gained immutable-by-default bindings (`let mut`), and the interpreter
distinct `i32` and `i64` values. There is no memory model, IR, or
compilation yet. The memory model (Phase 6) is planned next.

Versions and phases do not line up one-to-one. Each `dijk-0.0.x`
release is a milestone that may span part of a phase, or (like `0.0.4`)
parts of two.

## Correspondence with `dijkstra-book`

Each phase above is expected to correspond to a chapter or chapter group in
`dijkstra-book`, so the curriculum tracks actual implementation milestones
rather than being written ahead of or independently from the code.
