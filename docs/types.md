# The Type System

This document describes Dijk Core's static type system and the type
checker that enforces it (`src/types/`). It covers the types, the rules
for every construct, how much is inferred, the diagnostics, and what the
system deliberately cannot express yet.

The type checker answers one question:

> **Are this program's operations valid for the values they are applied
> to, before the program runs?**

Semantic analysis (`docs/semantic.md`) has already checked that every
name refers to something. That is not enough. In this program every name
resolves:

```dijk
fn main() -> i32 {
    let x: i32 = "hello";
    return x;
}
```

But `x` was declared to hold an `i32`, and it is given a string. The
type checker reads the program, without executing any of it, and
rejects it:

```text
type error[E0001]: type mismatch
 --> main.dk:2:18
  |
2 |     let x: i32 = "hello";
  |                  ^^^^^^^
  = expected `i32`
  = found `string`
```

A program can be valid or invalid at several distinct levels, and each
level is the job of a different stage:

```text
1. lexically valid        the lexer       (docs/lexer.md)
2. syntactically valid    the parser      (docs/parser.md)
3. semantically valid     names resolve   (docs/semantic.md)
4. type-correct           this document
5. executable             the interpreter (docs/interpreter.md)
```

A program with a type error **never runs**. `dijk run` checks types
before interpreting, and stops if there is any error. Being type-correct
does not guarantee that a program runs without a runtime error, though.
Dividing by zero is well-typed.

## The types

| Type | Values | Written as |
|---|---|---|
| `i32` | 32-bit signed integers, −2147483648 to 2147483647 | `42`, `-7` (the default type of an integer literal) |
| `i64` | 64-bit signed integers, −9223372036854775808 to 9223372036854775807 | `42` where an `i64` is expected |
| `bool` | `true`, `false` | `true` |
| `string` | text (no escapes yet) | `"hello"` |
| `unit` | "no value": what a function without `-> T` returns | (no literal; cannot be written) |

`unit` is internal. There is no `()` literal, and `unit` is not a name a
program may use in an annotation. A function that returns nothing simply
leaves out `-> T`. Annotations may name exactly `i32`, `i64`, `bool` and
`string`. Any other name, `unit` included, is an unknown type (E0008).

Floats, `char`, unsigned integers, arrays, structs, enums, tuples,
references, pointers and generic types are all deliberately absent.

### A type is not a memory location

A type says **what kind of value something is, and which operations are
valid on it**. It does not say **where the value lives**: in a register,
in a stack slot, or on the heap. `i32` promises "a 32-bit signed integer
on which `+`, `<`, `==` and `-` are valid". It promises nothing about
storage. That question belongs to the memory model, a later milestone,
and Dijk keeps the two apart on purpose.

## Compatibility: no implicit conversions

Two types are compatible exactly when they are **the same type**. There
are no implicit conversions, not even the "safe" widening one:

```dijk
let x: i32 = 10;
let y: i64 = x;     // type error[E0001]: expected `i64`, found `i32`
```

Every `i32` value would fit in an `i64`, but Dijk never converts between
types on its own. A conversion will be written explicitly when Dijk has
one, probably as `x as i64`. **There is no `as` (or any cast) yet.** This
section records the policy that casts, when added, will follow:
conversions are always explicit and visible in the source.

## Typing judgments

The rules below are written as *typing judgments*:

```text
Γ ⊢ e : T
```

It reads: **"in the environment Γ, the expression `e` has type `T`."**

- **Γ** (gamma) is the *typing environment*: the type of every variable
  in scope, such as `{ x : i32, name : string }`.
- **⊢** ("turnstile") separates what is known from what follows from it.
- **`e : T`** is the conclusion: `e` has type `T`.

A rule has premises above a line and a conclusion below it. If every
premise holds, the conclusion holds:

```text
Γ ⊢ a : i32    Γ ⊢ b : i32
───────────────────────────
      Γ ⊢ a + b : i32
```

"If `a` has type `i32` and `b` has type `i32`, then `a + b` has type
`i32`." The checker applies rules like this one from the leaves of the
expression tree upward. It never computes a value. `10 + 20 / 0` is just
as much an `i32` as `10 + 20 * 3`.

## Literals

```text
─────────────────      ──────────────────      ───────────────────
Γ ⊢ true : bool        Γ ⊢ false : bool        Γ ⊢ "…" : string
```

An integer literal is the one expression whose type depends on its
context. The checker passes every expression the type its context
*expects*, if it expects one, and an integer literal takes that type when
it is an integer type:

```text
42    expected nothing        → i32    (the default)
42    expected i64            → i64
42    expected bool           → i32    (only a fallback; the context
                                        reports the mismatch)
```

The contexts that expect a type are an annotation (`let x: i64 = 42`), a
parameter (`f(42)`), a return type (`return 42;`), the variable being
assigned (`x = 42`), a condition (which expects `bool`), and the other
operand of a binary operator (below).

**Range.** A literal's value must fit in its type, or it is E0010:

```text
let a = 2147483647;                      fine: the largest i32
let b = 2147483648;                      type error[E0010]: integer literal out of range
let c: i64 = 2147483648;                 fine: an i64
let d: i64 = 9223372036854775808;        type error[E0010]: one past the largest i64
```

A minus sign written directly before digits is **part of the literal**,
so `-2147483648`, the smallest `i32`, is accepted even though the parser
reads it as `-` applied to `2147483648`, which alone would not fit. The
same goes for `-9223372036854775808`, the smallest `i64`. The parser
reads any digits that fit in 64 bits (unsigned), and the range check is
left entirely to the type checker, the one stage that knows both the
literal's sign and its type.
Parentheses leave no trace in the AST, so `-(2147483648)` is the same
literal. `--5` and `0 - 5` are operations on literals, not literals.

Where no integer is wanted at all (`let b: bool = 3000000000;`), the
literal's range is not checked. The mismatch is the one mistake there.

## Operators

Every operator has a table of rules (`src/types/rules.rs`), one row per
combination of operand types it accepts. Any other combination is a
type error: E0002 for a binary operator, E0003 for a unary one.

| Operator | Rules | Result |
|---|---|---|
| `+` | `i32 + i32`, `i64 + i64`, `string + string` | the operand type (`+` on strings concatenates) |
| `-` `*` `/` | `i32 ∘ i32`, `i64 ∘ i64` | the operand type |
| `<` `<=` `>` `>=` | `i32 ∘ i32`, `i64 ∘ i64` | `bool` |
| `==` `!=` | `i32 ∘ i32`, `i64 ∘ i64`, `bool ∘ bool`, `string ∘ string` | `bool` |
| `-` (unary) | `-i32`, `-i64` | the operand type |
| `!` | `!bool` | `bool` |

No operator accepts `unit`. There is no `&&`, `||` or `%` in the
language yet. Step 5's table lists `true && false : bool`, which will be
a row of `rules.rs` once the grammar has `&&`.

Rejected, for example:

```text
10 + "hello"       E0002   expected `i32 + i32`, `i64 + i64` or `string + string`
true * 10          E0002   found `bool * i32`
"hello" < false    E0002
a + b              E0002   when a : i64 and b : i32
1 < 2 < 3          E0002   it is (1 < 2) < 3: a bool compared with an i32
-true              E0003
!1                 E0003
```

**Literal operands.** A literal operand takes its type from the *other*
operand, so `x + 1` and `1 + x` both work whatever integer type `x` has.
The left operand is checked first, and its type is what the right one is
expected to have. The exception is when the left operand is a bare
literal and the right one is not; then the order is swapped. For `+ - *
/`, whose result has the operands' type, what the context expects of the
result is also passed down to the operands, so in
`let x: i64 = 1 + 2 * 3;` every literal is an `i64`.

## Variables, and what is inferred

A `let` gives its variable a type in one of two ways:

```text
Γ ⊢ e : T                                    Γ ⊢ e : T    (e checked against T)
────────────────────────────── (infer)       ──────────────────────────────────── (annotated)
let x = e;     adds x : T to Γ               let x: T = e;   adds x : T to Γ
```

- **Inference** (`let x = 10;`): the variable takes the initializer's
  type, `i32`.
- **Checking** (`let x: i64 = 10;`): the annotation is the expected type
  of the initializer, which must then have exactly that type (E0001
  otherwise). The variable gets the annotated type.

Either way, a variable's type is fixed at its declaration and never
changes. Every later use of the variable has that type.

Scopes follow exactly the rules semantic analysis checks: each `if`/
`else` branch and `while` body is a block scope, an inner `let` may
shadow an outer variable (even with a different type), and an
initializer is checked *before* its variable is declared, so
`let x = x + 1;` reads an outer `x`. The type environment is the third of
three twins:

```text
semantic analysis   name → (exists)     which names are visible?
type checker        name → Type         what kind of value is each?
interpreter         name → Value        what value does each hold now?
```

### The inference policy

| What | Inferred? |
|---|---|
| A `let` variable's type | Yes, from its initializer, unless annotated |
| An integer literal's type | Yes, from what its context expects, or `i32` |
| A parameter's type | No: always written (`a: i32`) |
| A function's return type | No: written as `-> T`, or `unit` if absent |

That is the whole of it. Information flows from the context **down**
into an expression, once. Nothing flows back up, and nothing is solved
for. There is no Hindley–Milner inference, no type variables and no
constraint solving. Types are inferred only where they can be read
straight off the code in front of them. This is deliberately *local*
inference: each `let` is typed from its own line.

## Assignments and mutability

`x = value` stores a value in an existing variable, so the value must
have the variable's type (E0007). The variable's type is also the
expected type of the value:

```dijk
let mut total: i64 = 0;
total = 3000000000;        // fine: the literal is an i64
total = "many";            // type error[E0007]: expected `i64`, found `string`
```

An assignment is itself an expression of the variable's type, which is
what makes `a = b = 0` work.

**Mutability is a separate rule**, and not a type rule. A type
constrains the *values* a binding may hold. Mutability constrains whether
the *binding* may change at all. Only a `let mut` may be assigned to.
Assigning to a plain `let` or to a parameter (parameters cannot be
declared `mut`) is a **semantic error**, reported by semantic analysis
before type checking starts (`docs/semantic.md`):

```text
semantic error: cannot assign to immutable variable `x` (declare it with `let mut x` to allow assignment)
```

The two rules are implemented, reported and tested separately.

## Functions

A function's **signature** is its type: the types of its parameters and
its return type. A function without `-> T` returns `unit`.

```dijk
fn add(a: i32, b: i32) -> i32 {     // add : (i32, i32) -> i32
    return a + b;
}
```

A signature is a contract between a function and its callers, and lets
each side be checked without looking at the other:

- **A call** is checked against the signature alone. Each argument must
  have its parameter's type (E0004), and that type is what a literal
  argument is expected to be. The call expression has the return type.

  ```text
  Γ ⊢ f : (T1, …, Tn) -> R    Γ ⊢ e1 : T1   …   Γ ⊢ en : Tn
  ──────────────────────────────────────────────────────────
                    Γ ⊢ f(e1, …, en) : R
  ```

- **A body** is checked with its parameters in Γ at their declared
  types, and its `return`s are checked against the return type.

Every signature is collected **before** any body is checked, so a
function may call a function declared later in the file, call itself,
or call a function that calls it back. Unknown type names in parameters
or return types are E0008.

Whether a called function *exists*, and whether a call passes the right
*number* of arguments, are questions about names, answered by semantic
analysis before the checker runs. The type checker does not repeat them.

`main` takes no parameters (a semantic rule) and may return any type.
`dijk run` prints the value it returns, and nothing for `unit`.

## Returns and return paths

Every `return` is checked against the enclosing function's return type
(E0005):

```text
Γ ⊢ e : R                                      R = unit
──────────────────────────────────────        ───────────────────────────────
return e;  is valid in a function -> R         return;  is valid in a function -> R
```

So `return "hello";` in a `-> i32` function is E0005, and so are a bare
`return;` in a `-> i32` function and `return 1;` in a function without
`-> T`. A bare `return;` counts as `unit`, so the rule is stated on types
alone: a unit function may return the result of a unit call.

Checking each `return` is not enough, though. This function passes
every `return` check and is still wrong:

```dijk
fn get_number() -> i32 {
    if true {
        return 10;
    }
}
```

The checker never evaluates `true`, so as far as it can tell the `if`
may be skipped, and the function would then run off its end with no
value to return. A function whose return type is not `unit` must
**definitely return**, judged from the shape of its body alone:

| Statement | Definitely returns when |
|---|---|
| a block | any statement in it definitely returns (what follows can never run) |
| `return …;` | always |
| `if … { } else { }` | both branches definitely return |
| `if … { }` (no `else`) | never: the condition might be false |
| `while … { }` | never: the condition might be false the first time, even `while true`, because conditions are never evaluated |
| `let`, an expression statement | never |

Breaking this rule is E0009, reported at the function's closing `}`:

```text
type error[E0009]: missing return in `get_number`
 --> main.dk:5:1
  |
5 | }
  | ^
  = expected `i32`
  = found the end of `get_number`, with no `return`
  = note: a `return` inside an `if` without `else`, or inside a `while`, might never run
```

This is the checker's first piece of **control-flow analysis**. It is
conservative: it never accepts a function that might not return, but it
rejects some that always do in practice, such as one whose only
`return` is inside `while true { … }`. Adding `return` after the loop
satisfies it.

## Conditions

The condition of every `if`, `else if` and `while` must be a `bool`
(E0006):

```text
Γ ⊢ c : bool
──────────────────────────────
if c { … }  and  while c { … }  are valid
```

Dijk has no "truthiness": `if 1 { … }` is an error, not "true because
non-zero", and so are `if "yes"` and a condition that is a `unit` call.

## Diagnostics

Type errors use the shared diagnostic renderer (`src/diagnostics.rs`)
with the label `type error[CODE]:`. A type belongs to a whole
expression, so the whole offending expression is underlined (on the
first line it spans), and the notes say what type the context *expected*
and what the checker *found*:

```text
type error[E0002]: invalid binary operation: `+`
 --> op.dk:2:17
  |
2 |     let total = 10 + "hello";
  |                 ^^^^^^^^^^^^
  = expected `i32 + i32`, `i64 + i64` or `string + string`
  = found `i32 + string`
```

| Code | Meaning | Example |
|---|---|---|
| E0001 | Type mismatch | `let x: i32 = "hello";` |
| E0002 | Invalid binary operation | `10 + "hello"` |
| E0003 | Invalid unary operation | `-true`, `!1` |
| E0004 | Invalid function argument | `add(10, "hello")` |
| E0005 | Invalid return type | `return "hello";` in `-> i32`; `return;` in `-> i32`; `return 1;` with no `-> T` |
| E0006 | Invalid condition type | `if 10 { … }` |
| E0007 | Invalid assignment | `x = "hello";` for an `i32` `x` |
| E0008 | Unknown type name | `let x: banana = 1;`, `-> unit` |
| E0009 | Missing return on some path | an `if` without `else` as a function's last statement |
| E0010 | Integer literal out of range | `let x = 3000000000;` |

**Codes are stable and never reused.** A code always means the same rule.
If a rule is ever retired, its code is retired with it, and a new rule
gets a new code.

The checker reports **every** type error it finds, in source order, and
each mistake **once**. An expression that failed gets an internal error
type that is compatible with everything, so using it later (in an
operator, a call, an assignment) is not reported again. A variable with
an unknown annotated type is untyped in the same way.

Only type errors have codes. Lexical and parse errors keep the label
`error:`, semantic analysis keeps `semantic error:`, and the interpreter
keeps `runtime error:`. Those stages also keep a single caret at the
start of the problem.

Every stage reports positions the same way. Lines and columns count
**characters**, not bytes (`é` is one column), and the underline is
measured in characters too. A tab counts as one column, and the caret
line repeats the source line's tabs, so the two line up in any terminal.
A file with Windows line endings is shown without its `\r`. The one
remaining limit: characters a terminal draws two columns wide (most CJK
characters and emoji) still count as one.

## `dijk check`

```text
dijk check <file.dk | file.dijk>
```

`dijk check` runs every stage up to and including type checking, reports
each one, and **never runs the program**:

```text
$ dijk check examples/add.dk
Checking examples/add.dk...
Lexical analysis       OK
Parsing                OK
Semantic analysis      OK
Type checking          OK

Program is well-typed.
```

It stops at the first stage that fails, marked `FAILED` with the number
of errors, and prints that stage's diagnostics to stderr. The exit code
is 0 only for a well-typed program. `examples/runtime_error.dk` passes
`dijk check`, because dividing by zero is well-typed, and fails only
under `dijk run`.

### `dijk check --types`: the typing environment made visible

```text
dijk check --types <file.dk | file.dijk>
```

After a successful check, `--types` prints what the checker learned:
each function's signature, then every variable it declares (parameters
first, then each `let` in source order) with the type it was given and
where its name is. Each line is one entry of Γ, `name : type`:

```text
$ dijk check --types examples/add.dk
…
Program is well-typed.

fn add(a: i32, b: i32) -> i32
    a      : i32   (1:8)
    b      : i32   (1:16)
    result : i32   (2:9)
fn main() -> i32
```

Variables declared in nested blocks are indented one step per level, so
a shadowing variable appears as its own entry. A `let mut` is marked
`mut` at the end of its line, outside the `name : type` column, because
mutability is not part of a type. A function without `-> T` is shown as
`-> unit`, the type it really returns:

```text
fn main() -> unit
    x         : i32      (2:9)
        x     : string   (4:13)
        count : i32      (5:17)  mut
```

Only variables are listed, not the type of every expression. A program
with errors prints its diagnostics and no listing.

## Implementation

```text
src/types/
├── mod.rs     the stage's contract: check(&Program) → TypedProgram or errors
├── ty.rs      what a type is: the Type enum, equality, display, annotations
├── rules.rs   the operator typing rules, as a table
├── env.rs     the type environment Γ: a stack of name → Type scopes
├── check.rs   the checker: a recursive walk over the AST
└── error.rs   type errors, their codes, messages and notes
```

- **Entry point.** `types::check(&Program) -> Result<TypedProgram,
  Vec<TypeError>>`. It must only be given a program that parsed and
  passed semantic analysis. It relies on every name resolving, and does
  not report unresolved names again.
- **The walk.** Like the interpreter, and unlike semantic analysis, the
  checker is a direct recursive `match`, not a `Visitor`, because
  checking an expression must produce something: its type. The central
  operation, `check_expression(expr, expected) → Type`, is the static
  twin of the interpreter's `evaluate(expr) → Value`.
- **Two passes.** Pass 1 collects every signature. Pass 2 checks every
  body, then its return paths (`always_returns`, a pure function of the
  body's shape).
- **Error recovery.** `Type::Error` is not a Dijk type. It is the type of
  an expression already reported as wrong, compatible with every type,
  and never recorded in a `TypedProgram`.
- **What the checker produces.** A `TypedProgram` keeps what was learned
  so that later stages need not work it out again: each function's
  `FunctionSignature` and its `Binding`s (name, span of the name, type,
  mutability, block depth), and the type of every expression. The AST
  has no node IDs, so expression types live in a side table **keyed by
  the expression's span**. This is sound because no two expressions in a
  program share a span: a parent strictly contains its children, and
  siblings never overlap (a test pins this invariant).
- **Dependency direction.** `types` depends on the AST (and the lexer's
  `Span`), never on the interpreter. The interpreter may consume the
  checker's results, and does so in exactly one place: an integer
  literal becomes a `Value::I32` or a `Value::I64` according to the type
  the checker recorded for it (`docs/interpreter.md`). The two stages
  share `types::literal_value`, so they agree that `-2147483648` is one
  literal.

## Limitations

What this first type system cannot express or does not check, on
purpose:

- **No conversions.** There is no `as` yet, so an `i32` can never be used
  where an `i64` is expected. The policy is that conversions will be
  explicit.
- **A small set of types.** No floats, `char`, unsigned integers, arrays,
  structs, enums, tuples, references, pointers, function values or
  generics.
- **Literal operands only borrow the other operand's type when they are
  bare literals.** `(1 + 2) == x` with `x : i64` is rejected (the left
  side is checked first, as an `i32`); `x == 1 + 2` is accepted.
- **Return paths are judged from shape alone.** A `while` never counts as
  returning, even `while true`.
- **Diagnostics underline only the first line of a span**, and a span
  that starts or ends inside parentheses excludes them (`!(a == b)` is
  underlined without its closing `)`).
- **No warnings.** Unreachable code after a `return`, unused variables
  and unused results are not reported.
- **No `&&`, `||`, `%`** (not in the grammar), and no `print`.
