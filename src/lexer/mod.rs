//! Lexical analysis: turns Dijk source text into a stream of tokens.
//! See `docs/lexer.md` for the specification this module implements.

pub mod error;
mod scan;
pub mod token;

pub use scan::tokenize;
