use crate::diagnostics;
use crate::lexer::token::{Span, TokenKind};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// `expected` is a human-readable description of what would have been
    /// valid here, e.g. "')'" or "parameter or ')'" or "expression".
    UnexpectedToken {
        expected: String,
        found: TokenKind,
    },
    InvalidAssignmentTarget,
    /// The digits lexed fine, but don't fit in a 64-bit signed integer.
    InvalidIntegerLiteral {
        lexeme: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    fn message(&self) -> String {
        match &self.kind {
            ParseErrorKind::UnexpectedToken { expected, found } => {
                format!("expected {expected}, found {}", found.describe())
            }
            ParseErrorKind::InvalidAssignmentTarget => "invalid assignment target".to_string(),
            ParseErrorKind::InvalidIntegerLiteral { lexeme } => {
                format!("integer literal '{lexeme}' is out of range for a 64-bit integer")
            }
        }
    }

    /// Render this error as a source-anchored diagnostic. See
    /// `crate::diagnostics::render`.
    pub fn render(&self, file: &str, source: &str) -> String {
        diagnostics::render(file, source, self.span, &self.message())
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
