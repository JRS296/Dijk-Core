use super::token::Span;
use crate::diagnostics;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexErrorKind {
    UnexpectedCharacter(char),
    UnterminatedString,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

impl LexError {
    pub fn new(kind: LexErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    fn message(&self) -> String {
        match self.kind {
            // A non-ASCII character may be invisible (a non-breaking
            // space) or look like an ASCII one, so its code point is shown.
            LexErrorKind::UnexpectedCharacter(c) if !c.is_ascii() => {
                format!("unexpected character '{c}' (U+{:04X})", u32::from(c))
            }
            LexErrorKind::UnexpectedCharacter(c) => format!("unexpected character '{c}'"),
            LexErrorKind::UnterminatedString => "unterminated string literal".to_string(),
        }
    }

    /// Render this error as a source-anchored diagnostic. See
    /// `crate::diagnostics::render`.
    pub fn render(&self, file: &str, source: &str) -> String {
        diagnostics::render(file, source, self.span, "error", &self.message())
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}
