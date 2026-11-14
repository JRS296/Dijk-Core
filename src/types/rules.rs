//! The operator typing rules, all in one place.
//!
//! Each rule is one row of a table: "this operator, applied to operands
//! of these types, produces a value of this type". Written as a typing
//! rule, the first row for `+` says:
//!
//! ```text
//! Γ ⊢ a : i32    Γ ⊢ b : i32
//! ───────────────────────────
//!       Γ ⊢ a + b : i32
//! ```
//!
//! "If, in the environment Γ, `a` has type `i32` and `b` has type `i32`,
//! then `a + b` has type `i32`." An operator applied to a combination of
//! types that has no row is a type error. Nothing is allowed just because
//! it could be given *some* meaning: `"a" - "b"` and `1 + true` have no
//! row, so they are rejected, and `i32 + i64` has no row either, because
//! Dijk never converts between integer types on its own.
//!
//! The complete table:
//!
//! ```text
//! +            i32 × i32 → i32     i64 × i64 → i64    string × string → string
//! - * /        i32 × i32 → i32     i64 × i64 → i64
//! < <= > >=    i32 × i32 → bool    i64 × i64 → bool
//! == !=        i32 × i32 → bool    i64 × i64 → bool
//!              bool × bool → bool  string × string → bool
//! - (unary)    i32 → i32           i64 → i64
//! !            bool → bool
//! ```
//!
//! No operator accepts `unit`. Keeping the rules as data, instead of
//! scattering them through the checker, means the checker, its
//! diagnostics and the documentation all read from one table.

use super::ty::Type;
use crate::ast::expression::{BinaryOperator, UnaryOperator};

/// One typing rule for a binary operator: `left op right : result`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryRule {
    pub left: Type,
    pub right: Type,
    pub result: Type,
}

/// One typing rule for a unary operator: `op operand : result`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnaryRule {
    pub operand: Type,
    pub result: Type,
}

const fn binary(left: Type, right: Type, result: Type) -> BinaryRule {
    BinaryRule {
        left,
        right,
        result,
    }
}

/// `+`: integer addition, and string concatenation.
const ADDITION: [BinaryRule; 3] = [
    binary(Type::I32, Type::I32, Type::I32),
    binary(Type::I64, Type::I64, Type::I64),
    binary(Type::String, Type::String, Type::String),
];

/// `-`, `*`, `/`: integers only, both of the same type, which is also the
/// type of the result.
const ARITHMETIC: [BinaryRule; 2] = [
    binary(Type::I32, Type::I32, Type::I32),
    binary(Type::I64, Type::I64, Type::I64),
];

/// `<`, `<=`, `>`, `>=`: integers of the same type, compared to a `bool`.
const ORDERING: [BinaryRule; 2] = [
    binary(Type::I32, Type::I32, Type::Bool),
    binary(Type::I64, Type::I64, Type::Bool),
];

/// `==`, `!=`: any two values of the same type, except `unit`.
const EQUALITY: [BinaryRule; 4] = [
    binary(Type::I32, Type::I32, Type::Bool),
    binary(Type::I64, Type::I64, Type::Bool),
    binary(Type::Bool, Type::Bool, Type::Bool),
    binary(Type::String, Type::String, Type::Bool),
];

/// Unary `-`: integers.
const NEGATION: [UnaryRule; 2] = [
    UnaryRule {
        operand: Type::I32,
        result: Type::I32,
    },
    UnaryRule {
        operand: Type::I64,
        result: Type::I64,
    },
];

/// `!`: booleans.
const LOGICAL_NOT: [UnaryRule; 1] = [UnaryRule {
    operand: Type::Bool,
    result: Type::Bool,
}];

/// Every rule for `operator`.
pub fn binary_rules(operator: BinaryOperator) -> &'static [BinaryRule] {
    match operator {
        BinaryOperator::Add => &ADDITION,
        BinaryOperator::Sub | BinaryOperator::Mul | BinaryOperator::Div => &ARITHMETIC,
        BinaryOperator::Lt | BinaryOperator::Le | BinaryOperator::Gt | BinaryOperator::Ge => {
            &ORDERING
        }
        BinaryOperator::Eq | BinaryOperator::NotEq => &EQUALITY,
    }
}

/// Every rule for `operator`.
pub fn unary_rules(operator: UnaryOperator) -> &'static [UnaryRule] {
    match operator {
        UnaryOperator::Neg => &NEGATION,
        UnaryOperator::Not => &LOGICAL_NOT,
    }
}

/// The type of `left operator right`, or `None` if no rule allows it.
pub fn binary_result(operator: BinaryOperator, left: Type, right: Type) -> Option<Type> {
    binary_rules(operator)
        .iter()
        .find(|rule| rule.left == left && rule.right == right)
        .map(|rule| rule.result)
}

/// The type of `operator operand`, or `None` if no rule allows it.
pub fn unary_result(operator: UnaryOperator, operand: Type) -> Option<Type> {
    unary_rules(operator)
        .iter()
        .find(|rule| rule.operand == operand)
        .map(|rule| rule.result)
}

/// Whether `operator` is arithmetic: its result has the same type as its
/// operands (`+ - * /`), as opposed to a comparison, whose result is a
/// `bool` whatever its operands are. This decides whether what the
/// context expects of the result can be passed down to the operands.
pub fn is_arithmetic(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Add | BinaryOperator::Sub | BinaryOperator::Mul | BinaryOperator::Div
    )
}
