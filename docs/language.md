# Language

This document defines the initial design constraints for Dijk Core. It is
deliberately not a full language specification — that will be written
incrementally, alongside the implementation, as each construct is designed
and built.

## Intended characteristics

```text
Statically typed
Compiled
General purpose
Systems-oriented
Educational
Extensible
Cross-platform
```

## Syntax philosophy

Dijk Core syntax should be familiar rather than novel:

```dijk
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}
```

> Syntax is intentionally not considered a primary research contribution of
> this project. Time is better spent on the mechanisms syntax describes than
> on inventing punctuation.

## File extension

A Dijk source file uses either extension:

```text
.dk      short form, for everyday use
.dijk    long form, unambiguous
```

Both name the same language; every command (`dijk lex`, `dijk parse`,
`dijk ast`, `dijk check`, `dijk run`, and later `dijk build`, ...)
accepts either. `.dk` is expected to be the convention used in
practice; `.dijk` exists for contexts where `.dk` alone would be
ambiguous or where the fuller name is preferred.

## Initial Core feature boundary

Dijk Core initially targets:

```text
- primitive types
- variables
- expressions
- control flow
- functions
- arrays
- structs
- enums
- references/pointers
- basic memory operations
- modules
- basic I/O
```

## Explicitly out of scope for initial Core

```text
- HTTP
- databases
- web frameworks
- package ecosystem
- async/await
- advanced generics
- macros
- GUI
- IDE
```

These may eventually be addressed by Dijk+ or `dijk-stdlib`, built on top of
Core rather than folded into it.

## Type system, initial scope

Covers:

```text
- booleans
- integers
- floating-point values
- characters
- arrays
- structs
- enums
- references/pointers
- functions
```

Deferred to later, once the above is well understood:

```text
- generics
- traits/interfaces
- advanced inference
- algebraic data types
- ownership-related typing
```

Of the initial scope, `dijk-0.0.5` implements booleans and the integers
`i32` and `i64` (plus strings, and `unit` for functions that return
nothing), with strict compatibility: no implicit conversions between
types. See `docs/types.md`.

## Memory model, initial scope

Dijk Core prioritizes educational transparency over an immediately
sophisticated safety model:

```text
Explicit memory
    ↓
Understand memory bugs
    ↓
Understand why safety abstractions exist
    ↓
Introduce safer abstractions
```

An ownership/borrowing model may be investigated later, but it is not a
requirement for the initial language.

## Implemented so far

The feature boundary above is a target. What Dijk Core implements today
is much smaller:

- A program is one or more functions, and it starts running at
  `fn main()`, which takes no parameters (`docs/parser.md` has the
  grammar). `main` may return a value of any type, which `dijk run`
  prints.
- Every value has a **static type**: `i32`, `i64`, `bool` or `string`,
  plus `unit` for "no value". Types are checked before the program runs
  (`dijk check`, and as part of `dijk run`), and a program with a type
  error never runs. There are **no implicit conversions**: an `i32` is
  never accepted where an `i64` is expected, and there is no cast yet
  (`docs/types.md`).
- Integer literals are `i32` unless their context expects an `i64`.
  Integer arithmetic is checked: a result that does not fit in its type is
  a runtime error, never a silent wrap-around. Strings have no escapes.
- Variables are declared with `let`, and their type is inferred from the
  initializer or written as an annotation (`let x: i64 = 0;`). A variable
  is **immutable** unless declared `let mut`; only a `let mut` may be
  assigned to, and only with a value of its own type. Variables are
  lexically scoped: a block's variables end with the block, an inner
  block may shadow an outer variable, and one scope may not declare the
  same name twice (`docs/semantic.md`).
- Control flow is `if`/`else` and `while`, whose conditions must be
  `bool`.
- Functions declare the type of every parameter, and their return type
  with `-> T`; without it they return `unit`. A function with a return
  type must return a value on every path. Parameters are immutable, and
  arguments are passed by value. Functions may recurse
  (`docs/interpreter.md`).

## Status

This is a living document that will grow as each construct above is
actually designed and implemented. It should never grow ahead of the
implementation it describes.
