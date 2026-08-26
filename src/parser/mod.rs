//! Parsing: turns a token stream into an initial syntax representation.
//! See `docs/parser.md` for the grammar this module implements, including
//! why that representation is deliberately not yet called "the AST."

pub mod error;
mod parse;
pub mod syntax;

pub use parse::parse;
