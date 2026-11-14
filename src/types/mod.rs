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
//! This module starts with the vocabulary the checker will speak: what a
//! type is (`ty.rs`), and how a type error is reported (`error.rs`).

pub mod error;
pub mod ty;
