//! Semantic errors: programs that parse, but do not mean anything.

use crate::diagnostics;
use crate::lexer::token::Span;
use std::fmt;

/// One variant per rule semantic analysis enforces.
///
/// There are none yet. The analyzer can walk a program (see
/// `resolve.rs`), but it enforces no rules, so no semantic error can be
/// constructed. The type says exactly that: an enum with no variants has
/// no values. Each rule, once implemented, adds its own variant here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticErrorKind {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: Span,
}

impl SemanticError {
    pub fn new(kind: SemanticErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    fn message(&self) -> String {
        match self.kind {}
    }

    /// Render this error as a source-anchored diagnostic labelled
    /// `semantic error:`, so it cannot be mistaken for a lexical or parse
    /// error. See `crate::diagnostics::render`.
    pub fn render(&self, file: &str, source: &str) -> String {
        diagnostics::render(file, source, self.span, "semantic error", &self.message())
    }
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
