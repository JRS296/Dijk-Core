//! Type errors: programs whose names all mean something, but whose
//! operations are not valid for the kinds of value they are applied to.

use super::ty::Type;
use crate::ast::expression::{BinaryOperator, UnaryOperator};
use crate::diagnostics;
use crate::lexer::token::Span;
use std::fmt;

/// One variant per rule the type checker enforces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeErrorKind {
    /// A type annotation names a type Dijk does not have, such as
    /// `let x: banana = 1;`.
    UnknownType { name: String },
    /// An expression's type is not the type required where it appears,
    /// such as `let x: i32 = "hello";`.
    Mismatch { expected: Type, found: Type },
    /// An assignment stores a value whose type is not the variable's,
    /// such as `x = "hello";` for an `i32` variable `x`.
    InvalidAssignment {
        name: String,
        expected: Type,
        found: Type,
    },
    /// An integer literal whose value does not fit in the type it was
    /// given, such as `3000000000` where an `i32` is expected. `value`
    /// includes a minus sign written directly before the digits.
    IntegerLiteralOutOfRange { value: i64, ty: Type },
    /// A call passes an argument whose type is not its parameter's, such
    /// as `add(10, "hello")` for `fn add(a: i32, b: i32)`. `position`
    /// counts from 1.
    InvalidArgument {
        function: String,
        parameter: String,
        position: usize,
        expected: Type,
        found: Type,
    },
    /// A `return` whose value does not match the function's return type.
    /// A bare `return;` is found to be `unit`, so it is this error in a
    /// function with `-> T`, and so is `return value;` in a function
    /// without one (which is expected to return `unit`).
    InvalidReturn {
        function: String,
        expected: Type,
        found: Type,
    },
    /// The condition of an `if` or `while` (`statement`) is not a `bool`,
    /// such as `if 10 { … }`.
    InvalidCondition {
        statement: &'static str,
        found: Type,
    },
    /// A function with a return type other than `unit` can reach the end
    /// of its body without returning a value.
    MissingReturn { function: String, expected: Type },
    /// A binary operator applied to operand types it has no rule for, such
    /// as `10 + "hello"` or `1 < true` (see `rules.rs`).
    InvalidBinaryOperation {
        operator: BinaryOperator,
        left: Type,
        right: Type,
    },
    /// A unary operator applied to an operand type it has no rule for,
    /// such as `-true` or `!1`.
    InvalidUnaryOperation {
        operator: UnaryOperator,
        operand: Type,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub span: Span,
}

impl TypeError {
    pub fn new(kind: TypeErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    fn message(&self) -> String {
        match &self.kind {
            TypeErrorKind::UnknownType { name } => format!("unknown type `{name}`"),
            TypeErrorKind::Mismatch { expected, found } => {
                format!("mismatched types: expected `{expected}`, found `{found}`")
            }
            TypeErrorKind::InvalidAssignment {
                name,
                expected,
                found,
            } => format!(
                "cannot assign a value of type `{found}` to `{name}`, which has type `{expected}`"
            ),
            TypeErrorKind::IntegerLiteralOutOfRange { value, ty } => {
                format!("integer literal `{value}` is out of range for `{ty}`")
            }
            TypeErrorKind::InvalidArgument {
                function,
                parameter,
                position,
                expected,
                found,
            } => format!(
                "argument {position} of `{function}` (`{parameter}`) has type `{found}`, but `{function}` expects `{expected}`"
            ),
            TypeErrorKind::InvalidReturn {
                function,
                expected,
                found,
            } => match (expected, found) {
                (_, Type::Unit) => {
                    format!("`{function}` must return `{expected}`, but this returns no value")
                }
                (Type::Unit, _) => format!(
                    "`{function}` has no return type, so it cannot return a value of type `{found}`"
                ),
                _ => format!(
                    "`{function}` must return `{expected}`, but this returns `{found}`"
                ),
            },
            TypeErrorKind::InvalidCondition { statement, found } => {
                format!("the condition of `{statement}` must be `bool`, found `{found}`")
            }
            TypeErrorKind::MissingReturn { function, expected } => format!(
                "`{function}` may reach its end without returning a value of type `{expected}`"
            ),
            TypeErrorKind::InvalidBinaryOperation {
                operator,
                left,
                right,
            } => format!("cannot apply `{operator}` to `{left}` and `{right}`"),
            TypeErrorKind::InvalidUnaryOperation { operator, operand } => {
                format!("cannot apply `{operator}` to `{operand}`")
            }
        }
    }

    /// Render this error as a source-anchored diagnostic labelled
    /// `type error:`, distinct from every other stage's label. See
    /// `crate::diagnostics::render`.
    pub fn render(&self, file: &str, source: &str) -> String {
        diagnostics::render(file, source, self.span, "type error", &self.message())
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
