# Changelog

## [0.0.5] - 2026-11-14

Experimental / educational development release. Grammar, ABI, standard
library, compiler compatibility, and runtime behavior are all still
unstable.

The first release with a static type system: every program is
type-checked before it runs, and a program with a type error never runs.

### Added

- Type representation (`src/types/ty.rs`): `Type` with `i32`, `i64`,
  `bool`, `string` and `unit`, equality, display, and resolution of
  annotations. Only `i32`, `i64`, `bool` and `string` can be written.
  An internal `Type::Error` lets the checker report each mistake once
- Literal typing: integer literals take the integer type their context
  expects (annotation, parameter, return type, assigned variable, other
  operand), defaulting to `i32`, and are range-checked against it.
  `-2147483648` is one literal
- Operator typing rules as one table (`src/types/rules.rs`): `+ - * /`
  on same-type integers (`+` also on strings), `< <= > >=` on same-type
  integers, `== !=` on any same type but `unit`, unary `-` on integers,
  `!` on `bool`. No implicit conversions: `i32` and `i64` never mix
- The type environment Γ (`src/types/env.rs`), local type inference for
  `let`, checking of annotated `let`s, and shadowing consistent with
  semantic analysis
- `let mut`: `Statement::Let` gains `mutable` and `name_span`, and the
  AST printer shows `Mutable: true` for mutable bindings
- Immutable bindings, enforced by semantic analysis: assigning to a plain
  `let` or to a parameter is a semantic error
- Assignment type checking, function signatures (collected before any
  body, so forward references and recursion work), argument type
  checking at every call, return statement checking, `bool` conditions,
  and return-path analysis (a function with `-> T` must return on every
  path)
- `types::check(&Program) -> Result<TypedProgram, Vec<TypeError>>`. A
  `TypedProgram` records each function's signature and bindings (name,
  span, type, mutability, block depth) and each expression's type, keyed
  by span
- `dijk check <file>` (`.dk` and `.dijk`): a per-stage report (lexical
  analysis, parsing, semantic analysis, type checking) that stops at the
  first failing stage and never runs the program. Exit code 0 only for a
  well-typed program
- `dijk check --types <file>`: each function's signature and the
  inferred type of every variable, with its position, `mut` marker and
  block nesting. This is the typing environment made visible
- Source-aware type diagnostics with stable codes E0001–E0010, rendered
  as `type error[E000N]:` with the whole offending expression underlined
  and `expected`/`found` notes (`diagnostics::render_with_notes`)
- Runtime values `Value::I32` and `Value::I64`: the interpreter builds
  each integer literal at the type the checker gave it
- Examples: `examples/add.dk` (well-typed) and `examples/type_error.dk`
- `dijk --help` and `dijk doctor` describe the type checker and
  `dijk check`

### Changed / Breaking

- **`let mut` is now required for assignment.** `let x = 1; x = 2;` is a
  semantic error, and so is assigning to a parameter (parameters are
  always immutable). Programs that reassign variables must declare them
  `let mut`
- **`fn main()` without `-> T` returns `unit`**, like every function
  without a return type, so `return value;` in it is a type error
  (E0005). The 0.0.4 milestone program needs `fn main() -> i32`
- **Integers are now `i32` or `i64`**, statically and at run time.
  `Value::Integer(i64)` is replaced by `Value::I32`/`Value::I64`, a literal
  with no context is an `i32`, and `i32` arithmetic overflows at 32 bits
  (`2147483647 + 1` is a runtime error; `factorial(13)` overflows an
  `i32`). Overflow errors name the type: `does not fit in `i32``
- **Type errors stop programs before they run.** `1 + true`, `if 5 { }`,
  `"a" - "b"`, unknown type names such as `let x: banana = …` and
  out-of-range literals were runtime errors, or silently accepted, in
  0.0.4. The interpreter's `invalid operation` errors remain only as
  guards for unchecked programs, and runtime kind names are now type
  names (`i32`, `bool`, …)
- `interpreter::run` takes the `TypedProgram` as well as the `Program`
- `Expression::Integer` holds a `u64` (the literal's digits) instead of an
  `i64`. Digits up to 64 bits parse. Whether the value fits is now always
  the type checker's E0010, so `9223372036854775808` is a type error
  rather than a parse error. Only a literal beyond 64 bits is still a
  parse error (`integer literal '…' is too large`)
- Diagnostic positions count characters instead of bytes (see Fixed)
- Examples updated for the above: `hello.dk`, `hello.dijk`,
  `factorial.dk`, `runtime_error.dk` and `semantic_error.dk` now declare
  `fn main() -> i32`

### Fixed

- A non-ASCII character outside a string or comment is reported once, as
  the character it is, with its code point
  (`unexpected character 'é' (U+00E9)`), instead of once per UTF-8 byte
  as mojibake. Only ASCII characters count as whitespace
- Diagnostic columns, carets and underlines count characters, not bytes,
  so they line up after non-ASCII text. This applies to every stage and
  to `dijk check --types` positions
- Files with Windows line endings (`\r\n`) no longer show a stray `\r`
  at the end of the source line in diagnostics
- A tab before the problem no longer misaligns the caret: the caret line
  repeats the source line's tabs
- The smallest `i64` can be written as a literal: `-9223372036854775808`
- Line endings are normalized: a new `.gitattributes` (`* text=auto`)
  stores every text file with LF. The five files that were committed with
  CRLF (`docs/ast.md`, `docs/parser.md`, `src/ast/expression.rs`,
  `src/ast/visit.rs`, `src/parser/error.rs`) were converted

### Testing

- `tests/types_tests.rs` (133 tests): unit tests of every piece of the
  `types` module, added with each piece
- `tests/type_tests/` (104 tests): the language-level suite, with one
  file per typing rule (literals, operators, variables, assignments,
  functions, returns, conditions, errors), valid and invalid programs
  asserted by diagnostic code, and a pipeline test proving that a
  type-invalid program never reaches the interpreter
- `tests/interpreter_tests.rs`: runs through the type checker; `i32` and
  `i64` values, overflow in both, `i32` and `i64` factorials, and the
  interpreter's guards on unchecked programs
- `tests/semantic_tests.rs`: immutable bindings and parameters, including
  under shadowing
- `tests/parser_tests.rs`, `tests/ast_tests.rs`: `let mut`
- `tests/cli_tests.rs`: `dijk check` on `.dk` and `.dijk`, exit codes,
  stopping at the first failing stage, `dijk check --types`, and
  `dijk run` refusing to execute a program with a type error
- `tests/lexer_tests.rs`, `tests/parser_tests.rs`, `tests/cli_tests.rs`:
  non-ASCII characters, character columns, CRLF files, tabs, and
  64-bit literals
- 507 tests in total

### Documentation

- `docs/types.md`: the types, compatibility policy, typing judgments,
  operator table, inference policy, functions, return paths, diagnostic
  codes, `dijk check` and `--types`, implementation, and limitations
- `docs/lexer.md` (non-ASCII text and whitespace), `docs/language.md`,
  `docs/semantic.md` (mutability),
  `docs/interpreter.md` (`i32`/`i64`, overflow), `docs/architecture.md`
  (pipeline and dependency direction), `docs/testing.md`, `docs/ast.md`,
  `docs/parser.md` (`let mut`), `docs/roadmap.md`, and the README
- `dijkstra-book` Chapter 09 — Static Types

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
