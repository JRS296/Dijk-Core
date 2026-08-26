# Changelog

## [Unreleased]

Not yet tagged as a release. In progress toward `dijk-0.0.3` — the AST.

### Added

- Dijk's AST (`src/ast/`): `Program`, `Declaration`/`Function`/`Param`,
  `Type`, `Statement`, `Expression`, `BinaryOperator`, `UnaryOperator`
- The parser (`src/parser/parse.rs`) now constructs AST nodes directly;
  the provisional `src/parser/syntax.rs` from `0.0.2` is retired
- `else if` desugars into a single-statement `else_branch`, rather than a
  distinct AST case
- No `Grouping` node — parenthesized expressions produce no node of their
  own, once precedence is resolved (see `docs/ast.md`)
- Integer literals are parsed to `i64` at AST-construction time, with a
  new `ParseErrorKind::InvalidIntegerLiteral` for values that overflow
- Source spans on every AST node (reusing the lexer's `Span`, not a new
  type), with `Statement::span()` / `Expression::span()` accessors
- A minimal `Visitor` trait and `walk_*` functions for AST traversal
  (`src/ast/visit.rs`), with `dijk ast`'s node-count summary as its first
  real consumer
- `dijk ast` command, printing the AST plus function/statement/expression
  counts

### Documentation

- `docs/ast.md`: the AST shape, parse-tree-vs-AST, deviations from a
  "typical" example AST, invariants, and traversal
- `docs/parser.md` updated to describe the parser building the AST
  directly, with the retired provisional-syntax step noted as history

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
