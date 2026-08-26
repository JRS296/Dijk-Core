# Changelog

## [0.0.2] - 2026-08-27

Experimental / educational development release. Grammar, ABI, standard
library, compiler compatibility, and runtime behavior are all still
unstable.

### Added

- Hand-written recursive-descent parser
- Provisional syntax representation (`src/parser/syntax.rs`) — deliberately
  not yet Dijk's formal AST; see `docs/parser.md`
- Function, `let`, `return`, `if`/`else`, `while`, and expression-statement
  parsing
- Expression parsing with operator precedence (unary, `*`/`/`, `+`/`-`,
  comparisons, equality, assignment) and parenthesized grouping
- Assignment expressions (identifier targets only)
- Parser error reporting with source-anchored diagnostics, sharing the
  lexer's rendering (`src/diagnostics.rs`)
- Basic error recovery: synchronization at statement (`;`/`}`) and
  declaration (`fn`) boundaries, so a single pass can report more than
  one problem
- `dijk parse` command, printing the syntax tree
- `TokenKind::describe()` for human-readable parser diagnostics

### Documentation

- Dijk grammar specification (`docs/parser.md`)
- The syntax-vs-AST boundary and why it's drawn where it is

## [0.0.1] - 2026-08-25

Experimental / educational development release. Grammar, ABI, standard
library, compiler compatibility, and runtime behavior are all still
unstable.

### Added

- Initial Dijk lexer
- `Token` and `TokenKind` representation
- Source spans (byte offsets)
- Keyword recognition
- Identifier recognition
- Integer literals
- String literals
- Operators and punctuation
- Line comments
- Lexical error reporting with source-anchored diagnostics
- `.dk` source file support
- `.dijk` source file support
- `dijk lex` command

### Documentation

- Dijk Core language charter (philosophy, design constraints, architecture, roadmap)
- Lexical grammar specification
- File extension convention (`.dk` / `.dijk`)
- Manual and automated testing guide

## [0.0.0] - 2026-08-25

### Added

- Project foundations: README, license, contributing guide
- Language philosophy and design constraints
- Intended compiler architecture and provisional roadmap
- Minimal CLI (`--version`, `--help`, `doctor`)
