//! Semantic errors: programs that parse, but do not mean anything.

use crate::diagnostics;
use crate::lexer::token::Span;
use std::fmt;

/// One variant per rule semantic analysis enforces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticErrorKind {
    /// A name is used as a variable, but no variable of that name is
    /// visible here.
    UnknownIdentifier { name: String },
    /// An assignment targets a variable that was never declared with
    /// `let` (or as a parameter). Assignment changes an existing
    /// variable; it never creates one.
    AssignToUndeclared { name: String },
    /// A call names a function that the program does not define.
    UnknownFunction { name: String },
    /// A call passes a different number of arguments than the function
    /// declares parameters.
    ArityMismatch {
        name: String,
        expected: usize,
        found: usize,
    },
    /// Two functions share a name, so a call to it would be ambiguous.
    DuplicateFunction { name: String },
    /// Two parameters of one function share a name.
    DuplicateParameter { name: String },
    /// The program defines no `main`, so there is nowhere to start
    /// running it.
    MissingMain,
    /// `main` declares parameters, but nothing could ever pass it
    /// arguments: it is called by `dijk run`, not by Dijk code.
    MainHasParameters,
}

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
        match &self.kind {
            SemanticErrorKind::UnknownIdentifier { name } => {
                format!("unknown identifier `{name}`")
            }
            SemanticErrorKind::AssignToUndeclared { name } => {
                format!("cannot assign to undeclared variable `{name}`")
            }
            SemanticErrorKind::UnknownFunction { name } => format!("unknown function `{name}`"),
            SemanticErrorKind::ArityMismatch {
                name,
                expected,
                found,
            } => {
                format!("wrong number of arguments to `{name}`: expected {expected}, found {found}")
            }
            SemanticErrorKind::DuplicateFunction { name } => {
                format!("function `{name}` is defined more than once")
            }
            SemanticErrorKind::DuplicateParameter { name } => {
                format!("parameter `{name}` is declared more than once")
            }
            SemanticErrorKind::MissingMain => {
                "no `main` function: a Dijk program starts running at `fn main()`".to_string()
            }
            SemanticErrorKind::MainHasParameters => "`main` must not take parameters".to_string(),
        }
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
