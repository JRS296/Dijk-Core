use super::token::Span;
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
            LexErrorKind::UnexpectedCharacter(c) => format!("unexpected character '{c}'"),
            LexErrorKind::UnterminatedString => "unterminated string literal".to_string(),
        }
    }

    /// Render this error as a source-anchored diagnostic, e.g.:
    ///
    /// ```text
    /// error: unexpected character '@'
    ///  --> example.dk:1:12
    ///   |
    /// 1 | let x = 42 @ 10;
    ///   |             ^
    /// ```
    pub fn render(&self, file: &str, source: &str) -> String {
        let (line, col, line_text) = locate(source, self.span.start);
        let gutter = line.to_string().len();
        let pointer = " ".repeat(col.saturating_sub(1));

        format!(
            "error: {}\n{pad}--> {file}:{line}:{col}\n{pad} |\n{line} | {line_text}\n{pad} | {pointer}^\n",
            self.message(),
            pad = " ".repeat(gutter),
        )
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

/// Given a byte offset into `source`, return its 1-based line, 1-based
/// column, and the full text of the line it falls on.
fn locate(source: &str, offset: usize) -> (usize, usize, &str) {
    let mut line = 1;
    let mut line_start = 0;

    for (i, b) in source.as_bytes().iter().enumerate() {
        if i >= offset {
            break;
        }
        if *b == b'\n' {
            line += 1;
            line_start = i + 1;
        }
    }

    let line_end = source[line_start..]
        .find('\n')
        .map(|i| line_start + i)
        .unwrap_or(source.len());

    let col = offset - line_start + 1;
    (line, col, &source[line_start..line_end])
}
