//! Parsing: turns a token stream directly into Dijk's AST (`crate::ast`).
//! See `docs/parser.md` for the grammar this module implements, and
//! `docs/ast.md` for the representation it constructs.

pub mod error;
mod parse;

pub use parse::parse;
