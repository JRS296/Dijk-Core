# Semantic Analysis

This document describes Dijk Core's semantic analysis (`src/semantic/`):
the stage between the AST and execution. It covers what the stage checks,
what it deliberately does not check, and how it is implemented.

Semantic analysis answers one question:

> **Is this program allowed to mean something?**

The parser has already answered "is this structurally valid?"
(`docs/parser.md`), and the AST records *what was written*
(`docs/ast.md`). Neither notices that this program is meaningless:

```dijk
fn main() {
    return unknown_variable;
}
```

It parses into a perfectly good tree (`Return` holding
`Identifier(unknown_variable)`), but the name refers to nothing. Semantic
analysis is the stage that asks, for every name in the program, *what
does this refer to, and is that allowed here?* It rejects meaningless
programs before anything runs:

```text
semantic error: unknown identifier `unknown_variable`
 --> example.dk:2:12
  |
2 |     return unknown_variable;
  |            ^
```

```text
Parser             "Is this structurally valid?"
  ↓
AST                "What was written?"
  ↓
Semantic analysis  "Is this allowed to mean something?"
  ↓
Type checking      "Are its operations valid for its values?"  (docs/types.md)
  ↓
Interpreter        "What does it do?"        (docs/interpreter.md)
```

## Deliberately small

Semantic analysis is **name resolution**, plus one rule about what names
allow: only a binding declared `let mut` may be assigned to. It resolves
variables, functions and scopes. It is **not** a type checker. This
passes semantic analysis, because every name in it refers to something:

```dijk
fn main() -> i32 {
    let x: banana = "hi";
    let y: i32 = x + true;
    return y;
}
```

Type annotations and operand types are the *next* stage's question. The
type checker (`docs/types.md`) runs only after semantic analysis has
accepted a program, and rejects this one (an unknown type `banana`),
still before anything runs. Until `dijk-0.0.5` there was no such stage,
and a mistake like `x + true` was only found by the interpreter, at run
time.

## What it checks

| Rule | Example | Diagnostic |
|---|---|---|
| A variable must be declared before it is used | `return x;` with no `x` | `unknown identifier `x`` |
| Assignment changes an existing variable; it never declares one | `y = 3;` with no `let y` | `cannot assign to undeclared variable `y`` |
| A called function must exist | `missing();` | `unknown function `missing`` |
| A call passes exactly as many arguments as there are parameters | `add(1)` for `add(a, b)` | `wrong number of arguments to `add`: expected 2, found 1` |
| Function names are unique | two `fn twice()` | `function `twice` is defined more than once` |
| Parameter names are unique within a function | `fn f(a: i32, a: i32)` | `parameter `a` is declared more than once` |
| A name is declared at most once per scope | `let x = 1; let x = 2;` | `variable `x` is already declared in this scope` |
| The program has a `main` | only `fn helper() {}` | `no `main` function: a Dijk program starts running at `fn main()`` |
| `main` takes no parameters | `fn main(argc: i32)` | `` `main` must not take parameters `` |
| Only a `let mut` may be assigned to | `let x = 1; x = 2;` | `cannot assign to immutable variable `x` (declare it with `let mut x` to allow assignment)` |
| A parameter is never assigned to | `fn f(n: i32) { n = 0; }` | `cannot assign to parameter `n`: parameters are immutable` |

`main` takes no parameters because nothing in a Dijk program calls it.
`dijk run` does, with no arguments.

## What it does not check

- **Types.** Whether annotations name real types, whether operands,
  arguments, assigned values and returned values have the right types,
  and whether a function returns on every path are all the type
  checker's rules (`docs/types.md`).
- **Reachability.** Code after a `return` is legal. It simply never runs.
  There is no "unreachable code" warning.
- **Unused variables, parameters or functions.**

## Mutability

A variable is **immutable** unless it is declared `let mut`:

```dijk
let mut count = 0;    // may be assigned
let limit = 10;       // may not
count = count + 1;    // fine
limit = 11;           // semantic error: cannot assign to immutable variable `limit`
```

Parameters are immutable too, always: the grammar has no way to declare
a parameter `mut` (`docs/parser.md`). To change an argument's value,
copy it into a `let mut` first.

This is a rule about **bindings**, not values: whether `x = …;` is
allowed depends only on how the name `x` was declared. It is checked
against whichever binding the name resolves to at that point, so
shadowing can change the answer:

```dijk
let mut x = 1;
if true {
    let x = 2;   // a new, immutable `x` hides the mutable one
    x = 3;       // semantic error: this `x` is immutable
}
x = 4;           // fine: the outer, mutable `x` again
```

Whether the assigned *value* has the variable's type is a different
rule, about values, and belongs to the type checker (E0007 in
`docs/types.md`). Semantic analysis accepts `let mut x = 1; x = "one";`;
the type checker rejects it. The two are implemented, reported and
tested separately.

## Scopes

Where a variable is visible is decided by where it is *written*, by its
enclosing braces. This is **lexical scoping**. Scopes nest:

```text
Global scope            every function
    │
    └── Function scope  parameters, and `let`s directly in the body
           │
           └── Block scope   `let`s inside an `if`/`else` branch or a
                             `while` body, nested as deep as the blocks are
```

Dijk has no global variables, so the global scope holds only functions.

A block's variables disappear when the block ends. This is Step 4's own
example:

```dijk
fn main() {
    let x = 10;

    if true {
        let y = 20;
    }

    return y;     // semantic error: unknown identifier `y`
}
```

The rules within that structure:

- **Lookup goes outward.** A use of a name searches the innermost scope
  first, then each enclosing scope. Outer variables are visible (and
  assignable) from inner blocks.
- **Shadowing is allowed in an inner scope.** `let x` inside a block may
  reuse the name of an outer `x`. The inner variable hides the outer one
  until the block ends, and the outer one is then visible again,
  unchanged.
- **Redeclaring in the same scope is an error.** A second `let x` in the
  *same* scope would make the first `x` silently unreachable, so it is
  reported instead.
- **Parameters share the function scope** with the body's top-level
  `let`s, so `fn f(n: i32) { let n = 2; }` is a redeclaration. (A block
  inside the body may shadow `n`.)
- **A `let` initializer is resolved before the name is declared.**
  `let x = x + 1;` refers to an *outer* `x`. If there is none, it is an
  unknown identifier, never the variable being declared.
- **Each branch is its own scope.** The `then` and `else` branches cannot
  see each other's variables. `else if` is represented as an `else`
  branch holding one `if` (`docs/ast.md`), so it gets a scope of its own
  too.
- **Functions are isolated.** A function body sees its own parameters and
  locals, and the global functions. It never sees another function's
  variables, including its caller's.

## Functions, and why there are two passes

```text
Pass 1: collect every function's name and parameter count
Pass 2: walk every function body, checking each name against what
        pass 1 collected and what the body has declared so far
```

Collecting every function first means that, by the time any body is
checked, every function is known. So a function can call itself
(recursion), two functions can call each other (mutual recursion), and a
call can name a function defined *later* in the file. Variables get no
such treatment: a variable must be declared before the point where it is
used.

Dijk has no first-class functions, so the two kinds of name never meet.
A name in call position (`foo(...)`) can only mean a function, and is
looked up only among functions. A name anywhere else can only mean a
variable, and is looked up only among variables. `return foo;`, where
`foo` is a function, is therefore an unknown *identifier*.

## Implementation

`semantic::analyze(&Program) -> Vec<SemanticError>` is the entry point
(`src/semantic/mod.rs`). It must only be given a program that parsed
without errors. A partial AST (functions or statements dropped by parser
error recovery) would produce misleading semantic errors, so `dijk run`
never analyzes one.

The resolver (`src/semantic/resolve.rs`) is built on the AST's `Visitor`
(`src/ast/visit.rs`), which makes semantic analysis the visitor's second
real consumer after `dijk ast`'s node counts. Analysis only *looks* at
nodes. It never computes a value from them, which is exactly what the
visitor does. The resolver overrides:

- `visit_function`, to open the function scope and declare the
  parameters;
- `visit_statement`, to declare `let`s, with their mutability (after
  resolving the initializer), and to open a block scope around each
  `if`/`else` branch and `while` body (the AST has no `Block` node, so these are the places a scope
  must be opened explicitly);
- `visit_expression`, to check identifiers, assignment targets (that
  they exist, and are mutable) and calls.

Everything else keeps descending through the default `walk_*` functions.

Two tables do the bookkeeping:

```text
functions: name → parameter count          the global scope (pass 1)
scopes:    [ {name → kind}, ... ]          a stack: function scope first,
                                           innermost block scope last
```

Each scope maps the names it declares to the kind of variable each one
is: a parameter, an immutable `let`, or a mutable `let mut`. Entering a
block pushes an empty scope, and leaving it pops the scope. A lookup
searches the stack from the top down.

### Errors

Like the lexer and parser, semantic analysis **collects** errors rather
than stopping at the first one, and returns them in source order. A name
that fails to resolve is reported once. A `let` whose initializer had an
error still declares its name, so later uses don't cascade into more
errors.

Each error is rendered by the shared renderer (`src/diagnostics.rs`)
with the label `semantic error:`. That is how a reader tells it apart
from a lexical or parse `error:` and from a `runtime error:`:

```text
semantic error: wrong number of arguments to `add`: expected 2, found 1
 --> example.dk:6:13
  |
6 |     let x = add(1);
  |             ^
```

Where the caret points:

- **Identifiers, assignments and calls:** at the name.
- **Duplicate parameters:** at the parameter.
- **Duplicate `let`s and duplicate functions:** at the `let` or `fn`
  keyword. (The AST does not record a separate span for a function's
  name.)
- **A missing `main`:** it has no location of its own, so it is anchored
  at the end of the program, where a `main` could be added, or at the
  start of an empty file.

## Relationship to the type checker and the interpreter

The type checker (`docs/types.md`) runs only on programs that passed
semantic analysis, and the interpreter (`docs/interpreter.md`) only on
programs that passed both. Each keeps its own twin of the resolver's
scope stack. The resolver records *which* names exist, the type checker's
environment what *type* each has, and the interpreter's environment each
name's current *value*. Because all three follow the same scoping rules,
every name the later stages look up is found, and neither of them
reports an unresolved name again.
