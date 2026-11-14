# Changelog

## [0.0.4] - 2026-11-14

Experimental / educational development release. Grammar, ABI, standard
library, compiler compatibility, and runtime behavior are all still
unstable.

The first release in which Dijk programs run.

### Added

- Call expressions in the AST: `Expression::Call { callee, arguments,
  span }`, where the callee is a function name (`String`), not a general
  expression, because Dijk has no first-class functions. Spans, the
  `Visitor`, and the pretty-printer (`Call(name)`) cover it
- Function-call parsing: `call ::= identifier "(" arguments? ")"`, folded
  into `primary`
- Semantic analysis (`src/semantic/`): `semantic::analyze` resolves every
  name in a parsed program before it runs, as the AST `Visitor`'s second
  real consumer. It reports unknown identifiers, unknown functions,
  wrong argument counts, duplicate functions and parameters, assignment
  to an undeclared variable, a missing `main`, and `main` with parameters.
  Functions are collected first, so recursion and calls to later-defined
  functions resolve
- Lexical scopes (global → function → block) in semantic analysis:
  block-local variables end with their block, shadowing in an inner
  scope is allowed, and redeclaring a name in the same scope is an error.
  Types are not checked yet
- Runtime values (`src/interpreter/value.rs`): `Integer(i64)`,
  `Boolean`, `String`, `Unit`
- The environment (`src/interpreter/environment.rs`): a stack of scopes
  mapping names to values, with define, lookup, assign, and nested
  scopes
- A tree-walking interpreter (`src/interpreter/`): arithmetic,
  comparisons, booleans, strings (`+` concatenates), variables,
  assignment (which evaluates to the assigned value), `if`/`else if`/
  `else`, and `while`, with a block scope per body
- Function calls with call frames: arguments are evaluated left to right
  in the caller, bound to parameters in a fresh frame, and passed by
  value. Recursion is supported, up to a limit of 1000 calls in progress;
  the interpreter runs on a thread with a 256 MiB reserved stack so that
  the limit, not Rust's stack, is what stops runaway recursion
- Return propagation: `return` ends the enclosing call from any depth of
  nested blocks and loops, and code after it never runs
- Runtime errors, distinct from semantic errors: division by zero,
  checked integer overflow, invalid operations (such as `1 + true` or
  `if 5`), and call-depth overflow. Each is reported with the call stack
  at the moment of the error (`note: call stack: divide ← main`)
- `diagnostics::render` takes a label, so the four error classes read
  `error:` (lexical and parse, unchanged), `semantic error:`, and
  `runtime error:`
- `dijk run <file>` for `.dk` and `.dijk`: lex → parse → analyze →
  interpret, where each stage runs only if the previous ones succeeded. It
  prints `main`'s return value (nothing for `Unit`) and exits 0, or
  exits 1 on any error
- Examples: `examples/factorial.dk`, `examples/semantic_error.dk`,
  `examples/runtime_error.dk`
- `dijk doctor` and `dijk --help` describe the new stages and command

### Testing

- `tests/semantic_tests.rs`: every semantic rule, lexical scopes,
  shadowing, forward references, exact spans, and `semantic error:`
  rendering
- `tests/environment_tests.rs`: definition, lookup, assignment, nested
  scopes, and shadowing, independent of the interpreter
- `tests/interpreter_tests.rs`: end-to-end execution (arithmetic,
  booleans, strings, variables, assignment, `if`, `while`, functions,
  arguments, return values), runtime errors, recursion (factorial,
  Fibonacci, mutual recursion), and the exact call-depth limit
- `tests/parser_tests.rs`: function-call parsing
- `tests/cli_tests.rs`: `dijk run` on `.dk` and `.dijk`, recursion, a
  `Unit` `main`, and exit codes for semantic, runtime, parse, and lexical
  errors

### Documentation

- `docs/semantic.md`: what semantic analysis checks and deliberately
  does not, scoping rules, and implementation
- `docs/interpreter.md`: the execution model, covering values,
  environments, evaluation, control flow, call frames, the depth limit,
  return propagation, runtime errors, and `dijk run`
- `docs/architecture.md` brought up to date, with the interpreter branch,
  implemented vs. planned stages, and the source layout. It no longer
  claims that no subcommands exist
- `docs/testing.md`, `docs/language.md`, `docs/ast.md`, `docs/parser.md`,
  `docs/roadmap.md`, and the README updated for the new stages
- `dijkstra-book` Chapter 08 — Building the Interpreter

## [0.0.3] - 2026-08-27

Experimental / educational development release. Grammar, ABI, standard
library, compiler compatibility, and runtime behavior are all still
unstable.

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

### Testing

- `tests/ast_tests.rs`: AST construction, equality, span accessors, and
  traversal over hand-built trees, independent of the parser
- Additional `tests/parser_tests.rs` coverage: AST source-span
  correctness (function/parameter/expression spans, nested-span growth),
  integer-to-`i64` conversion and overflow, and traversal over
  parser-produced ASTs

### Documentation

- `docs/ast.md`: the AST shape, parse-tree-vs-AST, deviations from a
  "typical" example AST, invariants, and traversal
- `docs/parser.md` updated to describe the parser building the AST
  directly, with the retired provisional-syntax step noted as history
- `dijkstra-book` Chapter 07 — Abstract Syntax Trees

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
