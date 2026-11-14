//! Runtime errors: problems that only show up while a program runs.
//!
//! Semantic analysis rejects programs that cannot mean anything, and type
//! checking rejects programs whose operations don't fit their types, both
//! before they run. Some problems, though, depend on the *values* a
//! program computes, such as a division whose divisor turns out to be
//! zero, or a sum too large for its type. No earlier stage can see those. They are runtime errors, and they are
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
    /// The true result of an integer operation does not fit in the type
    /// of its operands (`ty`, `i32` or `i64`).
    IntegerOverflow { operator: String, ty: &'static str },
    /// An integer literal does not fit in the type it was given. The type
    /// checker rejects such programs before they run; the interpreter
    /// still reports it rather than crash, should it ever be given an
    /// unchecked program.
    IntegerLiteralOutOfRange { value: i128, ty: &'static str },
    /// A binary operator was applied to kinds of value it is not defined
    /// for, such as `1 + true` or `"a" - "b"`. The type checker rejects
    /// such programs before they run; like the errors below, this is the
    /// interpreter's own guard against a program that skipped it.
    InvalidBinaryOperation {
        operator: String,
        left: &'static str,
        right: &'static str,
    },
    /// A unary operator was applied to a kind of value it is not defined
    /// for, such as `-true` or `!1`. Rejected by the type checker first.
    InvalidUnaryOperation {
        operator: String,
        operand: &'static str,
    },
    /// The condition of an `if` or `while` evaluated to something other
    /// than a boolean. Rejected by the type checker first.
    InvalidCondition { found: &'static str },
    /// A variable that does not exist was read or assigned. Semantic
    /// analysis rejects such programs before they run; the interpreter
    /// still reports it rather than crash, should it ever be given an
    /// unchecked program.
    UndefinedVariable { name: String },
    /// A call to a function that does not exist. As with
    /// `UndefinedVariable`, semantic analysis rejects this first.
    UndefinedFunction { name: String },
    /// A call passing the wrong number of arguments. Semantic analysis
    /// rejects this first.
    ArityMismatch {
        name: String,
        expected: usize,
        found: usize,
    },
    /// The program has no `main` to start from. Semantic analysis rejects
    /// this first.
    MissingMain,
    /// A call would have put more than `limit` calls in progress at
    /// once, almost always because of recursion that never stops.
    CallDepthExceeded { limit: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub span: Span,
    /// The functions whose calls were in progress when the error
    /// happened, innermost (the one that failed) first, e.g.
    /// `["divide", "main"]`. The interpreter fills this in as the error
    /// leaves each call (see `eval.rs`). It is empty for an error raised
    /// before any call started.
    pub call_stack: Vec<String>,
}

impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind, span: Span) -> Self {
        Self {
            kind,
            span,
            call_stack: Vec::new(),
        }
    }

    fn message(&self) -> String {
        match &self.kind {
            RuntimeErrorKind::DivisionByZero => "division by zero".to_string(),
            RuntimeErrorKind::IntegerOverflow { operator, ty } => {
                format!("integer overflow: the result of `{operator}` does not fit in `{ty}`")
            }
            RuntimeErrorKind::IntegerLiteralOutOfRange { value, ty } => {
                format!("integer literal `{value}` does not fit in `{ty}`")
            }
            RuntimeErrorKind::InvalidBinaryOperation {
                operator,
                left,
                right,
            } => format!("invalid operation: cannot apply `{operator}` to {left} and {right}"),
            RuntimeErrorKind::InvalidUnaryOperation { operator, operand } => {
                format!("invalid operation: cannot apply `{operator}` to {operand}")
            }
            RuntimeErrorKind::InvalidCondition { found } => {
                format!("invalid operation: a condition must be a boolean, found {found}")
            }
            RuntimeErrorKind::UndefinedVariable { name } => format!("undefined variable `{name}`"),
            RuntimeErrorKind::UndefinedFunction { name } => format!("undefined function `{name}`"),
            RuntimeErrorKind::ArityMismatch {
                name,
                expected,
                found,
            } => {
                format!("wrong number of arguments to `{name}`: expected {expected}, found {found}")
            }
            RuntimeErrorKind::MissingMain => "no `main` function to run".to_string(),
            RuntimeErrorKind::CallDepthExceeded { limit } => {
                format!("stack overflow: more than {limit} nested function calls")
            }
        }
    }

    /// The call stack as one line, innermost call first:
    /// `divide ← main`. Runaway recursion can leave a thousand frames, so
    /// a long stack keeps only its first few and its last entry.
    fn call_stack_note(&self) -> String {
        const SHOWN: usize = 4;

        let frames = &self.call_stack;
        if frames.len() <= SHOWN + 1 {
            return frames.join(" ← ");
        }

        let first = frames[..SHOWN].join(" ← ");
        let omitted = frames.len() - SHOWN - 1;
        let last = &frames[frames.len() - 1];
        format!("{first} ← … {omitted} more … ← {last}")
    }

    /// Render this error as a source-anchored diagnostic labelled
    /// `runtime error:`, distinct from every error an earlier stage can
    /// report, followed by the call stack at the moment it happened. See
    /// `crate::diagnostics::render`.
    ///
    /// ```text
    /// runtime error: division by zero
    ///  --> example.dk:2:16
    ///   |
    /// 2 |     return a / b;
    ///   |                ^
    /// note: call stack: divide ← main
    /// ```
    pub fn render(&self, file: &str, source: &str) -> String {
        let mut rendered =
            diagnostics::render(file, source, self.span, "runtime error", &self.message());
        if !self.call_stack.is_empty() {
            rendered.push_str(&format!("note: call stack: {}\n", self.call_stack_note()));
        }
        rendered
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
