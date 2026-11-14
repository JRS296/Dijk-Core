//! The type checker: a walk over the whole AST that works out the type
//! of every expression, without evaluating any of them.
//!
//! Like the interpreter, and unlike semantic analysis, this walk does not
//! use the AST's `Visitor`. A visitor only *looks* at nodes and returns
//! nothing, but checking an expression must *produce* its type. So each
//! kind of node is handled by an explicit `match`, and the central
//! operation is:
//!
//! ```text
//! check_expression(expression, expected) → Type
//! ```
//!
//! It is the static twin of the interpreter's `evaluate(expression) →
//! Value`. Both walk the same tree in the same shape; one computes what
//! kind of value an expression *would* produce, the other the value
//! itself.
//!
//! # The expected type
//!
//! The second argument is what the surrounding code expects the
//! expression's type to be, if it expects anything. Most expressions
//! ignore it: `true` is a `bool` whatever anyone expects. Integer
//! literals are the exception, because `42` could be an `i32` or an
//! `i64`. A literal takes its type from what is expected of it and, with
//! no expectation, defaults to `i32`:
//!
//! ```text
//! 42              expected nothing   → i32   (the default)
//! 42              expected i64       → i64
//! 42              expected bool      → i32   (a fallback: the mismatch
//!                                             is reported by whoever
//!                                             expected bool)
//! ```
//!
//! This is all the inference an integer literal gets: information flows
//! from the context *down* into the literal, once, and is never solved
//! for. There is no constraint solving and no Hindley–Milner.
//!
//! # Literal ranges
//!
//! Once a literal has a type, its value must fit in it. `3000000000` does
//! not fit in an `i32`, so it is a type error, found without running
//! anything. A minus sign written directly before a literal counts as
//! part of the literal, so `-2147483648`, the smallest `i32`, is accepted
//! even though `2147483648` on its own is not.
//!
//! # Operators
//!
//! An operator expression is typed from the inside out: first its
//! operands, then the operator's rule for those operand types
//! (`rules.rs`). The checker never computes anything:
//!
//! ```text
//! 10 + 20 * 3
//!
//! Binary(+)              check 10           → i32
//! ├── Integer(10)        check 20 * 3
//! └── Binary(*)              check 20       → i32
//!     ├── Integer(20)        check 3        → i32
//!     └── Integer(3)         i32 * i32      → i32
//!                        i32 + i32          → i32
//! ```
//!
//! The interpreter walks the same tree and gets `70`; the checker gets
//! `i32`, and would get it just the same for `10 + 20 / 0`.
//!
//! A literal operand takes its type from the *other* operand, so that
//! `x + 1` works whatever integer type `x` has. The left operand is
//! checked first and its type is what the right one is expected to have,
//! unless the left operand is a bare literal and the right one is not, in
//! which case the order is swapped (`1 + x`).

use super::error::{TypeError, TypeErrorKind};
use super::rules;
use super::ty::Type;
use super::TypedProgram;
use crate::ast::declaration::{Declaration, Function, Program};
use crate::ast::expression::{BinaryOperator, Expression, UnaryOperator};
use crate::ast::statement::Statement;
use crate::lexer::token::Span;

/// Check every function in `program`, returning what was learned about
/// it, or every type error found, in source order.
pub fn check_program(program: &Program) -> Result<TypedProgram, Vec<TypeError>> {
    let mut checker = Checker {
        typed: TypedProgram::default(),
        errors: Vec::new(),
    };

    for declaration in &program.declarations {
        let Declaration::Function(function) = declaration;
        checker.check_function(function);
    }

    if checker.errors.is_empty() {
        Ok(checker.typed)
    } else {
        let mut errors = checker.errors;
        errors.sort_by_key(|e| e.span.start);
        Err(errors)
    }
}

struct Checker {
    /// What has been learned so far: grows as the walk proceeds.
    typed: TypedProgram,
    errors: Vec<TypeError>,
}

impl Checker {
    fn error(&mut self, kind: TypeErrorKind, span: Span) {
        self.errors.push(TypeError::new(kind, span));
    }

    // --- functions and statements ---

    fn check_function(&mut self, function: &Function) {
        self.check_statements(&function.body);
    }

    fn check_statements(&mut self, statements: &[Statement]) {
        for stmt in statements {
            self.check_statement(stmt);
        }
    }

    /// Statements have no type of their own. Checking one means checking
    /// the expressions it contains, and the statements nested in it.
    fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let { value, .. } => {
                self.check_expression(value, None);
            }
            Statement::Return { value, .. } => {
                if let Some(value) = value {
                    self.check_expression(value, None);
                }
            }
            Statement::Expression { expr, .. } => {
                self.check_expression(expr, None);
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.check_expression(condition, None);
                self.check_statements(then_branch);
                if let Some(else_branch) = else_branch {
                    self.check_statements(else_branch);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.check_expression(condition, None);
                self.check_statements(body);
            }
        }
    }

    // --- expressions ---

    /// Work out the type of `expr`, given the type the surrounding code
    /// `expected` of it (if any), and record it in the side table.
    ///
    /// The result is `Type::Error` for an expression that could not be
    /// typed. Every such expression either has had its error reported
    /// already, or contains one that has, so callers never report it
    /// again.
    fn check_expression(&mut self, expr: &Expression, expected: Option<Type>) -> Type {
        let ty = match expr {
            Expression::Integer(value, _) => self.check_integer_literal(expr, *value, expected),
            Expression::String(..) => Type::String,
            Expression::Boolean(..) => Type::Bool,
            Expression::Unary {
                operator, operand, ..
            } => self.check_unary(expr, *operator, operand, expected),
            // Typing variables, assignments and calls needs to know what
            // names stand for, which this checker does not know yet. Until
            // it does, such an expression is left untyped.
            Expression::Binary {
                left,
                operator,
                right,
                ..
            } => self.check_binary(expr, left, *operator, right, expected),
            Expression::Identifier(..) | Expression::Assign { .. } | Expression::Call { .. } => {
                Type::Error
            }
        };
        self.record(expr.span(), ty);
        ty
    }

    /// Record `ty` as the type of the expression at `span`. `Error` is
    /// never recorded: it is not a Dijk type, only the checker's marker
    /// for "already reported".
    fn record(&mut self, span: Span, ty: Type) {
        if ty != Type::Error {
            self.typed.expression_types.insert(span, ty);
        }
    }

    /// An integer literal (`42`, or `-42`) takes the integer type its
    /// context expects, or `i32` if the context expects no integer type,
    /// and its `value` must fit in that type.
    ///
    /// When the context expects something that is not an integer at all
    /// (`let b: bool = 3000000000;`), `i32` is only a fallback: the
    /// mistake is using an integer there, which whoever expected the
    /// `bool` reports. The literal's range is then not checked, so that
    /// one mistake produces one diagnostic.
    ///
    /// For `-42` both nodes, the `Unary(-)` and the `Integer(42)` inside
    /// it, get the literal's type: the minus sign is part of the literal.
    fn check_integer_literal(
        &mut self,
        expr: &Expression,
        value: i64,
        expected: Option<Type>,
    ) -> Type {
        let (ty, check_range) = match expected {
            Some(expected) if expected.is_integer() => (expected, true),
            // No context, or one that could not be typed: the default.
            None | Some(Type::Error) => (Type::I32, true),
            // Something other than an integer is wanted here.
            Some(_) => (Type::I32, false),
        };

        if check_range && !fits(value, ty) {
            self.error(
                TypeErrorKind::IntegerLiteralOutOfRange { value, ty },
                expr.span(),
            );
        }

        if let Expression::Unary { operand, .. } = expr {
            self.record(operand.span(), ty);
        }
        ty
    }

    /// A unary expression. A minus sign written directly before digits is
    /// not an operation at all: `-42` is a negative integer literal.
    /// Otherwise the operand is checked, then the operator's rule for its
    /// type is looked up.
    fn check_unary(
        &mut self,
        expr: &Expression,
        operator: UnaryOperator,
        operand: &Expression,
        expected: Option<Type>,
    ) -> Type {
        if let Some(value) = literal_value(expr) {
            return self.check_integer_literal(expr, value, expected);
        }

        // `-x` has the type of `x`, so what is expected of the result is
        // what is expected of the operand. `!x` is a `bool` whatever is
        // expected of it.
        let operand_expected = match operator {
            UnaryOperator::Neg => expected,
            UnaryOperator::Not => None,
        };
        let operand_ty = self.check_expression(operand, operand_expected);
        if operand_ty == Type::Error {
            return Type::Error;
        }

        match rules::unary_result(operator, operand_ty) {
            Some(result) => result,
            None => {
                self.error(
                    TypeErrorKind::InvalidUnaryOperation {
                        operator,
                        operand: operand_ty,
                    },
                    expr.span(),
                );
                Type::Error
            }
        }
    }

    /// A binary expression: check both operands, then look up the
    /// operator's rule for their types.
    fn check_binary(
        &mut self,
        expr: &Expression,
        left: &Expression,
        operator: BinaryOperator,
        right: &Expression,
        expected: Option<Type>,
    ) -> Type {
        // The result of `+ - * /` has the type of its operands, so what is
        // expected of the result is what is expected of them. A comparison
        // is a `bool` whatever its operands are, so nothing is passed down.
        let operand_expected = if rules::is_arithmetic(operator) {
            expected
        } else {
            None
        };

        // Check the operand that can decide the type first, and expect the
        // other one to have the same type. That is the left one, unless it
        // is a bare literal and the right one is not (`1 + x`).
        let (left_ty, right_ty) = if literal_value(left).is_some() && literal_value(right).is_none()
        {
            let right_ty = self.check_expression(right, operand_expected);
            let left_ty = self.check_expression(left, Some(right_ty));
            (left_ty, right_ty)
        } else {
            let left_ty = self.check_expression(left, operand_expected);
            let right_ty = self.check_expression(right, Some(left_ty));
            (left_ty, right_ty)
        };

        // An operand that could not be typed has already been reported.
        if left_ty == Type::Error || right_ty == Type::Error {
            return Type::Error;
        }

        match rules::binary_result(operator, left_ty, right_ty) {
            Some(result) => result,
            None => {
                self.error(
                    TypeErrorKind::InvalidBinaryOperation {
                        operator,
                        left: left_ty,
                        right: right_ty,
                    },
                    expr.span(),
                );
                Type::Error
            }
        }
    }
}

/// The value of `expr` if it is an integer literal: digits (`42`), or a
/// minus sign written directly before digits (`-42`). Anything else,
/// including `-(-42)` or `0 - 42`, is an operation on literals, not a
/// literal.
///
/// The minus sign has to count as part of the literal. Otherwise
/// `-2147483648`, the smallest `i32`, could not be written: the parser
/// reads it as `-` applied to `2147483648`, which on its own is one more
/// than the largest `i32`.
fn literal_value(expr: &Expression) -> Option<i64> {
    match expr {
        Expression::Integer(value, _) => Some(*value),
        Expression::Unary {
            operator: UnaryOperator::Neg,
            operand,
            ..
        } => match operand.as_ref() {
            // The parser only produces non-negative literals, so the
            // negation always fits in an `i64`.
            Expression::Integer(value, _) => Some(-*value),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `value` can be represented in the integer type `ty`.
fn fits(value: i64, ty: Type) -> bool {
    match ty {
        Type::I32 => i32::try_from(value).is_ok(),
        // The parser has already rejected any literal beyond `i64`.
        _ => true,
    }
}
