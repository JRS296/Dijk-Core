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

use super::environment::Environment;
use super::error::{RuntimeError, RuntimeErrorKind};
use super::value::Value;
use crate::ast::declaration::{Declaration, Function, Program};
use crate::ast::expression::{BinaryOperator, Expression, UnaryOperator};
use crate::ast::statement::Statement;
use crate::lexer::token::Span;
use std::collections::HashMap;

/// The most calls that may be in progress at once, `main` included.
pub const MAX_CALL_DEPTH: usize = 1000;

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
    /// Every call in progress, outermost (`main`) first. The top frame
    /// is the running call, and its environment is the one statements
    /// and expressions read and write.
    call_stack: Vec<CallFrame<'p>>,
}

impl<'p> Interpreter<'p> {
    /// Prepare to run `program`: enter every function into the global
    /// scope. If two functions share a name the first one wins, matching
    /// what semantic analysis resolves calls to (and reports as an error).
    pub fn new(program: &'p Program) -> Self {
        let mut functions = HashMap::new();
        for declaration in &program.declarations {
            let Declaration::Function(function) = declaration;
            functions.entry(function.name.as_str()).or_insert(function);
        }

        Self {
            functions,
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
    /// A `return` written directly in the body ends the call with its
    /// value. A body that runs out of statements returns `Unit`. A
    /// `return` nested inside an `if` or `while` cannot be seen from
    /// here: it runs as an ordinary statement (see `execute_statement`),
    /// and does not yet end the call.
    fn execute_body(&mut self, body: &[Statement]) -> Result<Value, RuntimeError> {
        for stmt in body {
            if let Statement::Return { value, .. } = stmt {
                return match value {
                    Some(value) => self.evaluate(value),
                    None => Ok(Value::Unit),
                };
            }
            self.execute_statement(stmt)?;
        }
        Ok(Value::Unit)
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
    /// whether the statements finish normally or fail.
    fn execute_block(&mut self, statements: &[Statement]) -> Result<(), RuntimeError> {
        self.push_scope();
        let result = self.execute_statements(statements);
        self.pop_scope();
        result
    }

    /// Execute `statements` in order, in the current scope, stopping at
    /// the first error.
    fn execute_statements(&mut self, statements: &[Statement]) -> Result<(), RuntimeError> {
        for stmt in statements {
            self.execute_statement(stmt)?;
        }
        Ok(())
    }

    fn execute_statement(&mut self, stmt: &Statement) -> Result<(), RuntimeError> {
        match stmt {
            Statement::Let { name, value, .. } => {
                let value = self.evaluate(value)?;
                self.define(name, value);
                Ok(())
            }
            // An expression statement is evaluated for its effect (an
            // assignment, say); its value is discarded.
            Statement::Expression { expr, .. } => {
                self.evaluate(expr)?;
                Ok(())
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
                    Ok(())
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                while self.evaluate_condition(condition)? {
                    self.execute_block(body)?;
                }
                Ok(())
            }
            // A `return` directly in a function body never reaches this
            // point: `execute_body` handles it. One nested in a block has
            // no way yet to stop the statements around it from running,
            // so its value is evaluated (which may fail) and execution
            // carries on with the next statement.
            Statement::Return { value, .. } => {
                if let Some(value) = value {
                    self.evaluate(value)?;
                }
                Ok(())
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
            Expression::Integer(value, _) => Ok(Value::Integer(*value)),
            Expression::String(value, _) => Ok(Value::String(value.clone())),
            Expression::Boolean(value, _) => Ok(Value::Boolean(*value)),
            Expression::Identifier(name, span) => self.evaluate_identifier(name, *span),
            Expression::Unary {
                operator,
                operand,
                span,
            } => {
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

/// Apply a unary operator: `-` negates an integer, `!` negates a
/// boolean. Anything else (`-true`, `!1`) is an invalid operation.
fn apply_unary(operator: UnaryOperator, operand: Value, span: Span) -> Result<Value, RuntimeError> {
    match (operator, operand) {
        (UnaryOperator::Neg, Value::Integer(n)) => checked(n.checked_neg(), "-", span),
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
/// Without a type checker, the interpreter is the only place that
/// notices an operator applied to the wrong kinds of value. The rules:
///
/// ```text
/// integer  + - * /  integer   → integer (checked: overflow is an error)
/// string   +        string    → string  (concatenation)
/// integer  < <= > >= integer  → boolean
/// a        == !=    a         → boolean (both operands the same kind)
/// anything else               → invalid operation
/// ```
///
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
        (Add, Value::Integer(a), Value::Integer(b)) => checked(a.checked_add(b), "+", span),
        (Add, Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),
        (Sub, Value::Integer(a), Value::Integer(b)) => checked(a.checked_sub(b), "-", span),
        (Mul, Value::Integer(a), Value::Integer(b)) => checked(a.checked_mul(b), "*", span),
        (Div, Value::Integer(_), Value::Integer(0)) => Err(RuntimeError::new(
            RuntimeErrorKind::DivisionByZero,
            divisor_span,
        )),
        // Integer division truncates toward zero: `-7 / 2` is `-3`. The
        // one overflowing case is the most negative integer divided by -1.
        (Div, Value::Integer(a), Value::Integer(b)) => checked(a.checked_div(b), "/", span),
        (Lt, Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a < b)),
        (Le, Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
        (Gt, Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a > b)),
        (Ge, Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a >= b)),
        // Equality is only defined between values of the same kind:
        // `1 == true` is an invalid operation, not `false`.
        (Eq, a, b) if a.kind_name() == b.kind_name() => Ok(Value::Boolean(a == b)),
        (NotEq, a, b) if a.kind_name() == b.kind_name() => Ok(Value::Boolean(a != b)),
        (operator, left, right) => Err(RuntimeError::new(
            RuntimeErrorKind::InvalidBinaryOperation {
                operator: operator.to_string(),
                left: left.kind_name(),
                right: right.kind_name(),
            },
            span,
        )),
    }
}

/// Turn the result of a checked integer operation into a value, or into
/// an overflow error if the true result does not fit in 64 bits. Integer
/// arithmetic never silently wraps around and never crashes.
fn checked(result: Option<i64>, operator: &str, span: Span) -> Result<Value, RuntimeError> {
    match result {
        Some(n) => Ok(Value::Integer(n)),
        None => Err(RuntimeError::new(
            RuntimeErrorKind::IntegerOverflow {
                operator: operator.to_string(),
            },
            span,
        )),
    }
}
