# Dijk Core

Dijk is an educational programming language designed to teach programming and
systems concepts from first principles, while remaining capable of scaling
into practical application development through Dijk+.

```text
Dijk
├── Dijk Core
│   └── foundational language and compiler
│
└── Dijk+
    └── application-development environment
```

**Dijk Core** is the foundational profile: a small, low-level language whose
implementation teaches how a language and its compiler are actually built.

This repository has completed **Step 1 — Lexical Analysis**: source text
(`.dk` or `.dijk` files) can be turned into a token stream via `dijk lex`.
There is no parser or compiler yet. See [docs/](docs/) for the philosophy,
design constraints, architecture, and roadmap that govern implementation.

## Project trajectory

```text
Language
  ↓
Lexer
  ↓
Parser
  ↓
Interpreter
  ↓
Type System
  ↓
IR
  ↓
Compiler
  ↓
Runtime
  ↓
Standard Library
  ↓
Dijk+
  ↓
Real Application
  ↓
Self-hosting
```

## Documentation

- [docs/philosophy.md](docs/philosophy.md) — the core educational principle
- [docs/language.md](docs/language.md) — initial design constraints and feature boundary
- [docs/architecture.md](docs/architecture.md) — intended compiler architecture
- [docs/roadmap.md](docs/roadmap.md) — provisional implementation phases
- [docs/lexer.md](docs/lexer.md) — lexical grammar implemented by `dijk lex`
- [docs/testing.md](docs/testing.md) — automated tests and manual CLI testing

## Building and running

Requires a Rust toolchain (stable). From this directory:

```bash
cargo build
cargo run -- --version
cargo run -- --help
cargo run -- doctor
cargo run -- lex examples/hello.dk
cargo test
```

See [docs/testing.md](docs/testing.md) for the full manual-testing
workflow, including sample fixtures for every token category and error
case.

Or build once and use the binary directly:

```bash
cargo build --release
./target/release/dijk --version        # Linux/macOS
.\target\release\dijk.exe --version    # Windows PowerShell
```

## Status

```text
dijk lex hello.dk
```

```text
FN
IDENTIFIER("main")
LPAREN
RPAREN
LBRACE
LET
IDENTIFIER("x")
COLON
IDENTIFIER("i32")
EQUAL
INTEGER("42")
SEMICOLON
RBRACE
EOF
```

There is no `dijk parse` or `dijk compile` yet.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
