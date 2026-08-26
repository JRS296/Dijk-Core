//! Dijk's abstract syntax tree: the compiler's structured representation
//! of a program. The parser (`crate::parser`) constructs these nodes
//! directly. See `docs/ast.md`.

pub mod declaration;
pub mod expression;
pub mod statement;
pub mod types;
pub mod visit;
