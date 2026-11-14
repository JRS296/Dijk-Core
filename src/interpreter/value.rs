//! Runtime values: what evaluating an expression produces.
//!
//! An `Expression::Integer(10, span)` is a piece of source code: it
//! records that someone *wrote* `10`, and where. A `Value::Integer(10)`
//! is the number ten, produced while the program runs, with no source
//! position at all. `1 + 2` and `3` are different expressions, but they
//! evaluate to the same value.
//!
//! The set of values is deliberately small. It has exactly one variant
//! for each kind of literal the language has, plus `Unit`. Arrays,
//! structs, floats, references and functions-as-values will each add a
//! variant when the language gains them, not before.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A 64-bit signed integer. Type annotations like `i32` are not
    /// enforced yet (that is the type checker's job), so every integer is
    /// an `i64` at run time.
    Integer(i64),
    Boolean(bool),
    String(String),
    /// "No value": the result of a call to a function that finishes
    /// without returning one.
    Unit,
}

impl Value {
    /// The name of this value's kind, as runtime error messages show it,
    /// e.g. "invalid operation: cannot apply `+` to integer and boolean".
    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::Integer(_) => "integer",
            Value::Boolean(_) => "boolean",
            Value::String(_) => "string",
            Value::Unit => "unit",
        }
    }
}

/// How `dijk run` prints a value: `30`, `true`, or a string's contents
/// without quotes. `Unit` prints as `()`, though `dijk run` never shows it.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(value) => write!(f, "{value}"),
            Value::Boolean(value) => write!(f, "{value}"),
            Value::String(value) => write!(f, "{value}"),
            Value::Unit => write!(f, "()"),
        }
    }
}
