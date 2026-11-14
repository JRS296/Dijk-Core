//! Static types: the stage that asks whether a program's operations are
//! valid *before* the program runs.
//!
//! Semantic analysis (`crate::semantic`) checks that every name in a
//! program refers to something. That is not enough to make a program
//! sensible. Every name in this program resolves:
//!
//! ```dijk
//! fn main() -> i32 {
//!     let x: i32 = "hello";
//!     return x;
//! }
//! ```
//!
//! But `"hello"` is a string, and `x` was declared to hold an `i32`.
//! Until now the interpreter discovered mistakes like this, if at all,
//! while running the program. A static type system finds them by reading
//! the program, without executing a single line of it:
//!
//! ```text
//! Semantic analysis  "Is this allowed to mean something?"
//!   ↓
//! Type checking      "Are its operations valid for its values?"
//!   ↓
//! Interpreter        "What does it do?"
//! ```
//!
//! The module is split by concept: what a type is (`ty.rs`), how a type
//! error is reported (`error.rs`), which operations each type allows
//! (`rules.rs`), which type each variable in scope has (`env.rs`), and
//! the checker that walks a program giving every expression a type
//! (`check.rs`). What the checker
//! *produces* is defined here, because it is this stage's contract with
//! every stage after it. See `docs/types.md`.

mod check;
pub mod env;
pub mod error;
pub mod rules;
pub mod ty;

use crate::ast::declaration::Program;
use crate::ast::expression::Expression;
use crate::lexer::token::Span;
use error::TypeError;
use std::collections::HashMap;
use ty::Type;

pub use check::literal_value;

/// Everything the type checker learned about a program, kept so that
/// later stages can use it instead of working it out again.
///
/// The AST itself is not changed: checking *reads* the tree and records
/// what it finds alongside it, in side tables.
///
/// # Why the table is keyed by `Span`
///
/// The AST's nodes have no identities of their own: no node IDs, no
/// pointers that later stages could share. What every expression does
/// have is its span, and **no two expressions in one program have the
/// same span**: a parent expression strictly contains its children (every
/// operator, call or assignment adds at least one token of its own), and
/// two siblings never overlap. Parentheses create no node at all, so
/// `(e)` and `e` are the same expression. A span therefore identifies an
/// expression exactly, and is something every stage already has.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TypedProgram {
    /// What was learned about each function, in declaration order.
    pub functions: Vec<TypedFunction>,
    /// The type of every expression the checker could type, keyed by the
    /// expression's span.
    pub expression_types: HashMap<Span, Type>,
}

/// What the checker learned about one function.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedFunction {
    pub name: String,
    pub signature: FunctionSignature,
    /// Every variable the function declares, parameters first, then each
    /// `let` in the order the checker met them (source order). Each one
    /// is listed separately, so a variable that shadows another appears
    /// as a binding of its own.
    pub bindings: Vec<Binding>,
}

/// A function's type: what it takes and what it gives back.
///
/// A signature is a contract between a function and everyone who calls
/// it. A caller promises to pass arguments of the parameter types; the
/// function promises to return a value of the return type. Each side can
/// then be checked on its own: a call against the signature, without
/// looking at the body, and the body against the signature, without
/// looking at any caller.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    /// Each parameter's name and type, in order. The names are not part
    /// of the contract (a caller passes arguments by position), but they
    /// make a signature readable.
    pub params: Vec<(String, Type)>,
    /// `unit` for a function declared without `-> T`.
    pub return_type: Type,
}

/// A variable and the type it was given: one entry of the typing
/// environment Γ, as it stood where the variable was declared.
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub name: String,
    /// Where the variable's name was written in its declaration.
    pub span: Span,
    pub ty: Type,
    /// Whether the variable was declared `let mut`. This is not part of
    /// its type: mutability says whether a *binding* may change, a type
    /// says what kind of value it holds. It is recorded here because this
    /// is the record of every binding, not because the checker uses it.
    pub mutable: bool,
    /// How deeply nested the declaring scope is: 0 for the function
    /// scope, 1 for a block directly inside it, and so on.
    pub depth: usize,
}

impl TypedProgram {
    /// The type the checker gave `expr`, if it gave it one.
    pub fn type_of(&self, expr: &Expression) -> Option<Type> {
        self.expression_types.get(&expr.span()).copied()
    }
}

/// Type-check a complete program that has passed semantic analysis.
///
/// Like every earlier stage, checking does not stop at the first problem:
/// it reports every type error it finds, in source order. The program
/// must come from a parse with no errors *and* an analysis with no
/// errors. The checker relies on every name resolving, and does not
/// report unresolved names again.
pub fn check(program: &Program) -> Result<TypedProgram, Vec<TypeError>> {
    check::check_program(program)
}
