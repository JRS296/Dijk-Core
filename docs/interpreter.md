# The Interpreter

This document describes how Dijk Core executes a program
(`src/interpreter/`): runtime values, environments, evaluation, control
flow, function calls and the call stack, return propagation, and runtime
errors. It also describes `dijk run`, which puts every stage together.

The interpreter answers one question:

> **What does this program do?**

```bash
dijk run hello.dk
```

```dijk
fn main() -> i32 {
    let x = 10;
    let y = x + 20;

    return y;
}
```

```text
30
```

Every stage before this one only *described* the program. The lexer found
its pieces (`docs/lexer.md`), the parser its structure (`docs/parser.md`),
the AST recorded what was written (`docs/ast.md`), semantic analysis
checked that its names mean something (`docs/semantic.md`), and type
checking that its operations are valid for their values
(`docs/types.md`). The interpreter is the first stage that *does* what
the program says:

```text
Source → Lexer → Parser → AST → Semantic analysis → Type checking → Interpreter → 30
```

## A tree-walking interpreter

Dijk's interpreter evaluates the AST directly, node by node, with no
further translation. To evaluate a node, it evaluates the node's children
and then combines their values:

```text
10 + 20 * 3

Binary(+)                  evaluate(Integer(10))       → 10
├── Integer(10)            evaluate(Binary(*))
└── Binary(*)                  evaluate(Integer(20))   → 20
    ├── Integer(20)            evaluate(Integer(3))    → 3
    └── Integer(3)             20 * 3                  → 60
                           10 + 60                     → 70
```

The shape of the tree *is* the order of the computation. The parser
already put `*` below `+`, so the interpreter needs to know nothing about
precedence. This is the payoff of building an AST.

The interpreter is a direct recursive `match` over node kinds
(`src/interpreter/eval.rs`). It does not use the AST's `Visitor`, which
only *looks* at nodes and returns nothing. Evaluating a node must produce
a value and must be able to fail.

## Runtime values

**AST nodes are source. Evaluating them produces runtime values.** An
`Expression::Integer(10, span)` records that someone *wrote* `10`, and
where. A `Value::I32(10)` is the number ten, produced while the program
runs, with no source position at all. `1 + 2` and `3` are different
expressions that evaluate to the same value.

```text
Value                      src/interpreter/value.rs
├── I32(i32)               a value of type i32, e.g. 30
├── I64(i64)               a value of type i64, e.g. 3000000000
├── Boolean(bool)          true, false
├── String(String)         e.g. hello  (printed without quotes)
└── Unit                   "no value": what a function returns when it
                           returns nothing
```

There is **one variant per type** (`docs/types.md`), so every value
carries its type at run time, and `I32(5)` and `I64(5)` are different
values. Floats, arrays, structs, references and functions-as-values will
each add a variant when the language gains them.

### Integers: `i32` and `i64`

The type checker decides whether each integer literal is an `i32` or an
`i64` (an `i32` unless its context expects an `i64`) and records the
decision in the `TypedProgram` it produces. Evaluating a literal looks
that decision up and builds a value of that type. That is the **only**
thing the interpreter takes from the type checker. Every other integer
is the result of an operation, and has its operands' type.

Integer arithmetic is **defined by Dijk**, not inherited from whatever
the implementation language happens to do:

```text
1. compute the true, mathematical result (exactly, in a wider type)
2. if it fits in the operands' type, that is the result
3. otherwise it is a runtime error: integer overflow
```

So an `i32` overflows exactly where a real 32-bit integer would:

```text
2147483647 + 1               i32: runtime error: integer overflow … `i32`
2147483647 + 1 (as i64)      i64: 2147483648
-2147483648 / -1             i32: runtime error: integer overflow
```

Nothing ever silently wraps around, and nothing crashes. Division
truncates toward zero (`-7 / 2` is `-3`), and dividing by zero is a
runtime error. Negating, or dividing by `-1`, the smallest value of a type
overflows, because the true result is one more than its largest value.

Before `dijk-0.0.5`, every integer was an `i64` at run time, whatever
its annotation said, so a program checked as `i32` could quietly compute
values no `i32` can hold. Distinct runtime values keep the running
program consistent with the types it was checked against.

## The environment

A running program's variables live in an **environment**
(`src/interpreter/environment.rs`). At its simplest, an environment is a
table from names to values:

```text
let x = 10;        Environment
let y = x + 20;    ┌──────────────┐
                   │ x → 10       │
                   │ y → 30       │
                   └──────────────┘
```

Scopes nest, though, so an environment is a *stack* of such tables, one
per open scope. The function scope is at the bottom; a block scope is
pushed when a block is entered and popped (with all its variables) when
it ends:

```text
Function scope   x → 10          ← opened when the call starts
    │
    └── Block scope   y → 20     ← opened on entering a block,
                                    discarded when it ends
```

| Operation | Language construct | Behavior |
|---|---|---|
| `define(name, value)` | `let` | Declare in the innermost scope. An outer variable of the same name is shadowed, not changed |
| `lookup(name)` | using a variable | Search innermost → outermost |
| `assign(name, value)` | `x = …` | Change the innermost variable of that name. Never creates one |
| `push_scope` / `pop_scope` | entering / leaving a block | The function scope is never popped |

This is the run-time twin of the scope stack semantic analysis keeps (and
of the type checker's environment of types), and it follows exactly the
same rules (`docs/semantic.md`). There is no global
variable scope, because Dijk has no global variables. Its only global
names are functions, which the interpreter keeps in a separate table.

## Expressions

| Expression | Evaluates to |
|---|---|
| `42`, `true`, `"hi"` | the corresponding value |
| `x` | the current value of the variable `x` |
| `-e`, `!e` | negation of an integer, or of a boolean |
| `a op b` | see the operator rules below |
| `x = e` | the value of `e`, after storing it in `x` |
| `f(a, b)` | the value `f` returns (see [Function calls](#function-calls)) |

- **Order.** Operands and arguments are evaluated **left to right**. Both
  operands of a binary operator are always evaluated: Dijk has no
  short-circuiting `&&`/`||` yet.
- **Assignment is an expression.** It evaluates to the assigned value,
  which is what makes the right-associative chain `a = b = 7` assign `7`
  to both.

The type checker has already made sure every operator is applied to
operands it has a rule for (`docs/types.md`), so the interpreter only
has to give those rules their meaning:

```text
i32 + - * / i32,  i64 + - * / i64   → same type   (checked: overflow is an error)
string + string                     → string      (concatenation)
i32 < <= > >= i32, same for i64     → bool
a == != a   (i32, i64, bool, string) → bool
- i32, - i64                        → same type   (checked)
! bool                              → bool
```

Anything else (`1 + true`, `"a" - "b"`, an `i32` plus an `i64`) is a
type error, and a program containing one never runs. The interpreter
still reports it as a runtime `invalid operation`, rather than crashing,
if it is ever handed a program nobody checked.

## Statements and control flow

Statements are *executed*, not evaluated. They run for their effect on
the environment, not for a value.

| Statement | Effect |
|---|---|
| `let x = e;` | evaluate `e`, then define `x` in the current scope |
| `e;` | evaluate `e` and discard the value |
| `if c { … } else { … }` | evaluate `c`; run the first block if it is `true`, otherwise the `else` block if there is one |
| `while c { … }` | evaluate `c`; while it is `true`, run the block and evaluate `c` again |
| `return e;` / `return;` | end the current function call with the value of `e` / with `Unit` |

- **Conditions are booleans.** There is no "truthiness": `if 5` and
  `while 0` are type errors (E0006), not "true because it's non-zero".
- **Every branch and loop body runs in a block scope of its own.** It is
  pushed on entry and popped on exit however the block finishes. A `while`
  body gets a **fresh scope on every iteration**, so a `let` inside a loop
  declares a new variable each time around.
- `else if` needs no special handling. The AST represents it as an `else`
  branch holding a single `if` (`docs/ast.md`).
- `while true { }` with no `return` inside runs forever. Nothing limits
  how long a program runs.

## Function calls

Functions live in the **global scope**: a table from each function's name
to its declaration, built before `main` starts. Every call gets a **call
frame**: a fresh environment of its own, holding the callee's parameters
and everything its body declares. The frames of all calls in progress form
the **call stack**:

```dijk
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

fn main() -> i32 {
    let x = add(10, 20);
    return x;
}
```

```text
Call stack, while `add` runs:

┌─────────────────────┐
│ add   a → 10        │  ← top: the running call
│       b → 20        │
├─────────────────────┤
│ main                │  ← waiting for `add` to return (x not yet declared)
└─────────────────────┘
```

A call goes through these steps:

```text
1. Look up the function in the global scope.
2. Evaluate the arguments, left to right, in the caller's frame.
3. Push a new frame; bind each parameter to its argument, in order.
4. Run the body in that frame.
5. Pop the frame. The call evaluates to what the body returned.
```

Consequences worth knowing:

- **A function sees only its own frame.** It can neither read nor change
  its caller's variables. They are in a different environment.
- **Arguments are passed by value.** Parameters are immutable, so a
  function that wants to change an argument copies it into a `let mut`,
  and that copy is the callee's alone. The caller's variable never
  changes.
- **Each call gets fresh locals.** Nothing persists from one call of a
  function to the next.
- **Recursion needs nothing special.** Each recursive call pushes another
  frame, with its own copy of every parameter:

```dijk
fn factorial(n: i32) -> i32 {
    if n <= 1 {
        return 1;
    }

    return n * factorial(n - 1);
}
```

```text
factorial(5)
  = 5 * factorial(4)
          = 4 * factorial(3)
                  = 3 * factorial(2)
                          = 2 * factorial(1)
                                  = 1          ← deepest point
                          = 2 * 1 = 2
                  = 3 * 2 = 6
          = 4 * 6 = 24
  = 5 * 24 = 120
```

At the deepest point, six frames are on the call stack, each with its own
`n`:

```text
┌──────────────────┐
│ factorial  n → 1 │  ← top: the running call
│ factorial  n → 2 │
│ factorial  n → 3 │
│ factorial  n → 4 │
│ factorial  n → 5 │
│ main             │
└──────────────────┘
```

In a real machine, a stack frame also holds a return address and saved
registers. Here the Rust code that made the call (`call_function`, in
`eval.rs`) is itself the "return address": when the body finishes, Rust
returns to it. A Dijk frame therefore only needs to hold its function's
name and its variables.

### The depth limit, and the interpreter's own stack

Every Dijk call nests a dozen or so Rust calls (`call_function` →
`execute_statement` → `evaluate` → …), so **Dijk's call stack lives on
top of Rust's**. A program that recursed forever would eventually exhaust
Rust's stack and crash the interpreter itself. Two things prevent that:

- **At most 1000 calls may be in progress at once** (`MAX_CALL_DEPTH`,
  `main` included). The 1001st is a runtime error, the interpreter's
  version of a stack overflow. The limit counts calls *in progress*, not
  calls made: `fib(20)` makes 21,891 calls, but never more than 21 at a
  time.
- **The interpreter runs on a thread with a known stack size**
  (`STACK_SIZE` in `src/interpreter/mod.rs`, 256 MiB). An unoptimized
  build can spend tens of kilobytes of Rust stack per Dijk call, so 1000
  calls do not fit in a thread's default stack (8 MiB for a program's
  main thread, 2 MiB for each `cargo test` thread). The size is address
  space *reserved*, not memory allocated: the operating system only backs
  the pages actually touched. If the operating system refuses a stack that
  large, the interpreter falls back to the current thread.

This mirrors what a machine does: a thread's stack has a fixed size, and
exceeding it is a stack overflow. The difference is that here the
overflow is detected and reported.

One limitation remains. The amount of Rust stack a single Dijk call needs
grows with how deeply its source is nested, for example an expression
inside hundreds of nested parentheses. Recursing 1000 levels through such
a function can still exhaust even the interpreter's stack. The parser has
the same weakness on deeply nested source. Both go away once the parser
limits nesting depth.

## Return propagation

`return` may sit several blocks deep inside a function, but it ends the
whole *call*, not just its own block. Every statement between the `return`
and the end of the function must be skipped:

```text
fn find() -> i32 {
    while true {
        if found() {
            return 10;     → Return(10)
            log();         skipped: the block stops at a Return
        }                  if:    passes Return(10) outward
        step();            skipped
    }                      while: stops looping, passes it outward
    cleanup();             skipped
}                          call:  Return(10) becomes the call's value
```

So executing a statement reports *how it finished*. Together with failure,
every statement ends in exactly one of three ways:

```text
Ok(Execution::Normal)          next statement, please
Ok(Execution::Return(value))   unwind to the function call
Err(RuntimeError)              unwind all the way out and report
```

A sequence of statements stops at the first `Return`. `if` passes it
outward, and `while` stops looping and passes it outward. Only a function
call consumes it, turning it into the call's value. A body that runs out
of statements returns `Unit`, and so does `return;`. The type checker
has made sure both only happen in a function that returns `unit`: a
function with `-> T` must return a value of type `T` on every path. Expressions, by
contrast, simply produce `Result<Value, RuntimeError>`: an expression
cannot contain a `return`.

## Runtime errors

Semantic analysis rejects programs that cannot mean anything, and the
type checker programs whose operations don't fit their values. Some
problems, though, depend on the *values* a program computes, and no
earlier stage can see them. They are **runtime errors**, reported against
the expression being evaluated when things went wrong:

```text
runtime error: division by zero
 --> examples/runtime_error.dk:2:16
  |
2 |     return a / b;
  |                ^
note: call stack: divide ← main
```

The final line is the **call stack at the moment of the error**, innermost
call first. As the error leaves each call on its way out, it records that
call's frame. A runaway recursion would list a thousand frames, so a long
stack is abbreviated:
`forever ← forever ← forever ← forever ← … 995 more … ← main`.

| Error | Example | Message |
|---|---|---|
| Division by zero | `10 / 0` | `division by zero` (caret on the divisor) |
| Integer overflow | `2147483647 + 1` | `integer overflow: the result of `+` does not fit in `i32`` |
| Call depth exceeded | unbounded recursion | `stack overflow: more than 1000 nested function calls` |

More kinds exist that `dijk run` can never produce, because an earlier
stage rejects those programs first, with better diagnostics: `undefined
variable`, `undefined function`, `wrong number of arguments` and `no main
function` (semantic analysis), and `invalid operation` (an operator or a
condition given the wrong kind of value) and `integer literal … does not
fit` (the type checker). The interpreter still reports them, rather than
crashing, if it is ever handed a program nobody checked. The tests hand
it such programs on purpose.

The five classes of error form a ladder. Each stage detects a different
kind of problem, and `dijk run` stops at the first stage that finds one:

```text
error:              lexical   "What are the pieces?"        unexpected character
error:              parse     "How do they fit?"            expected ')'
semantic error:               "What do the names mean?"     unknown identifier
type error[E…]:               "Are the operations valid?"   type mismatch
runtime error:                "What happens?"               division by zero
```

## `dijk run`

```text
dijk run <file.dk | file.dijk>
```

`dijk run` runs a program through every stage in turn, and each stage
runs only if every stage before it succeeded:

```text
lex ──errors?──→ report, exit 1
 ↓
parse ──errors?──→ report, exit 1         (a partial AST is never analyzed)
 ↓
semantic analysis ──errors?──→ report, exit 1
 ↓
type checking ──errors?──→ report, exit 1       (nothing is executed)
 ↓
interpret ──runtime error?──→ report, exit 1
 ↓
print main's value, exit 0
```

- The program starts at `fn main()`, which takes no parameters and is
  called with none. It may return any type (`fn main() -> i32`), or
  nothing.
- On success, the value `main` returns is printed on its own line (`30`,
  `true`, or a string without quotes). **If `main` returns nothing
  (`Unit`), nothing is printed.** The exit code is 0. It is not the
  returned value.
- Any error, from any stage, goes to stderr with exit code 1.
- There is no `print` yet. A program's only output is `main`'s return
  value.

## A complete trace

```dijk
fn double(x: i32) -> i32 {
    return x * 2;
}

fn main() -> i32 {
    let x = 10;
    let y = double(x);

    return y + 5;
}
```

```text
call main()                            stack: [main]
  let x = 10                           main: x → 10
  let y = double(x)
    evaluate x in main's frame         → 10
    call double(10)                    stack: [main, double]
      bind x → 10                      double: x → 10   (a different x)
      return x * 2                     → Return(20)
    pop double's frame                 stack: [main]
                                       main: x → 10, y → 20
  return y + 5                         → Return(25)
pop main's frame                       stack: []

dijk run prints: 25
```

Both functions have a variable called `x`, and they never interfere,
because each lives in its own frame.
