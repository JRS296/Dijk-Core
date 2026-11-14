//! Runtime errors: problems that only show up while a program runs.
//!
//! Semantic analysis rejects programs that cannot mean anything, before
//! they run. Some problems, though, depend on the *values* a program
//! computes, such as a division whose divisor turns out to be zero. No
//! earlier stage can see those. They are runtime errors, and they are
//! reported against the expression that was being evaluated when things
//! went wrong.

use crate::diagnostics;
use crate::lexer::token::Span;
use std::fmt;

/// One variant per way execution can fail.
///
/// There are none yet. Nothing is evaluated so far, so nothing can fail.
/// An enum with no variants has no values, and each part of the
/// interpreter adds the failures it can produce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeErrorKind {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub span: Span,
}

impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    fn message(&self) -> String {
        match self.kind {}
    }

    /// Render this error as a source-anchored diagnostic labelled
    /// `runtime error:`, distinct from every error an earlier stage can
    /// report. See `crate::diagnostics::render`.
    pub fn render(&self, file: &str, source: &str) -> String {
        diagnostics::render(file, source, self.span, "runtime error", &self.message())
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
