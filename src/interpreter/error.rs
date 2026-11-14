//! Runtime errors: problems that only show up while a program runs.
//!
//! Semantic analysis rejects programs that cannot mean anything, before
//! they run. Some problems, though, depend on the *values* a program
//! computes, such as a division whose divisor turns out to be zero. No
//! earlier stage can see those. They are runtime errors, and they are
//! reported against the expression that was being evaluated when things
//! went wrong.

use crate::diagnostics;
use crate::lexer::token::Span;
use std::fmt;

/// One variant per way execution can fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeErrorKind {
    /// An integer was divided by zero.
    DivisionByZero,
    /// The true result of an integer operation does not fit in 64 bits.
    IntegerOverflow { operator: String },
    /// A binary operator was applied to kinds of value it is not defined
    /// for, such as `1 + true` or `"a" - "b"`.
    InvalidBinaryOperation {
        operator: String,
        left: &'static str,
        right: &'static str,
    },
    /// A unary operator was applied to a kind of value it is not defined
    /// for, such as `-true` or `!1`.
    InvalidUnaryOperation {
        operator: String,
        operand: &'static str,
    },
    /// A variable that does not exist was read or assigned. Semantic
    /// analysis rejects such programs before they run; the interpreter
    /// still reports it rather than crash, should it ever be given an
    /// unchecked program.
    UndefinedVariable { name: String },
    /// A call to a function that does not exist. As with
    /// `UndefinedVariable`, semantic analysis rejects this first.
    UndefinedFunction { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub span: Span,
}

impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    fn message(&self) -> String {
        match &self.kind {
            RuntimeErrorKind::DivisionByZero => "division by zero".to_string(),
            RuntimeErrorKind::IntegerOverflow { operator } => {
                format!("integer overflow: the result of `{operator}` does not fit in 64 bits")
            }
            RuntimeErrorKind::InvalidBinaryOperation {
                operator,
                left,
                right,
            } => format!("invalid operation: cannot apply `{operator}` to {left} and {right}"),
            RuntimeErrorKind::InvalidUnaryOperation { operator, operand } => {
                format!("invalid operation: cannot apply `{operator}` to {operand}")
            }
            RuntimeErrorKind::UndefinedVariable { name } => format!("undefined variable `{name}`"),
            RuntimeErrorKind::UndefinedFunction { name } => format!("undefined function `{name}`"),
        }
    }

    /// Render this error as a source-anchored diagnostic labelled
    /// `runtime error:`, distinct from every error an earlier stage can
    /// report. See `crate::diagnostics::render`.
    pub fn render(&self, file: &str, source: &str) -> String {
        diagnostics::render(file, source, self.span, "runtime error", &self.message())
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
