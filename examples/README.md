# Examples

Small `.dk` fixtures for manually exercising the lexer and parser with
`dijk lex` / `dijk parse`. See [../docs/testing.md](../docs/testing.md)
for how to run these and how to try your own cases.

- `hello.dk` — a valid program exercising most token categories and parses
  cleanly (functions, typed `let`, a binary expression, `return`).
- `hello.dijk` — identical to `hello.dk`, demonstrating that `dijk lex`
  and `dijk parse` both accept the `.dijk` extension too (see
  [../docs/language.md](../docs/language.md)).
- `unexpected_character.dk` — triggers the `unexpected character` lexical
  error and its diagnostic rendering.
- `unterminated_string.dk` — triggers the `unterminated string literal`
  lexical error.
- `parse_error.dk` — lexes cleanly but fails to parse (missing `)` in a
  parameter list), demonstrating a parser diagnostic.
