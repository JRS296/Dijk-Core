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

**Phase 1 — Lexer.** Phase 0 (language design) produced the charter in
[philosophy.md](philosophy.md), [language.md](language.md), and
[architecture.md](architecture.md). Phase 1 has implemented lexical
analysis per [lexer.md](lexer.md): `dijk lex <file>` turns source text
into a token stream. There is no parser yet.

## Correspondence with `dijkstra-book`

Each phase above is expected to correspond to a chapter or chapter group in
`dijkstra-book`, so the curriculum tracks actual implementation milestones
rather than being written ahead of or independently from the code.
