//! Source-anchored diagnostic rendering, shared by the lexer and parser so
//! every compiler stage reports errors in the same shape.

use crate::lexer::token::Span;

/// Render `message` as a diagnostic anchored to `span` within `source`,
/// attributed to `file`, e.g.:
///
/// ```text
/// error: unexpected character '@'
///  --> example.dk:1:12
///   |
/// 1 | let x = 42 @ 10;
///   |            ^
/// ```
pub fn render(file: &str, source: &str, span: Span, message: &str) -> String {
    let (line, col, line_text) = locate(source, span.start);
    let gutter = line.to_string().len();
    let pointer = " ".repeat(col.saturating_sub(1));

    format!(
        "error: {message}\n{pad}--> {file}:{line}:{col}\n{pad} |\n{line} | {line_text}\n{pad} | {pointer}^\n",
        pad = " ".repeat(gutter),
    )
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
