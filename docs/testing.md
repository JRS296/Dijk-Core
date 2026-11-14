# Testing

Dijk Core has two layers of testing: an automated suite (`cargo test`) and
manual exercising of the CLI against real `.dk` files. This document
describes both: what the automated suites cover, how to run each stage
against the provided examples, and how to construct your own cases.

## Automated tests

```bash
cargo test
```

This runs seven suites, one per stage plus the CLI, and the unit tests in
`src/main.rs` (source-file extension handling):

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
  lexical scopes and shadowing, and `semantic error:` rendering.
- `tests/environment_tests.rs` — the interpreter's environment
  (`docs/interpreter.md`) on its own, with no source code involved:
  define, lookup, assignment, nested scopes, and shadowing.
- `tests/interpreter_tests.rs` — execution end to end, through the same
  lex → parse → analyze → run pipeline as `dijk run`: arithmetic,
  booleans, strings, variables, assignment, `if`, `while`, functions and
  arguments, return values, runtime errors, recursion (factorial,
  Fibonacci, mutual recursion), and the call-depth limit.
- `tests/cli_tests.rs` — end-to-end checks against the built `dijk`
  binary itself (not just the underlying modules), including that `dijk
  lex`/`dijk parse`/`dijk run` treat `.dk` and `.dijk` identically, and
  that `dijk run` prints `main`'s value and exits 1 on each class of
  error.

The semantic, interpreter and execution tests each build their program
from source text, asserting that the *earlier* stages accept it. A
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
./target/release/dijk run examples/hello.dk
./target/release/dijk run examples/hello.dijk
./target/release/dijk run examples/factorial.dk
./target/release/dijk run examples/semantic_error.dk
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
.\target\release\dijk.exe run examples\hello.dk
.\target\release\dijk.exe run examples\hello.dijk
.\target\release\dijk.exe run examples\factorial.dk
.\target\release\dijk.exe run examples\semantic_error.dk
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

Under `run`, `hello.dk` and `hello.dijk` should both print `52`, and
`factorial.dk` should print `120`, each exiting 0. `semantic_error.dk`
should print a `semantic error:` and run nothing. `runtime_error.dk`
should print a `runtime error:` followed by the call stack
(`note: call stack: divide ← main`). Both exit 1, and neither prints
anything to stdout.

`cargo run -- lex <file>`, `cargo run -- parse <file>`, `cargo run -- ast
<file>`, and `cargo run -- run <file>` all work the same way and skip the
explicit build step, at the cost of a slower first run per change.

### 3. Try your own cases

Create a new `.dk` file — anywhere, `examples/` is just a convention, not a
requirement — and run `dijk lex`, `dijk parse` or `dijk run` on it.

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

Parser cases, matching the categories in `docs/parser.md`:

| Category | Example content |
|---|---|
| Function with parameters/return type | `fn add(a: i32, b: i32) -> i32 { return a + b; }` |
| Variables | `let x = 10;` and `let y: i32 = x + 20;` |
| Precedence | `a + b * c` vs. `(a + b) * c` — compare the two trees |
| Comparisons | `x == y`, `x != y`, `x < y`, `x >= y` |
| If / else if / else | `if x > 10 { return x; } else { return 0; }` |
| While + assignment | `while x > 0 { x = x - 1; }` |
| Missing `)` (error) | `fn main( {` |
| Missing `;` (error) | `let x = 1` with no semicolon |
| Invalid expression (error) | `let x = ;` |
| Multiple broken statements | `let x = ;` followed by a bare `return` — both should be reported |

Semantic analysis cases (`docs/semantic.md`). Each parses cleanly, and
`dijk run` should report a `semantic error:` and run nothing:

| Category | Example content |
|---|---|
| Unknown identifier | `fn main() { return x; }` |
| Variable used outside its block | `fn main() { if true { let y = 1; } return y; }` |
| Unknown function | `fn main() { return foo(); }` |
| Wrong number of arguments | `fn f(a: i32) {}` and `fn main() { f(); }` |
| Duplicate declaration in one scope | `fn main() { let x = 1; let x = 2; }` |
| Shadowing in an inner scope (valid) | `fn main() { let x = 1; if true { let x = 2; } return x; }` prints `1` |
| Assignment to an undeclared variable | `fn main() { x = 1; }` |
| Missing `main` | `fn helper() {}` |

Execution cases (`docs/interpreter.md`), each a `main` body:

| Category | Example content | `dijk run` prints |
|---|---|---|
| Arithmetic and precedence | `return 10 + 20 * 3;` | `70` |
| Strings | `return "hello, " + "world";` | `hello, world` |
| While + assignment | `let x = 0; while x < 5 { x = x + 1; } return x;` | `5` |
| No return value | `let x = 1;` | nothing |
| Division by zero (error) | `return 1 / 0;` | `runtime error: division by zero` |
| Invalid operation (error) | `return 1 + true;` | `runtime error: invalid operation: …` |
| Non-boolean condition (error) | `if 5 { }` | `runtime error: invalid operation: …` |
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
