# Examples

Small `.dk` fixtures for manually exercising each stage of the compiler
with `dijk lex` / `dijk parse` / `dijk ast` / `dijk check` / `dijk run`.
See [../docs/testing.md](../docs/testing.md) for how to run these and how
to try your own cases.

- `hello.dk` — a valid program exercising several token categories
  (functions, typed `let`, a binary expression, `return`). It parses
  and type-checks cleanly, and `dijk run` prints `52`.
- `hello.dijk` — identical to `hello.dk`, demonstrating that every
  command accepts the `.dijk` extension too (see
  [../docs/language.md](../docs/language.md)).
- `add.dk` — a well-typed program with a function call: `dijk check`
  reports `Program is well-typed.`, and `dijk run` prints `30`.
- `factorial.dk` — recursion: `dijk run` prints `120`, with each call to
  `factorial` running in its own call frame.
- `semantic_error.dk` — parses cleanly, but uses a variable outside the
  block that declared it: `dijk run` reports `semantic error: unknown
  identifier` and runs nothing.
- `type_error.dk` — every name resolves, but a string initializes an
  `i32` variable: `dijk check` and `dijk run` both report a `type error`
  before anything runs.
- `runtime_error.dk` — valid in every static sense (`dijk check` accepts
  it), but divides by zero while running: `dijk run` reports
  `runtime error: division by zero` and the call stack at that moment.
- `unexpected_character.dk` — triggers the `unexpected character` lexical
  error and its diagnostic rendering.
- `unterminated_string.dk` — triggers the `unterminated string literal`
  lexical error.
- `parse_error.dk` — lexes cleanly but fails to parse (missing `)` in a
  parameter list), demonstrating a parser diagnostic.

Together, the last six show each class of error and the stage that
detects it: lexical, parse, semantic, type, and runtime.
