//! Semantic analysis: the stage between the AST and execution.
//!
//! The parser answers "is this program structurally valid?". The AST
//! records *what was written*. Neither asks whether the program is
//! allowed to *mean* anything. This program parses into a perfectly good
//! tree:
//!
//! ```dijk
//! fn main() {
//!     return unknown_variable;
//! }
//! ```
//!
//! But `unknown_variable` refers to nothing. Semantic analysis is the
//! stage that asks, for every name in the program, "what does this refer
//! to, and is that allowed here?". It rejects programs that are
//! well-formed but meaningless, *before* anything runs.
//!
//! ```text
//! Parser             "Is this structurally valid?"
//!   ↓
//! AST                "What was written?"
//!   ↓
//! Semantic analysis  "Is this allowed to mean something?"
//!   ↓
//! Interpreter        "What does it do?"
//! ```
//!
//! In Dijk 0.0.4 this stage is deliberately small. It resolves names and
//! nothing else. It is **not** a type checker: `let x: i32 = "hello";` is
//! still accepted here, because checking types is a later milestone.
//! See `docs/semantic.md`.

pub mod error;
mod resolve;

use crate::ast::declaration::Program;
use error::SemanticError;

/// Analyze a complete, successfully parsed `program`, returning every
/// semantic error found. An empty result means the program may be run.
///
/// Like the lexer and parser, analysis does not stop at the first
/// problem. It collects everything it can find, so one pass can report
/// more than one mistake. The program must come from a parse that
/// produced no errors: a partial AST (functions or statements dropped
/// during error recovery) would produce misleading semantic errors.
pub fn analyze(program: &Program) -> Vec<SemanticError> {
    resolve::resolve(program)
}
