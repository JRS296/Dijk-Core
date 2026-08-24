# Examples

Small `.dk` fixtures for manually exercising the lexer with `dijk lex`. See
[../docs/testing.md](../docs/testing.md) for how to run these and how to
try your own cases.

- `hello.dk` — a valid program exercising most token categories at once.
- `unexpected_character.dk` — triggers the `unexpected character` lexical
  error and its diagnostic rendering.
- `unterminated_string.dk` — triggers the `unterminated string literal`
  lexical error.
