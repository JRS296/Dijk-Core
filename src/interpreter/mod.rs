//! The interpreter: gives a checked program behavior by walking its AST.
//!
//! Every stage before this one only *described* the program: the lexer
//! found its pieces, the parser its structure, and semantic analysis
//! checked that its names mean something. The interpreter is the first
//! stage that *does* what the program says.
//!
//! It is a **tree-walking** interpreter. It evaluates the AST directly,
//! node by node, with no further translation in between:
//!
//! ```text
//! AST (source)                 Runtime
//!
//! Binary(+)                    Integer(30)
//! ├── Integer(10)      ──→
//! └── Integer(20)
//! ```
//!
//! The left side is a description of a computation. The right side is
//! what that computation produced. AST nodes are source; evaluating them
//! produces runtime values (`value.rs`).

pub mod environment;
pub mod error;
mod eval;
pub mod value;
