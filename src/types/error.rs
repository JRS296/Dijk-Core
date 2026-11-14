//! Type errors: programs whose names all mean something, but whose
//! operations are not valid for the kinds of value they are applied to.

use crate::diagnostics;
use crate::lexer::token::Span;
use std::fmt;

/// One variant per rule the type checker enforces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeErrorKind {
    /// A type annotation names a type Dijk does not have, such as
    /// `let x: banana = 1;`.
    UnknownType { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub span: Span,
}

impl TypeError {
    pub fn new(kind: TypeErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    fn message(&self) -> String {
        match &self.kind {
            TypeErrorKind::UnknownType { name } => format!("unknown type `{name}`"),
        }
    }

    /// Render this error as a source-anchored diagnostic labelled
    /// `type error:`, distinct from every other stage's label. See
    /// `crate::diagnostics::render`.
    pub fn render(&self, file: &str, source: &str) -> String {
        diagnostics::render(file, source, self.span, "type error", &self.message())
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
