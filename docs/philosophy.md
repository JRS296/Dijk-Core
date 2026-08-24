# Philosophy

> **Every major abstraction in Dijk should be understandable in terms of the
> mechanisms that implement it.**

This is the single principle that governs every design decision in Dijk
Core. An abstraction is only allowed into the language once its underlying
mechanism can be taught, implemented, and inspected by the student using it.

## Traceability

Abstractions are not permitted to be "magic." Each one should decompose into
a chain the student can follow all the way down to hardware-level behavior.

### Functions

```text
Function
 ↓
Call
 ↓
Stack frame
 ↓
Calling convention
 ↓
Registers
 ↓
Machine instructions
```

### Arrays

```text
Array
 ↓
Contiguous memory
 ↓
Address calculation
 ↓
Load/store
```

### Threads

```text
Thread
 ↓
Execution context
 ↓
Stack
 ↓
Scheduler
 ↓
CPU
```

## Core vs. Plus

```text
Dijk Core
    = understand the machinery

Dijk+
    = use the machinery through abstractions
```

Dijk Core exists so a student can build the mechanism themselves. Dijk+
exists so that, once the mechanism is understood, it does not need to be
rebuilt from scratch to write real software. A student should always be able
to descend from a Dijk+ abstraction back into the Core mechanism beneath it.

## Consequences of this principle

- **Syntax is not the point.** Familiar, unsurprising syntax is preferred so
  that attention stays on the mechanism, not the notation.
- **Convenience is deferred, not rejected.** Features that make programming
  easier but obscure the mechanism (implicit conversions, hidden allocation,
  magic globals) are pushed to Dijk+ or excluded, not silently added to Core.
- **Small is a feature.** A smaller Core is easier to hold in one's head
  completely, which is a precondition for tracing every abstraction in it.
- **The implementation is part of the curriculum.** The compiler's source
  code, its intermediate stages, and its tooling output are all teaching
  material, not just infrastructure.

This document is the anchor for future design decisions. When a proposed
feature is unclear, it should be evaluated against this principle before
anything else.
