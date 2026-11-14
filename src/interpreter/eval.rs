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

use super::environment::Environment;
use super::error::{RuntimeError, RuntimeErrorKind};
use super::value::Value;
use crate::ast::expression::{BinaryOperator, Expression, UnaryOperator};
use crate::lexer::token::Span;

/// The state of a running program: for now, one environment of
/// variables that every expression is evaluated against.
pub struct Interpreter {
    environment: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            environment: Environment::new(),
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
            // Calling a function needs functions to call, and a fresh
            // environment for each call. This interpreter has neither: it
            // evaluates everything against one environment and knows no
            // functions, so every callee is undefined.
            Expression::Call { callee, span, .. } => Err(RuntimeError::new(
                RuntimeErrorKind::UndefinedFunction {
                    name: callee.clone(),
                },
                *span,
            )),
        }
    }

    /// A variable evaluates to its current value. Semantic analysis
    /// guarantees the variable exists in any program it accepted. The
    /// error is here so that an unchecked program is still reported, not
    /// a crash.
    fn evaluate_identifier(&self, name: &str, span: Span) -> Result<Value, RuntimeError> {
        match self.environment.lookup(name) {
            Some(value) => Ok(value.clone()),
            None => Err(RuntimeError::new(
                RuntimeErrorKind::UndefinedVariable {
                    name: name.to_string(),
                },
                span,
            )),
        }
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
        if self.environment.assign(name, value.clone()) {
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
