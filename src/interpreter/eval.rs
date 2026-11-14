//! Evaluation: turning AST nodes into runtime values.
//!
//! Evaluation is a direct recursive walk. To evaluate a node, evaluate
//! its children, then combine their values:
//!
//! ```text
//! 10 + 20 * 3
//!
//! Binary(+)                  evaluate(Integer(10))       → 10
//! ├── Integer(10)            evaluate(Binary(*))
//! └── Binary(*)                  evaluate(Integer(20))   → 20
//!     ├── Integer(20)            evaluate(Integer(3))    → 3
//!     └── Integer(3)             20 * 3                  → 60
//!                            10 + 60                     → 70
//! ```
//!
//! The shape of the tree *is* the order of the computation: the parser
//! already put `*` below `+`, so nothing here needs to know about
//! precedence. This is the payoff of building the AST.
//!
//! This walk does not use the AST's `Visitor` (`crate::ast::visit`). A
//! visitor only *looks* at nodes and returns nothing, but evaluating a
//! node must produce a value and must be able to fail. So each kind of
//! node is handled by an explicit `match` that returns
//! `Result<Value, RuntimeError>`.
//!
//! Statements are *executed* rather than evaluated. They are run for
//! their effect on the environment, not for a value. Control flow
//! statements decide which statements run, and how many times:
//!
//! ```text
//! if condition { then } else { otherwise }
//!     evaluate condition → true  → execute then
//!                        → false → execute otherwise (if present)
//!
//! while condition { body }
//!     evaluate condition → true  → execute body, then evaluate again
//!                        → false → done
//! ```
//!
//! Each branch and loop body runs in a block scope of its own, pushed on
//! entry and popped on exit, matching the scopes semantic analysis
//! checked the program against. A `while` body gets a fresh scope on
//! every iteration, so a `let` inside the loop declares a new variable
//! each time around.
//!
//! # Function calls
//!
//! Every call gets a **call frame**: a fresh environment of its own,
//! holding the callee's parameters and everything its body declares. The
//! frames of all calls in progress form the **call stack**:
//!
//! ```text
//! fn add(a: i32, b: i32) -> i32 { return a + b; }
//! fn main() { let x = add(10, 20); return x; }
//!
//! Call stack, while `add` runs:
//! ┌─────────────────────┐
//! │ add   a → 10        │  ← top: the running call
//! │       b → 20        │
//! ├─────────────────────┤
//! │ main  (x not yet    │  ← waiting for `add` to finish
//! │        declared)    │
//! └─────────────────────┘
//! ```
//!
//! A call evaluates its arguments in the *caller's* frame, left to right.
//! It then pushes a new frame that binds each parameter to its argument,
//! runs the body in it, and pops the frame. A body can only ever see its
//! own frame, which is why a function cannot touch its caller's
//! variables. Recursion needs nothing special: each recursive call simply
//! pushes another frame, with its own copy of every parameter.
//!
//! Each Dijk call also nests a few Rust calls (`call_function` →
//! `execute_statement` → `evaluate` → …), so Dijk's call stack lives on
//! top of Rust's own, finite, stack. Unbounded recursion in a Dijk program
//! would eventually exhaust it and crash the interpreter itself. The
//! call stack is therefore capped at [`MAX_CALL_DEPTH`] frames, and going
//! deeper is a runtime error, the interpreter's version of a "stack
//! overflow". (The interpreter runs on a thread whose Rust stack is sized
//! to hold that many calls with room to spare; see `STACK_SIZE` in
//! `mod.rs`.)
//!
//! # Return propagation
//!
//! `return` may sit several blocks deep inside a function, but it ends
//! the whole *call*, not just its own block. Every statement between
//! the `return` and the end of the function must be skipped:
//!
//! ```text
//! fn find() -> i32 {
//!     while true {
//!         if found() {
//!             return 10;     → Return(10)
//!             log();         skipped: the block stops at a Return
//!         }                  if:    passes Return(10) outward
//!         step();            skipped
//!     }                      while: stops looping, passes it outward
//!     cleanup();             skipped
//! }                          call:  Return(10) becomes the call's value
//! ```
//!
//! So executing a statement reports *how it finished*, as an
//! [`Execution`]: normally (carry on with the next statement), or by
//! returning a value (stop, and tell the enclosing statement to stop
//! too). Together with `Err(RuntimeError)` for a failure, every
//! statement ends in exactly one of three ways:
//!
//! ```text
//! Ok(Execution::Normal)          next statement, please
//! Ok(Execution::Return(value))   unwind to the function call
//! Err(RuntimeError)              unwind all the way out and report
//! ```
//!
//! Blocks and loops pass a `Return` outward untouched. Only a function
//! call consumes it, turning it into the value of the call expression.
//!
//! # Integers
//!
//! `42` could be an `i32` or an `i64`: the type checker decided which,
//! from context, and recorded it in the [`TypedProgram`]. Evaluating a
//! literal looks that decision up and builds a value of that type. This
//! is the only place the interpreter needs the checker's results. Every
//! other integer is the result of an operation, and so has the type of
//! that operation's operands.
//!
//! Integer arithmetic is defined by Dijk, not inherited from whatever
//! Rust happens to do:
//!
//! ```text
//! 1. compute the true, mathematical result (exactly, in a wider type)
//! 2. if it fits in the operands' type, that is the result
//! 3. otherwise it is a runtime error: integer overflow
//! ```
//!
//! So `2147483647 + 1` is an overflow for `i32` operands but fine for
//! `i64` ones, and nothing ever silently wraps around. Division truncates
//! toward zero (`-7 / 2` is `-3`), dividing by zero is a runtime error,
//! and the smallest integer divided by `-1` is an overflow, since the
//! true result is one more than the largest.

use super::environment::Environment;
use super::error::{RuntimeError, RuntimeErrorKind};
use super::value::Value;
use crate::ast::declaration::{Declaration, Function, Program};
use crate::ast::expression::{BinaryOperator, Expression, UnaryOperator};
use crate::ast::statement::Statement;
use crate::lexer::token::Span;
use crate::types::ty::Type;
use crate::types::{self, TypedProgram};
use std::collections::HashMap;

/// The most calls that may be in progress at once, `main` included.
pub const MAX_CALL_DEPTH: usize = 1000;

/// How a statement finished executing (when it did not fail).
enum Execution {
    /// It ran to completion: continue with the next statement.
    Normal,
    /// A `return` ran: skip everything else up to the end of the
    /// function call, which evaluates to this value.
    Return(Value),
}

/// One call in progress.
struct CallFrame<'p> {
    /// The function this frame is running, as a call stack lists it.
    function: &'p str,
    /// This call's variables: its parameters, then whatever its body
    /// declares.
    environment: Environment,
}

/// The state of a running program.
pub struct Interpreter<'p> {
    /// The global scope: every function in the program, by name.
    functions: HashMap<&'p str, &'p Function>,
    /// What the type checker recorded about the program: here, the type
    /// it gave each integer literal.
    types: &'p TypedProgram,
    /// Every call in progress, outermost (`main`) first. The top frame
    /// is the running call, and its environment is the one statements
    /// and expressions read and write.
    call_stack: Vec<CallFrame<'p>>,
}

impl<'p> Interpreter<'p> {
    /// Prepare to run `program`, which the type checker described in
    /// `types`: enter every function into the global scope. If two
    /// functions share a name the first one wins, matching what semantic
    /// analysis resolves calls to (and reports as an error).
    pub fn new(program: &'p Program, types: &'p TypedProgram) -> Self {
        let mut functions = HashMap::new();
        for declaration in &program.declarations {
            let Declaration::Function(function) = declaration;
            functions.entry(function.name.as_str()).or_insert(function);
        }

        Self {
            functions,
            types,
            call_stack: Vec::new(),
        }
    }

    /// Run the program: call `main` with no arguments and return the
    /// value it returns.
    pub fn run_main(&mut self) -> Result<Value, RuntimeError> {
        match self.functions.get("main").copied() {
            Some(main) => self.call_function(main, Vec::new(), main.span),
            None => Err(RuntimeError::new(
                RuntimeErrorKind::MissingMain,
                Span::new(0, 0),
            )),
        }
    }

    /// Call `function` with already-evaluated `arguments`: push a frame
    /// binding each parameter to its argument, run the body in it, pop
    /// the frame. `call_span` is where the call was written, so that a
    /// call that cannot be made is reported there.
    fn call_function(
        &mut self,
        function: &'p Function,
        arguments: Vec<Value>,
        call_span: Span,
    ) -> Result<Value, RuntimeError> {
        if self.call_stack.len() == MAX_CALL_DEPTH {
            return Err(RuntimeError::new(
                RuntimeErrorKind::CallDepthExceeded {
                    limit: MAX_CALL_DEPTH,
                },
                call_span,
            ));
        }

        // Semantic analysis has already checked every call's argument
        // count; this is the interpreter's own guard against a program
        // that skipped it.
        if arguments.len() != function.params.len() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::ArityMismatch {
                    name: function.name.clone(),
                    expected: function.params.len(),
                    found: arguments.len(),
                },
                call_span,
            ));
        }

        let mut environment = Environment::new();
        for (param, argument) in function.params.iter().zip(arguments) {
            environment.define(&param.name, argument);
        }

        self.call_stack.push(CallFrame {
            function: &function.name,
            environment,
        });
        let result = self.execute_body(&function.body);
        let frame = self.call_stack.pop();

        // An error leaving this call passes through its frame on the way
        // out. Recording each frame it passes through rebuilds the call
        // stack as it was at the moment of the error, innermost first.
        result.map_err(|mut error| {
            if let Some(frame) = frame {
                error.call_stack.push(frame.function.to_string());
            }
            error
        })
    }

    /// Run a function body, directly in the function scope (the
    /// parameters' scope), and produce the call's value.
    ///
    /// This is where a `Return` stops unwinding: however deeply it was
    /// nested, its value becomes the value of the call. A body that runs
    /// out of statements without returning produces `Unit`.
    fn execute_body(&mut self, body: &[Statement]) -> Result<Value, RuntimeError> {
        match self.execute_statements(body)? {
            Execution::Return(value) => Ok(value),
            Execution::Normal => Ok(Value::Unit),
        }
    }

    // --- the running call's environment ---
    //
    // Statements and expressions only ever run inside `call_function`,
    // so a frame is always on the stack while they do. These helpers act
    // on the top frame's environment. If the stack were somehow empty,
    // they would behave as if no variable existed.

    fn lookup(&self, name: &str) -> Option<&Value> {
        self.call_stack.last()?.environment.lookup(name)
    }

    fn define(&mut self, name: &str, value: Value) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.environment.define(name, value);
        }
    }

    fn assign(&mut self, name: &str, value: Value) -> bool {
        match self.call_stack.last_mut() {
            Some(frame) => frame.environment.assign(name, value),
            None => false,
        }
    }

    fn push_scope(&mut self) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.environment.push_scope();
        }
    }

    fn pop_scope(&mut self) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.environment.pop_scope();
        }
    }

    // --- statements ---

    /// Execute `statements` in a new block scope. The scope is popped
    /// however the statements finish: normally, by returning, or by
    /// failing.
    fn execute_block(&mut self, statements: &[Statement]) -> Result<Execution, RuntimeError> {
        self.push_scope();
        let result = self.execute_statements(statements);
        self.pop_scope();
        result
    }

    /// Execute `statements` in order, in the current scope. The first
    /// statement that returns (or fails) ends the sequence: the
    /// statements after it never run.
    fn execute_statements(&mut self, statements: &[Statement]) -> Result<Execution, RuntimeError> {
        for stmt in statements {
            match self.execute_statement(stmt)? {
                Execution::Normal => {}
                Execution::Return(value) => return Ok(Execution::Return(value)),
            }
        }
        Ok(Execution::Normal)
    }

    fn execute_statement(&mut self, stmt: &Statement) -> Result<Execution, RuntimeError> {
        match stmt {
            Statement::Let { name, value, .. } => {
                let value = self.evaluate(value)?;
                self.define(name, value);
                Ok(Execution::Normal)
            }
            // An expression statement is evaluated for its effect (an
            // assignment, say); its value is discarded.
            Statement::Expression { expr, .. } => {
                self.evaluate(expr)?;
                Ok(Execution::Normal)
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                if self.evaluate_condition(condition)? {
                    self.execute_block(then_branch)
                } else if let Some(else_branch) = else_branch {
                    // An `else if` is an else branch holding one `If`, so
                    // chains need no special handling here.
                    self.execute_block(else_branch)
                } else {
                    Ok(Execution::Normal)
                }
            }
            // A `return` inside the body ends the loop as well as the
            // iteration: it is passed outward instead of looping again.
            Statement::While {
                condition, body, ..
            } => {
                while self.evaluate_condition(condition)? {
                    match self.execute_block(body)? {
                        Execution::Normal => {}
                        Execution::Return(value) => return Ok(Execution::Return(value)),
                    }
                }
                Ok(Execution::Normal)
            }
            // `return;` with no value returns `Unit`. (The type checker has
            // made sure that only happens in a function that returns
            // `unit`.)
            Statement::Return { value, .. } => {
                let value = match value {
                    Some(value) => self.evaluate(value)?,
                    None => Value::Unit,
                };
                Ok(Execution::Return(value))
            }
        }
    }

    /// Evaluate the condition of an `if` or `while`. Only a boolean can
    /// decide which way to go: there is no "truthiness", so `if 5` is an
    /// invalid operation, not "true because it's non-zero".
    fn evaluate_condition(&mut self, condition: &Expression) -> Result<bool, RuntimeError> {
        match self.evaluate(condition)? {
            Value::Boolean(value) => Ok(value),
            other => Err(RuntimeError::new(
                RuntimeErrorKind::InvalidCondition {
                    found: other.kind_name(),
                },
                condition.span(),
            )),
        }
    }

    /// Evaluate `expr` to a value.
    ///
    /// Operands are evaluated left to right, and both operands of a
    /// binary operator are always evaluated (Dijk has no short-circuiting
    /// `&&`/`||` yet). The first error stops evaluation, and the error is
    /// returned with the span of the expression that failed.
    pub fn evaluate(&mut self, expr: &Expression) -> Result<Value, RuntimeError> {
        match expr {
            Expression::Integer(value, _) => {
                self.evaluate_integer_literal(expr, i128::from(*value))
            }
            Expression::String(value, _) => Ok(Value::String(value.clone())),
            Expression::Boolean(value, _) => Ok(Value::Boolean(*value)),
            Expression::Identifier(name, span) => self.evaluate_identifier(name, *span),
            Expression::Unary {
                operator,
                operand,
                span,
            } => {
                // `-42` is a single, negative literal, not `-` applied to
                // `42` (see `types::literal_value`).
                if let Some(value) = types::literal_value(expr) {
                    return self.evaluate_integer_literal(expr, value);
                }
                let operand = self.evaluate(operand)?;
                apply_unary(*operator, operand, *span)
            }
            Expression::Binary {
                left,
                operator,
                right,
                span,
            } => {
                let left_value = self.evaluate(left)?;
                let right_value = self.evaluate(right)?;
                apply_binary(*operator, left_value, right_value, *span, right.span())
            }
            Expression::Assign { name, value, span } => {
                self.evaluate_assignment(name, value, *span)
            }
            Expression::Call {
                callee,
                arguments,
                span,
            } => self.evaluate_call(callee, arguments, *span),
        }
    }

    /// An integer literal becomes a value of the type the checker gave
    /// it. A literal the checker never saw (a program that skipped it)
    /// gets the type the checker gives a literal with no context, `i32`.
    fn evaluate_integer_literal(
        &self,
        literal: &Expression,
        value: i128,
    ) -> Result<Value, RuntimeError> {
        let ty = self.types.type_of(literal).unwrap_or(Type::I32);
        integer_value(value, ty).ok_or_else(|| {
            RuntimeError::new(
                RuntimeErrorKind::IntegerLiteralOutOfRange {
                    value,
                    ty: integer_type_name(ty),
                },
                literal.span(),
            )
        })
    }

    /// A variable evaluates to its current value. Semantic analysis
    /// guarantees the variable exists in any program it accepted. The
    /// error is here so that an unchecked program is still reported, not
    /// a crash.
    fn evaluate_identifier(&self, name: &str, span: Span) -> Result<Value, RuntimeError> {
        match self.lookup(name) {
            Some(value) => Ok(value.clone()),
            None => Err(RuntimeError::new(
                RuntimeErrorKind::UndefinedVariable {
                    name: name.to_string(),
                },
                span,
            )),
        }
    }

    /// A call: find the function in the global scope, evaluate the
    /// arguments in the caller's frame, left to right, then call it.
    fn evaluate_call(
        &mut self,
        callee: &str,
        arguments: &[Expression],
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let Some(function) = self.functions.get(callee).copied() else {
            return Err(RuntimeError::new(
                RuntimeErrorKind::UndefinedFunction {
                    name: callee.to_string(),
                },
                span,
            ));
        };

        let mut values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            values.push(self.evaluate(argument)?);
        }

        self.call_function(function, values, span)
    }

    /// `name = value` stores the value in the existing variable and, as
    /// an expression, evaluates to that same value. That is what makes
    /// the right-associative chain `x = y = 1` assign `1` to both.
    fn evaluate_assignment(
        &mut self,
        name: &str,
        value: &Expression,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let value = self.evaluate(value)?;
        if self.assign(name, value.clone()) {
            Ok(value)
        } else {
            Err(RuntimeError::new(
                RuntimeErrorKind::UndefinedVariable {
                    name: name.to_string(),
                },
                span,
            ))
        }
    }
}

/// Apply a unary operator: `-` negates an integer (checked: negating
/// the smallest integer overflows), `!` negates a boolean. Anything else
/// (`-true`, `!1`) is an invalid operation.
fn apply_unary(operator: UnaryOperator, operand: Value, span: Span) -> Result<Value, RuntimeError> {
    match (operator, operand) {
        (UnaryOperator::Neg, Value::I32(n)) => checked(-i128::from(n), Type::I32, "-", span),
        (UnaryOperator::Neg, Value::I64(n)) => checked(-i128::from(n), Type::I64, "-", span),
        (UnaryOperator::Not, Value::Boolean(b)) => Ok(Value::Boolean(!b)),
        (operator, operand) => Err(RuntimeError::new(
            RuntimeErrorKind::InvalidUnaryOperation {
                operator: operator.to_string(),
                operand: operand.kind_name(),
            },
            span,
        )),
    }
}

/// Apply a binary operator to two already-evaluated operands.
///
/// The type checker has made sure the operands fit the operator (see
/// `types::rules`), so in a checked program only the first three cases
/// ever happen:
///
/// ```text
/// i32 (op) i32, i64 (op) i64  → integer arithmetic or comparison
/// string + string             → string (concatenation)
/// string/bool == != same      → bool
/// anything else               → invalid operation
/// ```
///
/// The last case is the interpreter's own guard against a program that
/// skipped type checking, which it reports rather than crash on.
/// `divisor_span` points a division-by-zero error at the divisor itself.
/// Every other error points at the whole expression.
fn apply_binary(
    operator: BinaryOperator,
    left: Value,
    right: Value,
    span: Span,
    divisor_span: Span,
) -> Result<Value, RuntimeError> {
    use BinaryOperator::*;

    match (operator, left, right) {
        (_, Value::I32(a), Value::I32(b)) => {
            integer_binary(operator, a.into(), b.into(), Type::I32, span, divisor_span)
        }
        (_, Value::I64(a), Value::I64(b)) => {
            integer_binary(operator, a.into(), b.into(), Type::I64, span, divisor_span)
        }
        (Add, Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),
        (Eq, Value::String(a), Value::String(b)) => Ok(Value::Boolean(a == b)),
        (NotEq, Value::String(a), Value::String(b)) => Ok(Value::Boolean(a != b)),
        (Eq, Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a == b)),
        (NotEq, Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a != b)),
        (operator, left, right) => Err(invalid_binary(operator, &left, &right, span)),
    }
}

/// An integer operation on two operands of the integer type `ty`, given
/// as exact `i128`s. Arithmetic computes the true result, then checks it
/// fits in `ty` (see the module docs on integers). An `i128` holds the
/// true result of any of these operations on two `i64`s.
fn integer_binary(
    operator: BinaryOperator,
    a: i128,
    b: i128,
    ty: Type,
    span: Span,
    divisor_span: Span,
) -> Result<Value, RuntimeError> {
    use BinaryOperator::*;

    let exact = match operator {
        Add => a + b,
        Sub => a - b,
        Mul => a * b,
        Div if b == 0 => {
            return Err(RuntimeError::new(
                RuntimeErrorKind::DivisionByZero,
                divisor_span,
            ))
        }
        // Integer division truncates toward zero: `-7 / 2` is `-3`.
        Div => a / b,
        Lt => return Ok(Value::Boolean(a < b)),
        Le => return Ok(Value::Boolean(a <= b)),
        Gt => return Ok(Value::Boolean(a > b)),
        Ge => return Ok(Value::Boolean(a >= b)),
        Eq => return Ok(Value::Boolean(a == b)),
        NotEq => return Ok(Value::Boolean(a != b)),
    };
    checked(exact, ty, &operator.to_string(), span)
}

/// The true result `exact` of an integer operation, as a value of type
/// `ty`, or an overflow error if it does not fit. Integer arithmetic
/// never silently wraps around and never crashes.
fn checked(exact: i128, ty: Type, operator: &str, span: Span) -> Result<Value, RuntimeError> {
    integer_value(exact, ty).ok_or_else(|| {
        RuntimeError::new(
            RuntimeErrorKind::IntegerOverflow {
                operator: operator.to_string(),
                ty: integer_type_name(ty),
            },
            span,
        )
    })
}

/// `value` as a value of the integer type `ty`, if it fits in it.
fn integer_value(value: i128, ty: Type) -> Option<Value> {
    match ty {
        Type::I64 => i64::try_from(value).ok().map(Value::I64),
        // Every integer that is not an `i64` is an `i32`.
        _ => i32::try_from(value).ok().map(Value::I32),
    }
}

/// How runtime errors name the integer type `ty`.
fn integer_type_name(ty: Type) -> &'static str {
    match ty {
        Type::I64 => "i64",
        _ => "i32",
    }
}

fn invalid_binary(
    operator: BinaryOperator,
    left: &Value,
    right: &Value,
    span: Span,
) -> RuntimeError {
    RuntimeError::new(
        RuntimeErrorKind::InvalidBinaryOperation {
            operator: operator.to_string(),
            left: left.kind_name(),
            right: right.kind_name(),
        },
        span,
    )
}
