# Testing

Dijk Core has two layers of testing: an automated suite (`cargo test`) and
manual exercising of the CLI against real `.dk` files. This document
covers the manual workflow — how to run the lexer against the provided
examples, and how to construct your own cases.

## Automated tests

```bash
cargo test
```

This runs `tests/lexer_tests.rs`, which covers every category in the
lexical specification (`docs/lexer.md`): identifiers, integers, strings,
keywords, operators, punctuation, whitespace, comments, lexical errors,
and source spans. Run this first — it is the fastest way to confirm the
lexer still behaves correctly after a change.

## Manual testing

The automated tests confirm the lexer's behavior; running it by hand on
real files confirms the CLI and diagnostic output actually look right to a
human. Both are worth doing.

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
```

```powershell
# Windows PowerShell
.\target\release\dijk.exe lex examples\hello.dk
.\target\release\dijk.exe lex examples\hello.dijk
.\target\release\dijk.exe lex examples\unexpected_character.dk
.\target\release\dijk.exe lex examples\unterminated_string.dk
```

`hello.dk` and `hello.dijk` have identical contents — `dijk lex` accepts
both extensions (see `docs/language.md`). Anything else (`.txt`, no
extension, etc.) is rejected before the file is even read:

```text
$ dijk lex examples/hello.txt
dijk: 'examples/hello.txt' is not a Dijk source file (expected .dk or .dijk)
```

`hello.dk` should print a clean token stream ending in `EOF`. The other two
should print the tokens lexed before the problem, then a source-anchored
error, and exit with a non-zero status.

`cargo run -- lex <file>` works the same way and skips the explicit build
step, at the cost of a slower first run per change.

### 3. Try your own cases

Create a new `.dk` file — anywhere, `examples/` is just a convention, not a
requirement — and run `dijk lex` on it. Useful cases to try, matching the
categories in `docs/lexer.md`:

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

PowerShell one-liner for a quick scratch file, without leaving a file
tracked in the repo:

```powershell
'let x = 42 @ 10;' | Set-Content -Encoding utf8 scratch.dk
.\target\release\dijk.exe lex scratch.dk
```

`scratch.dk` (and any other stray `.dk` file at the repo root) is covered
by `.gitignore`, so ad hoc scratch files like this are never accidentally
committed. Fixtures meant to be kept belong in `examples/` instead.
