# Testing

Dijk Core has two layers of testing: an automated suite (`cargo test`) and
manual exercising of the CLI against real `.dk` files. This document
describes both: what the automated suites cover, how to run each stage
against the provided examples, and how to construct your own cases.

## Automated tests

```bash
cargo test
```

This runs nine suites, one or two per stage plus the CLI, and the unit
tests in `src/main.rs` (source-file extension handling):

- `tests/lexer_tests.rs` — every category in the lexical specification
  (`docs/lexer.md`): identifiers, integers, strings, keywords, operators,
  punctuation, whitespace, comments, lexical errors, and source spans.
- `tests/parser_tests.rs` — every category in the grammar (`docs/parser.md`):
  functions, variables, expression precedence, comparisons, control flow,
  assignment, calls, parse errors, multi-error recovery, and AST source
  spans.
- `tests/ast_tests.rs` — the AST (`docs/ast.md`) independent of parsing:
  constructing nodes directly, operator `Display`, `span()` accessors,
  and traversal (`Visitor`) over hand-built trees.
- `tests/semantic_tests.rs` — semantic analysis (`docs/semantic.md`) on
  programs that parse cleanly: unknown identifiers and functions, arity,
  duplicate functions/parameters/variables, `main`, assignment targets,
  mutability (immutable `let`s and parameters, including under
  shadowing), lexical scopes and shadowing, and `semantic error:`
  rendering.
- `tests/types_tests.rs` — unit tests of the type checker's pieces
  (`docs/types.md`), one section per piece, each added with the piece
  itself: the `Type` representation and annotation resolution, literal
  typing and ranges, the operator rules table, the type environment,
  variables, assignments, signatures, calls, returns, conditions and
  return paths, what `TypedProgram` records, and diagnostic rendering
  (codes, notes, underlines).
- `tests/type_tests/` — the type system's language-level suite: one
  file per typing rule (`literals`, `operators`, `variables`,
  `assignments`, `functions`, `returns`, `conditions`, `errors`), each
  with valid and invalid Dijk programs asserted by their stable error
  codes, plus `pipeline`, which proves that a program with a type error
  never reaches the interpreter. It is a single test target
  (`tests/type_tests/main.rs` with submodules).
- `tests/environment_tests.rs` — the interpreter's environment
  (`docs/interpreter.md`) on its own, with no source code involved:
  define, lookup, assignment, nested scopes, and shadowing.
- `tests/interpreter_tests.rs` — execution end to end, through the same
  lex → parse → analyze → type-check → run pipeline as `dijk run`:
  arithmetic, `i32` and `i64` values and their overflow, booleans,
  strings, variables, assignment, `if`, `while`, functions and
  arguments, return values, runtime errors, recursion (factorial,
  Fibonacci, mutual recursion), and the call-depth limit. A section
  hands the interpreter unchecked programs on purpose, to test its own
  guards.
- `tests/cli_tests.rs` — end-to-end checks against the built `dijk`
  binary itself (not just the underlying modules), including that `dijk
  lex`/`dijk parse`/`dijk check`/`dijk run` treat `.dk` and `.dijk`
  identically, `dijk check`'s per-stage report and exit codes, the
  `dijk check --types` listing, that `dijk run` refuses to execute a
  program with a type error, and that it prints `main`'s value and exits
  1 on each class of error.

The semantic, type and interpreter tests each build their program from
source text, asserting that the *earlier* stages accept it. A
failure in an earlier stage means the test's fixture is broken, not the
stage under test.

Run this first — it is the fastest way to confirm the compiler still
behaves correctly after a change.

## Manual testing

The automated tests confirm the lexer's and parser's behavior; running
them by hand on real files confirms the CLI and diagnostic output actually
look right to a human. Both are worth doing.

### 1. Build the binary

```bash
cargo build --release
```

### 2. Run it against the provided examples

From `dijk-core/`:

```bash
# Linux/macOS
./target/release/dijk lex examples/hello.dk
./target/release/dijk lex examples/hello.dijk
./target/release/dijk lex examples/unexpected_character.dk
./target/release/dijk lex examples/unterminated_string.dk
./target/release/dijk parse examples/hello.dk
./target/release/dijk parse examples/parse_error.dk
./target/release/dijk ast examples/hello.dk
./target/release/dijk check examples/add.dk
./target/release/dijk check --types examples/add.dk
./target/release/dijk check examples/type_error.dk
./target/release/dijk run examples/hello.dk
./target/release/dijk run examples/hello.dijk
./target/release/dijk run examples/add.dk
./target/release/dijk run examples/factorial.dk
./target/release/dijk run examples/semantic_error.dk
./target/release/dijk run examples/type_error.dk
./target/release/dijk run examples/runtime_error.dk
```

```powershell
# Windows PowerShell
.\target\release\dijk.exe lex examples\hello.dk
.\target\release\dijk.exe lex examples\hello.dijk
.\target\release\dijk.exe lex examples\unexpected_character.dk
.\target\release\dijk.exe lex examples\unterminated_string.dk
.\target\release\dijk.exe parse examples\hello.dk
.\target\release\dijk.exe parse examples\parse_error.dk
.\target\release\dijk.exe ast examples\hello.dk
.\target\release\dijk.exe check examples\add.dk
.\target\release\dijk.exe check --types examples\add.dk
.\target\release\dijk.exe check examples\type_error.dk
.\target\release\dijk.exe run examples\hello.dk
.\target\release\dijk.exe run examples\hello.dijk
.\target\release\dijk.exe run examples\add.dk
.\target\release\dijk.exe run examples\factorial.dk
.\target\release\dijk.exe run examples\semantic_error.dk
.\target\release\dijk.exe run examples\type_error.dk
.\target\release\dijk.exe run examples\runtime_error.dk
```

`hello.dk` and `hello.dijk` have identical contents — every command
accepts both extensions (see `docs/language.md`). Anything else (`.txt`,
no extension, etc.) is rejected before the file is even read:

```text
$ dijk lex examples/hello.txt
dijk: 'examples/hello.txt' is not a Dijk source file (expected .dk or .dijk)
```

`hello.dk` should print a clean token stream ending in `EOF`, and a clean
AST under `parse`. `unexpected_character.dk` and
`unterminated_string.dk` should print the tokens lexed before the
problem, then a source-anchored error, and exit non-zero. `parse_error.dk`
should print whatever of the AST parsed successfully, then a
source-anchored parse error, and exit non-zero.

Under `check`, `add.dk` should print one `OK` line per stage and
`Program is well-typed.`, exiting 0; with `--types` it then lists
`add`'s signature and the types of `a`, `b` and `result`. `type_error.dk`
should stop at `Type checking          FAILED (1 error)` and print a
`type error[E0001]: type mismatch` with `expected `i32`` / `found
`string`` notes, exiting 1.

Under `run`, `hello.dk` and `hello.dijk` should both print `52`,
`add.dk` `30`, and `factorial.dk` `120`, each exiting 0.
`semantic_error.dk` should print a `semantic error:` and run nothing, and
`type_error.dk` a `type error[E0001]:` and run nothing.
`runtime_error.dk` should print a `runtime error:` followed by the call
stack (`note: call stack: divide ← main`). All three exit 1, and none
prints anything to stdout.

`cargo run -- lex <file>`, `cargo run -- parse <file>`, `cargo run -- ast
<file>`, `cargo run -- check <file>`, and `cargo run -- run <file>` all
work the same way and skip the explicit build step, at the cost of a
slower first run per change.

### 3. Try your own cases

Create a new `.dk` file — anywhere, `examples/` is just a convention, not a
requirement — and run `dijk lex`, `dijk parse`, `dijk check` or
`dijk run` on it.

Lexer cases, matching the categories in `docs/lexer.md`:

| Category | Example content |
|---|---|
| Identifiers | `foo my_function User _private` |
| Integers | `0 42 12345` |
| String | `"Hello, world!"` |
| Keywords | `fn let mut if else while for return struct enum true false` |
| Primitive type name (not a keyword) | `i32` → prints `IDENTIFIER("i32")` |
| Single-character operators | `+ - * / % = ! < > & \|` |
| Multi-character operators | `== != <= >= ->` |
| Punctuation | `( ) { } [ ] , : ; .` |
| Whitespace | mixed spaces/tabs/newlines between tokens |
| Line comment | `let x = 1; // comment` |
| Unexpected character (error) | `let x = 42 @ 10;` |
| Unterminated string (error) | `"never closed` |
| Non-ASCII text in a string or comment (valid) | `let s = "café"; // ☕` |
| Non-ASCII character elsewhere (error, reported once) | `let café = 1;` → `unexpected character 'é' (U+00E9)` |
| Windows line endings | save any file with CRLF: diagnostics show no `\r` |

Parser cases, matching the categories in `docs/parser.md`:

| Category | Example content |
|---|---|
| Function with parameters/return type | `fn add(a: i32, b: i32) -> i32 { return a + b; }` |
| Variables | `let x = 10;` and `let y: i32 = x + 20;` |
| Precedence | `a + b * c` vs. `(a + b) * c` — compare the two trees |
| Comparisons | `x == y`, `x != y`, `x < y`, `x >= y` |
| If / else if / else | `if x > 10 { return x; } else { return 0; }` |
| While + assignment | `while x > 0 { x = x - 1; }` |
| Mutable binding | `let mut count = 0;` — the tree shows `Mutable: true` |
| Missing `)` (error) | `fn main( {` |
| Missing `;` (error) | `let x = 1` with no semicolon |
| Invalid expression (error) | `let x = ;` |
| Multiple broken statements | `let x = ;` followed by a bare `return` — both should be reported |

Semantic analysis cases (`docs/semantic.md`). Each parses cleanly, and
`dijk run` should report a `semantic error:` and run nothing:

| Category | Example content |
|---|---|
| Unknown identifier | `fn main() -> i32 { return x; }` |
| Variable used outside its block | `fn main() { if true { let y = 1; } return y; }` |
| Unknown function | `fn main() -> i32 { return foo(); }` |
| Wrong number of arguments | `fn f(a: i32) {}` and `fn main() { f(); }` |
| Duplicate declaration in one scope | `fn main() { let x = 1; let x = 2; }` |
| Shadowing in an inner scope (valid) | `fn main() -> i32 { let x = 1; if true { let x = 2; } return x; }` prints `1` |
| Assignment to an undeclared variable | `fn main() { x = 1; }` |
| Assignment to an immutable variable | `fn main() { let x = 1; x = 2; }` |
| Assignment to a parameter | `fn f(n: i32) { n = 0; }` and `fn main() { f(1); }` |
| Missing `main` | `fn helper() {}` |

Type checking cases (`docs/types.md`). Each passes semantic analysis,
and `dijk check` (and `dijk run`) should report a `type error[…]:` and
run nothing:

| Category | Example content | Code |
|---|---|---|
| Mismatched initializer | `fn main() { let x: i32 = "hello"; }` | E0001 |
| Invalid operands | `fn main() { let x = 10 + "hello"; }` | E0002 |
| `i32` mixed with `i64` | `fn main() { let a: i64 = 1; let b = 2; let c = a + b; }` | E0002 |
| Invalid unary operand | `fn main() { let x = -true; }` | E0003 |
| Wrong argument type | `fn f(n: i32) {}` and `fn main() { f("one"); }` | E0004 |
| Wrong return type | `fn main() -> i32 { return "hello"; }` | E0005 |
| Value returned from a unit function | `fn main() { return 1; }` | E0005 |
| Non-boolean condition | `fn main() { if 5 { } }` | E0006 |
| Wrong assigned value | `fn main() { let mut x = 1; x = true; }` | E0007 |
| Unknown type name | `fn main() { let x: banana = 1; }` | E0008 |
| Missing return on some path | `fn main() -> i32 { if true { return 1; } }` | E0009 |
| Literal out of range | `fn main() { let x = 3000000000; }` | E0010 |
| Smallest `i64` literal (valid) | `fn main() -> i64 { return -9223372036854775808; }` prints `-9223372036854775808` | — |
| A well-typed `--types` listing | `fn main() -> i64 { let big: i64 = 2; let sum = big + 3; return sum; }` | `sum : i64` |

Execution cases (`docs/interpreter.md`), each the body of a
`fn main() -> i32` unless noted:

| Category | Example content | `dijk run` prints |
|---|---|---|
| Arithmetic and precedence | `return 10 + 20 * 3;` | `70` |
| Strings (`fn main() -> string`) | `return "hello, " + "world";` | `hello, world` |
| While + assignment | `let mut x = 0; while x < 5 { x = x + 1; } return x;` | `5` |
| No return value (`fn main()`) | `let x = 1;` | nothing |
| `i64` arithmetic (`fn main() -> i64`) | `return 2147483647 + 1;` | `2147483648` |
| Division by zero (error) | `return 1 / 0;` | `runtime error: division by zero` |
| `i32` overflow (error) | `return 2147483647 + 1;` | `runtime error: integer overflow: … `i32`` |
| Unbounded recursion (error) | `fn f() -> i32 { return f(); }`, called from `main` | `runtime error: stack overflow: …` |

PowerShell one-liner for a quick scratch file, without leaving a file
tracked in the repo:

```powershell
'let x = 42 @ 10;' | Set-Content -Encoding utf8 scratch.dk
.\target\release\dijk.exe lex scratch.dk
```

`scratch.dk` (and any other stray `.dk` file at the repo root) is covered
by `.gitignore`, so ad hoc scratch files like this are never accidentally
committed. Fixtures meant to be kept belong in `examples/` instead.
