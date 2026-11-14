//! Runtime values: what evaluating an expression produces.
//!
//! An `Expression::Integer(10, span)` is a piece of source code: it
//! records that someone *wrote* `10`, and where. A `Value::I32(10)` is
//! the number ten, produced while the program runs, with no source
//! position at all. `1 + 2` and `3` are different expressions, but they
//! evaluate to the same value.
//!
//! There is one variant for each of Dijk's types, so every value carries
//! its type at run time, and the two integer types are two different
//! kinds of value. An `i32` is a 32-bit integer *while the program runs*,
//! not just in the type checker's opinion, so it overflows where a real
//! 32-bit integer would. Were every integer stored the same way, the
//! running program would quietly contradict the static types it was
//! checked against.
//!
//! The set of values is deliberately small. Arrays, structs, floats,
//! references and functions-as-values will each add a variant when the
//! language gains them, not before.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A value of type `i32`: a 32-bit signed integer.
    I32(i32),
    /// A value of type `i64`: a 64-bit signed integer.
    I64(i64),
    Boolean(bool),
    String(String),
    /// "No value": the result of a call to a function that finishes
    /// without returning one. The only value of type `unit`.
    Unit,
}

impl Value {
    /// The name of this value's type, as runtime error messages show it,
    /// e.g. "integer overflow: the result of `+` does not fit in `i32`".
    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::I32(_) => "i32",
            Value::I64(_) => "i64",
            Value::Boolean(_) => "bool",
            Value::String(_) => "string",
            Value::Unit => "unit",
        }
    }
}

/// How `dijk run` prints a value: `30`, `true`, or a string's contents
/// without quotes. An integer prints the same whatever its type. `Unit`
/// prints as `()`, though `dijk run` never shows it.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::I32(value) => write!(f, "{value}"),
            Value::I64(value) => write!(f, "{value}"),
            Value::Boolean(value) => write!(f, "{value}"),
            Value::String(value) => write!(f, "{value}"),
            Value::Unit => write!(f, "()"),
        }
    }
}
