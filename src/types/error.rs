//! Type errors: programs whose names all mean something, but whose
//! operations are not valid for the kinds of value they are applied to.
//!
//! A type error is rendered like this:
//!
//! ```text
//! type error[E0001]: type mismatch
//!  --> main.dk:2:18
//!   |
//! 2 |     let x: i32 = "hello";
//!   |                  ^^^^^^^
//!   = expected `i32`
//!   = found `string`
//! ```
//!
//! Each part comes from somewhere specific. The **code** (`E0001`) names
//! the rule that was broken; codes are stable, so one can be looked up
//! in `docs/types.md` and always means the same thing. The **message**
//! says which rule that is. The **span** is the expression the checker
//! was looking at when the rule failed, underlined in full. The
//! **notes** are the two types the checker compared: the one the context
//! *expected* (from an annotation, a parameter, a return type, an
//! operator's rules) and the one it *found* by checking the expression.
//! The checker knows both at the moment it reports the error, which is
//! why it can say more than "wrong type".
//!
//! Codes are never reused: if a rule is retired, its code is retired
//! with it, and a new rule gets a new code.

use super::rules;
use super::ty::Type;
use crate::ast::expression::{BinaryOperator, UnaryOperator};
use crate::diagnostics;
use crate::lexer::token::Span;
use std::fmt;

/// One variant per rule the type checker enforces, in order of their
/// diagnostic codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeErrorKind {
    /// E0001. An expression's type is not the type required where it
    /// appears, such as `let x: i32 = "hello";`.
    Mismatch { expected: Type, found: Type },
    /// E0002. A binary operator applied to operand types it has no rule
    /// for, such as `10 + "hello"` or `1 < true` (see `rules.rs`).
    InvalidBinaryOperation {
        operator: BinaryOperator,
        left: Type,
        right: Type,
    },
    /// E0003. A unary operator applied to an operand type it has no rule
    /// for, such as `-true` or `!1`.
    InvalidUnaryOperation {
        operator: UnaryOperator,
        operand: Type,
    },
    /// E0004. A call passes an argument whose type is not its parameter's,
    /// such as `add(10, "hello")` for `fn add(a: i32, b: i32)`.
    /// `position` counts from 1.
    InvalidArgument {
        function: String,
        parameter: String,
        position: usize,
        expected: Type,
        found: Type,
    },
    /// E0005. A `return` whose value does not match the function's return
    /// type. A bare `return;` is found to be `unit`, so it is this error
    /// in a function with `-> T`, and so is `return value;` in a function
    /// without one (which is expected to return `unit`).
    InvalidReturn {
        function: String,
        expected: Type,
        found: Type,
    },
    /// E0006. The condition of an `if` or `while` (`statement`) is not a
    /// `bool`, such as `if 10 { … }`.
    InvalidCondition {
        statement: &'static str,
        found: Type,
    },
    /// E0007. An assignment stores a value whose type is not the
    /// variable's, such as `x = "hello";` for an `i32` variable `x`.
    InvalidAssignment {
        name: String,
        expected: Type,
        found: Type,
    },
    /// E0008. A type annotation names a type Dijk does not have, such as
    /// `let x: banana = 1;`.
    UnknownType { name: String },
    /// E0009. A function with a return type other than `unit` can reach
    /// the end of its body without returning a value.
    MissingReturn { function: String, expected: Type },
    /// E0010. An integer literal whose value does not fit in the type it
    /// was given, such as `3000000000` where an `i32` is expected.
    /// `value` includes a minus sign written directly before the digits.
    IntegerLiteralOutOfRange { value: i128, ty: Type },
}

/// A type error: which rule was broken, and where.
///
/// Everything else a diagnostic shows is derived from these two: the
/// stable code ([`TypeError::code`]), the message, and the expected and
/// found types in the notes ([`TypeError::notes`]) all come from the
/// kind, which records the facts the checker had when the rule failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub span: Span,
}

impl TypeError {
    pub fn new(kind: TypeErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    /// The stable diagnostic code for this kind of error.
    pub fn code(&self) -> &'static str {
        match &self.kind {
            TypeErrorKind::Mismatch { .. } => "E0001",
            TypeErrorKind::InvalidBinaryOperation { .. } => "E0002",
            TypeErrorKind::InvalidUnaryOperation { .. } => "E0003",
            TypeErrorKind::InvalidArgument { .. } => "E0004",
            TypeErrorKind::InvalidReturn { .. } => "E0005",
            TypeErrorKind::InvalidCondition { .. } => "E0006",
            TypeErrorKind::InvalidAssignment { .. } => "E0007",
            TypeErrorKind::UnknownType { .. } => "E0008",
            TypeErrorKind::MissingReturn { .. } => "E0009",
            TypeErrorKind::IntegerLiteralOutOfRange { .. } => "E0010",
        }
    }

    /// The one-line summary: which rule was broken.
    fn message(&self) -> String {
        match &self.kind {
            TypeErrorKind::Mismatch { .. } => "type mismatch".to_string(),
            TypeErrorKind::InvalidBinaryOperation { operator, .. } => {
                format!("invalid binary operation: `{operator}`")
            }
            TypeErrorKind::InvalidUnaryOperation { operator, .. } => {
                format!("invalid unary operation: `{operator}`")
            }
            TypeErrorKind::InvalidArgument { function, .. } => {
                format!("invalid argument to `{function}`")
            }
            TypeErrorKind::InvalidReturn { function, .. } => {
                format!("invalid return type in `{function}`")
            }
            TypeErrorKind::InvalidCondition { statement, .. } => {
                format!("invalid condition type in `{statement}`")
            }
            TypeErrorKind::InvalidAssignment { name, .. } => {
                format!("invalid assignment to `{name}`")
            }
            TypeErrorKind::UnknownType { name } => format!("unknown type name `{name}`"),
            TypeErrorKind::MissingReturn { function, .. } => {
                format!("missing return in `{function}`")
            }
            TypeErrorKind::IntegerLiteralOutOfRange { .. } => {
                "integer literal out of range".to_string()
            }
        }
    }

    /// The lines shown after the source: first what was expected and what
    /// was found, then (prefixed `note:`) anything else that explains the
    /// rule.
    pub fn notes(&self) -> Vec<String> {
        match &self.kind {
            TypeErrorKind::Mismatch { expected, found } => expected_found(expected, found),
            TypeErrorKind::InvalidBinaryOperation {
                operator,
                left,
                right,
            } => {
                let allowed: Vec<String> = rules::binary_rules(*operator)
                    .iter()
                    .map(|rule| format!("`{} {operator} {}`", rule.left, rule.right))
                    .collect();
                vec![
                    format!("expected {}", one_of(&allowed)),
                    format!("found `{left} {operator} {right}`"),
                ]
            }
            TypeErrorKind::InvalidUnaryOperation { operator, operand } => {
                let allowed: Vec<String> = rules::unary_rules(*operator)
                    .iter()
                    .map(|rule| format!("`{operator}{}`", rule.operand))
                    .collect();
                vec![
                    format!("expected {}", one_of(&allowed)),
                    format!("found `{operator}{operand}`"),
                ]
            }
            TypeErrorKind::InvalidArgument {
                function,
                parameter,
                position,
                expected,
                found,
            } => {
                let mut notes = expected_found(expected, found);
                notes.push(format!(
                    "note: argument {position} of `{function}` is its parameter `{parameter}: {expected}`"
                ));
                notes
            }
            TypeErrorKind::InvalidReturn {
                function,
                expected,
                found,
            } => {
                let mut notes = expected_found(expected, found);
                notes.push(match expected {
                    Type::Unit => format!(
                        "note: `{function}` has no `-> T`, so it returns `unit`: no value at all"
                    ),
                    _ => format!("note: `{function}` is declared `-> {expected}`"),
                });
                notes
            }
            TypeErrorKind::InvalidCondition { statement, found } => {
                let mut notes = expected_found(&Type::Bool, found);
                notes.push(format!(
                    "note: Dijk has no \"truthiness\": the condition of `{statement}` must be a `bool`"
                ));
                notes
            }
            TypeErrorKind::InvalidAssignment {
                name,
                expected,
                found,
            } => {
                let mut notes = expected_found(expected, found);
                notes.push(format!(
                    "note: `{name}` has type `{expected}`, fixed when it was declared"
                ));
                notes
            }
            TypeErrorKind::UnknownType { name } => {
                let mut notes = vec![
                    "note: the types that can be written are `i32`, `i64`, `bool` and `string`"
                        .to_string(),
                ];
                if name == "unit" {
                    notes.push(
                        "note: `unit` cannot be written; leave out `-> T` for a function that returns nothing"
                            .to_string(),
                    );
                }
                notes
            }
            TypeErrorKind::MissingReturn { function, expected } => vec![
                format!("expected `{expected}`"),
                format!("found the end of `{function}`, with no `return`"),
                "note: a `return` inside an `if` without `else`, or inside a `while`, might never run"
                    .to_string(),
            ],
            TypeErrorKind::IntegerLiteralOutOfRange { value, ty } => {
                let mut notes = vec![format!("expected `{ty}`"), format!("found `{value}`")];
                if let Some((min, max)) = range(*ty) {
                    notes.push(format!("note: `{ty}` holds {min} to {max}"));
                }
                notes
            }
        }
    }

    /// Render this error as a source-anchored diagnostic labelled
    /// `type error[E000N]:`, underlining the whole offending expression
    /// and followed by its notes. See
    /// `crate::diagnostics::render_with_notes`.
    pub fn render(&self, file: &str, source: &str) -> String {
        diagnostics::render_with_notes(
            file,
            source,
            self.span,
            &format!("type error[{}]", self.code()),
            &self.message(),
            &self.notes(),
        )
    }
}

/// The two notes every type comparison ends in.
fn expected_found(expected: &Type, found: &Type) -> Vec<String> {
    vec![format!("expected `{expected}`"), format!("found `{found}`")]
}

/// `a`, `a or b`, `a, b or c`.
fn one_of(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [init @ .., last] => format!("{} or {last}", init.join(", ")),
    }
}

/// The smallest and largest value of an integer type.
fn range(ty: Type) -> Option<(i64, i64)> {
    match ty {
        Type::I32 => Some((i32::MIN.into(), i32::MAX.into())),
        Type::I64 => Some((i64::MIN, i64::MAX)),
        _ => None,
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
