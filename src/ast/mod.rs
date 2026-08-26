//! Dijk's abstract syntax tree: the compiler's structured representation
//! of a program, once the parser's provisional syntax representation has
//! been formalized. See `docs/ast.md`.
//!
//! This module is not wired into the crate yet — see the AST milestone's
//! commit progression. It becomes load-bearing once the parser is
//! rewritten to construct these nodes directly.

pub mod declaration;
