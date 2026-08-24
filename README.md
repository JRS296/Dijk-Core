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

This repository is currently at **Milestone 0 — Language Charter**. No
lexer, parser, or compiler exists yet. See [docs/](docs/) for the philosophy,
design constraints, architecture, and roadmap that will govern implementation.

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

## Building and running

Requires a Rust toolchain (stable). From this directory:

```bash
cargo build
cargo run -- --version
cargo run -- --help
cargo run -- doctor
```

Or build once and use the binary directly:

```bash
cargo build --release
./target/release/dijk --version        # Linux/macOS
.\target\release\dijk.exe --version    # Windows PowerShell
```

## Status

```text
dijk --version
```

```text
Dijk 0.0.0
```

There is no `dijk compile` yet. There is nothing to compile yet.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
