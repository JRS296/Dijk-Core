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

Both name the same language; `dijk lex` (and later `dijk parse`, `dijk
build`, ...) accept either. `.dk` is expected to be the convention used in
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

## Status

This is a living document that will grow as each construct above is
actually designed and implemented. It should never grow ahead of the
implementation it describes.
