# Dijk Core

**Dijk 0.0.4** — experimental / educational development release. Grammar,
ABI, standard library, compiler compatibility, and runtime behavior are
all still unstable. See [CHANGELOG.md](CHANGELOG.md).

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

## Current status

```text
✓ Project foundations (charter, philosophy, architecture, roadmap)
✓ Lexical analysis (dijk lex, .dk and .dijk source files)
✓ Parsing (dijk parse)
✓ AST (dijk ast)
✓ Semantic analysis: name resolution and lexical scopes
✓ Interpretation (dijk run): a tree-walking interpreter
□ Type checking
□ Intermediate representation
□ Native compilation
```

`0.0.4` is the first version in which Dijk programs *run*: `dijk run`
takes a program through every stage (lex → parse → semantic analysis →
interpret) and prints the value `main` returns. It is still
intentionally small. Semantic analysis resolves names and scopes but does
not check types (see [docs/semantic.md](docs/semantic.md)), so a mistake
like `1 + true` is found only at run time (see
[docs/interpreter.md](docs/interpreter.md)). There is no `print` yet, and
no compiler. See [docs/](docs/) for the philosophy, design constraints,
architecture, and roadmap that govern implementation.

## Project trajectory

```text
Language
  ↓
Lexer
  ↓
Parser
  ↓
Semantic Analysis
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
- [docs/architecture.md](docs/architecture.md) — compiler architecture: the pipeline, what exists, what's planned
- [docs/roadmap.md](docs/roadmap.md) — provisional implementation phases
- [docs/lexer.md](docs/lexer.md) — lexical grammar implemented by `dijk lex`
- [docs/parser.md](docs/parser.md) — grammar and parsing strategy implemented by `dijk parse`
- [docs/ast.md](docs/ast.md) — the AST shape, parse-tree-vs-AST, invariants, and traversal
- [docs/semantic.md](docs/semantic.md) — semantic analysis: name resolution, lexical scopes, and semantic errors
- [docs/interpreter.md](docs/interpreter.md) — the execution model: values, environments, call frames, return propagation, runtime errors, and `dijk run`
- [docs/testing.md](docs/testing.md) — automated tests and manual CLI testing
- [CHANGELOG.md](CHANGELOG.md) — what shipped in each release

## Building and running

Requires a Rust toolchain (stable). From this directory:

```bash
cargo build
cargo run -- --version
cargo run -- --help
cargo run -- doctor
cargo run -- lex examples/hello.dk
cargo run -- parse examples/hello.dk
cargo run -- ast examples/hello.dk
cargo run -- run examples/factorial.dk
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

## Example

For a file `hello.dk` containing `fn main() { let x: i32 = 42; }`:

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

```text
dijk parse examples/hello.dk
```

```text
Program
└── Function
    ├── Name: main
    ├── Parameters: []
    ├── Return type: i32
    └── Body
        ├── Let
        │   ├── Name: x
        │   ├── Type: i32
        │   └── Value
        │       └── Integer(42)
        ├── Let
        │   ├── Name: y
        │   ├── Type: i32
        │   └── Value
        │       └── Binary(+)
        │           ├── Identifier(x)
        │           └── Integer(10)
        └── Return
            └── Identifier(y)
```

`dijk ast` prints the same tree plus a node-count summary:

```text
1 function(s), 3 statement(s), 5 expression(s)
```

`dijk run` executes a program and prints the value its `main` returns:

```text
dijk run examples/factorial.dk
```

```dijk
fn factorial(n: i32) -> i32 {
    if n <= 1 {
        return 1;
    }

    return n * factorial(n - 1);
}

fn main() -> i32 {
    return factorial(5);
}
```

```text
120
```

A program that is well-formed but meaningless is rejected before it
runs, and a problem that only appears while running is reported with the
call stack at that moment:

```text
$ dijk run examples/semantic_error.dk
semantic error: unknown identifier `y`
 --> examples/semantic_error.dk:8:12
  |
8 |     return y;
  |            ^

$ dijk run examples/runtime_error.dk
runtime error: division by zero
 --> examples/runtime_error.dk:2:16
  |
2 |     return a / b;
  |                ^
note: call stack: divide ← main
```

There is no `dijk compile` yet.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
