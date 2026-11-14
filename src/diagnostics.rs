//! Source-anchored diagnostic rendering, shared by every compiler stage so
//! each one reports errors in the same shape.
//!
//! The shape is fixed; only the leading label changes. The label names
//! the class of problem, which is also the stage that detected it:
//! lexical and parse errors are plain `error:`, while semantic analysis,
//! the type checker and the interpreter say `semantic error:`,
//! `type error[E…]:` and `runtime error:`. The label tells a reader
//! *which* stage rejected the program.

use crate::lexer::token::Span;

/// Render `message` as a diagnostic anchored to `span` within `source`,
/// attributed to `file` and introduced by `label`. With the label
/// `"error"`:
///
/// ```text
/// error: unexpected character '@'
///  --> example.dk:1:12
///   |
/// 1 | let x = 42 @ 10;
///   |            ^
/// ```
///
/// A single caret points at where the problem starts.
pub fn render(file: &str, source: &str, span: Span, label: &str, message: &str) -> String {
    let location = locate(source, span.start);
    let mut rendered = header(file, &location, label, message);
    rendered.push_str(&marker_line(&location, 1));
    rendered
}

/// Like [`render`], but underline the *whole* of `span` (up to the end of
/// the line it starts on), and follow the source line with `notes`:
///
/// ```text
/// type error[E0001]: type mismatch
///  --> main.dk:2:18
///   |
/// 2 |     let x: i32 = "hello";
///   |                  ^^^^^^^
///   = expected `i32`
///   = found `string`
/// ```
///
/// The type checker uses this form: a type belongs to a whole expression,
/// so the diagnostic marks all of it, and the notes say what type was
/// expected there and what was found instead.
pub fn render_with_notes(
    file: &str,
    source: &str,
    span: Span,
    label: &str,
    message: &str,
    notes: &[String],
) -> String {
    let location = locate(source, span.start);
    let line_end = location.line_start + location.text.len();
    let width = span.end.min(line_end).saturating_sub(span.start).max(1);

    let mut rendered = header(file, &location, label, message);
    rendered.push_str(&marker_line(&location, width));
    let pad = " ".repeat(location.gutter());
    for note in notes {
        rendered.push_str(&format!("{pad} = {note}\n"));
    }
    rendered
}

/// Where an offset falls in the source, for display.
struct Location<'s> {
    /// 1-based line number.
    line: usize,
    /// 1-based column.
    column: usize,
    /// Byte offset at which the line starts.
    line_start: usize,
    /// The full text of the line, without its newline.
    text: &'s str,
}

impl Location<'_> {
    /// How wide the line-number gutter is.
    fn gutter(&self) -> usize {
        self.line.to_string().len()
    }
}

/// The lines every diagnostic starts with: the label and message, the
/// file and position, and the source line itself.
fn header(file: &str, location: &Location, label: &str, message: &str) -> String {
    let pad = " ".repeat(location.gutter());
    format!(
        "{label}: {message}\n{pad}--> {file}:{line}:{column}\n{pad} |\n{line} | {text}\n",
        line = location.line,
        column = location.column,
        text = location.text,
    )
}

/// The line under the source line: `width` carets under the problem.
fn marker_line(location: &Location, width: usize) -> String {
    format!(
        "{pad} | {indent}{carets}\n",
        pad = " ".repeat(location.gutter()),
        indent = " ".repeat(location.column.saturating_sub(1)),
        carets = "^".repeat(width),
    )
}

/// Given a byte offset into `source`, find its 1-based line and column,
/// and the full text of the line it falls on.
fn locate(source: &str, offset: usize) -> Location<'_> {
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

    Location {
        line,
        column: offset - line_start + 1,
        line_start,
        text: &source[line_start..line_end],
    }
}
