# Contributing

Dijk Core is an educational project. Its purpose is to teach how a
programming language and compiler are built from first principles. Before
proposing a change, read [docs/philosophy.md](docs/philosophy.md) — it is
the standard every design decision is measured against.

## Guiding questions for any change

- Can this be traced down to the mechanism that implements it?
- Does this belong in Core (the machinery), or in Dijk+ (use of the
  machinery)? See [docs/language.md](docs/language.md) for the current
  feature boundary.
- Does this keep Core small enough to be understood in one sitting?

## Scope discipline

Dijk Core intentionally excludes convenience features, ecosystem tooling,
and application-level abstractions until the underlying mechanism they
would sit on top of already exists and is understood. If a change adds
capability without a corresponding mechanism a student can inspect, it
likely belongs in `dijk-stdlib` or `dijk+` instead.

## Process

This project does not yet have CI, a formal review process, or released
versions beyond `0.0.2`. Until then:

1. Open an issue or discussion describing the change and which roadmap
   phase (see [docs/roadmap.md](docs/roadmap.md)) it belongs to.
2. Keep pull requests scoped to a single concern.
3. Update the relevant `docs/` file when a change affects the language
   design or architecture, not just the code.
4. Before proposing a change, run:

   ```bash
   cargo build
   cargo test
   cargo clippy --all-targets --all-features -- -D warnings
   cargo fmt -- --check
   ```
