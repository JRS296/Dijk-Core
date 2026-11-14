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
`dijk ast`, `dijk run`, and later `dijk build`, ...) accepts either. `.dk` is expected to be the convention used in
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
  grammar).
- Values are 64-bit integers, booleans, and strings without escapes.
  Integer arithmetic is checked: overflow is an error, never a silent
  wrap-around.
- Variables are declared with `let` and are lexically scoped: a block's
  variables end with the block, an inner block may shadow an outer
  variable, and one scope may not declare the same name twice
  (`docs/semantic.md`).
- Control flow is `if`/`else` and `while`, with boolean conditions.
  Functions take arguments by value and may recurse (`docs/interpreter.md`).
- Type annotations are parsed but not yet checked. A mismatch such as
  `1 + true` is found only when the program runs, as a runtime error.

## Status

This is a living document that will grow as each construct above is
actually designed and implemented. It should never grow ahead of the
implementation it describes.
