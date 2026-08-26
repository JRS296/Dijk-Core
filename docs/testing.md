# Testing

Dijk Core has two layers of testing: an automated suite (`cargo test`) and
manual exercising of the CLI against real `.dk` files. This document
covers the manual workflow — how to run the lexer against the provided
examples, and how to construct your own cases.

## Automated tests

```bash
cargo test
```

This runs four suites:

- `tests/lexer_tests.rs` — every category in the lexical specification
  (`docs/lexer.md`): identifiers, integers, strings, keywords, operators,
  punctuation, whitespace, comments, lexical errors, and source spans.
- `tests/parser_tests.rs` — every category in the grammar (`docs/parser.md`):
  functions, variables, expression precedence, comparisons, control flow,
  assignment, parse errors, multi-error recovery, and AST source spans.
- `tests/ast_tests.rs` — the AST (`docs/ast.md`) independent of parsing:
  constructing nodes directly, operator `Display`, `span()` accessors,
  and traversal (`Visitor`) over hand-built trees.
- `tests/cli_tests.rs` — end-to-end checks against the built `dijk`
  binary itself (not just the underlying modules), including that `dijk
  lex`/`dijk parse` treat `.dk` and `.dijk` identically.

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
```

`hello.dk` and `hello.dijk` have identical contents — both `dijk lex` and
`dijk parse` accept both extensions (see `docs/language.md`). Anything
else (`.txt`, no extension, etc.) is rejected before the file is even read:

```text
$ dijk lex examples/hello.txt
dijk: 'examples/hello.txt' is not a Dijk source file (expected .dk or .dijk)
```

`hello.dk` should print a clean token stream ending in `EOF`, and a clean
syntax tree under `parse`. `unexpected_character.dk` and
`unterminated_string.dk` should print the tokens lexed before the
problem, then a source-anchored error, and exit non-zero. `parse_error.dk`
should print whatever of the syntax tree parsed successfully, then a
source-anchored parse error, and exit non-zero.

`cargo run -- lex <file>`, `cargo run -- parse <file>`, and `cargo run --
ast <file>` all work the same way and skip the explicit build step, at
the cost of a slower first run per change.

### 3. Try your own cases

Create a new `.dk` file — anywhere, `examples/` is just a convention, not a
requirement — and run `dijk lex` or `dijk parse` on it.

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

PowerShell one-liner for a quick scratch file, without leaving a file
tracked in the repo:

```powershell
'let x = 42 @ 10;' | Set-Content -Encoding utf8 scratch.dk
.\target\release\dijk.exe lex scratch.dk
```

`scratch.dk` (and any other stray `.dk` file at the repo root) is covered
by `.gitignore`, so ad hoc scratch files like this are never accidentally
committed. Fixtures meant to be kept belong in `examples/` instead.
