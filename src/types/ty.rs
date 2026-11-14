//! What a type *is*, inside the compiler.
//!
//! A type answers one question about a value: **what kind of value is
//! this, and which operations are valid on it?** `i32` says "a 32-bit
//! signed integer: you may add it to another `i32`, compare it, negate
//! it". `bool` says "true or false: you may test it in an `if`, or
//! negate it with `!`". Nothing more. In particular, a type does *not*
//! say where a value lives (a register, a stack slot, the heap). That
//! is a separate question, the memory model's, and Dijk keeps the two
//! apart on purpose.
//!
//! Dijk's first type system is deliberately tiny: five types, each a
//! plain name with no parameters, so a type is just an enum variant.
//! There are no generic types, references, arrays or type variables to
//! represent yet, and so no structure inside a `Type` at all.
//!
//! The AST also has a thing called a type (`crate::ast::types::Type`),
//! but that is only a *name the programmer wrote*, like `i32` or
//! `banana`, with a span. It is an annotation, not a type. Turning the
//! annotation into a real [`Type`] (or rejecting it) is [`resolve`]'s
//! job.

use super::error::{TypeError, TypeErrorKind};
use crate::ast::types::Type as TypeAnnotation;
use std::fmt;

/// A Dijk type, as the type checker understands it.
///
/// Two types are the same type exactly when they are the same variant:
/// `i32 == i32`, but `i32 != i64`. There is no subtyping and no implicit
/// conversion between them (see [`Type::compatible`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    /// A 32-bit signed integer. The default type of an integer literal.
    I32,
    /// A 64-bit signed integer.
    I64,
    /// `true` or `false`.
    Bool,
    /// A string (without escapes, in Dijk Core today).
    String,
    /// "No meaningful value": what a function without `-> T` returns.
    /// It has no literal and cannot be written in an annotation; it only
    /// ever arises from a function that returns nothing.
    Unit,
    /// **Not a Dijk type.** The type the checker gives an expression it
    /// has already reported an error for (or could not type at all), so
    /// that one mistake produces one diagnostic instead of a cascade:
    /// `Error` is compatible with every type. A program that type-checks
    /// successfully never contains it.
    Error,
}

impl Type {
    /// The type a name in a type annotation refers to, if it names one.
    /// Only the four types a programmer can write have names: `unit` is
    /// internal, and `Error` is not a Dijk type at all.
    pub fn from_name(name: &str) -> Option<Type> {
        match name {
            "i32" => Some(Type::I32),
            "i64" => Some(Type::I64),
            "bool" => Some(Type::Bool),
            "string" => Some(Type::String),
            _ => None,
        }
    }

    /// Whether this is one of the integer types, the ones arithmetic and
    /// ordering work on.
    pub fn is_integer(self) -> bool {
        matches!(self, Type::I32 | Type::I64)
    }

    /// Whether a value of type `found` may be used where a value of type
    /// `expected` is required.
    ///
    /// In Dijk this is **equality**. There are no implicit conversions:
    /// an `i32` is not accepted where an `i64` is expected, even though
    /// every `i32` value would fit. A conversion, when Dijk has one, will
    /// be written explicitly. The only exception is the internal `Error`
    /// type, which is compatible with everything, so that an expression
    /// already reported as wrong is not reported again by whatever uses it.
    pub fn compatible(expected: Type, found: Type) -> bool {
        expected == found || expected == Type::Error || found == Type::Error
    }
}

/// How a type is written in Dijk source and in diagnostics: `i32`, `i64`,
/// `bool`, `string`, `unit`.
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Type::I32 => "i32",
            Type::I64 => "i64",
            Type::Bool => "bool",
            Type::String => "string",
            Type::Unit => "unit",
            Type::Error => "{error}",
        })
    }
}

/// Resolve a type annotation from the AST (`let x: i32`, `a: i64`,
/// `-> bool`) to the type it names, or report an unknown type name.
pub fn resolve(annotation: &TypeAnnotation) -> Result<Type, TypeError> {
    Type::from_name(&annotation.name).ok_or_else(|| {
        TypeError::new(
            TypeErrorKind::UnknownType {
                name: annotation.name.clone(),
            },
            annotation.span,
        )
    })
}
