/// The root of a parsed Dijk program: every top-level declaration in the
/// order it appeared in the source.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub declarations: Vec<Declaration>,
}

/// A top-level declaration. `Function` is the only kind Dijk Core
/// supports today; structs and enums are expected to join this enum once
/// the grammar grows to include them.
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    Function(Function),
}

use super::statement::Statement;
use super::types::Type;

/// A function declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Vec<Statement>,
}

/// A single function parameter, e.g. `a: i32`.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}
