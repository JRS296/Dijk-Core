//! Source-anchored diagnostic rendering, shared by every compiler stage so
//! each one reports errors in the same shape.
//!
//! The shape is fixed; only the leading label changes. The label names
//! the class of problem, which is also the stage that detected it:
//! lexical and parse errors are plain `error:`, while semantic analysis,
//! the type checker and the interpreter say `semantic error:`,
//! `type error[E…]:` and `runtime error:`. The label tells a reader
//! *which* stage rejected the program.
//!
//! # Positions
//!
//! Spans are byte offsets, but a reader counts characters. `é` is two
//! bytes in UTF-8 and one character on screen, so line and column numbers
//! are counted in characters, and so is the indentation of the caret. A
//! tab counts as one column, and under the source line the caret is
//! indented with a tab wherever the source line has one, so the two lines
//! expand to the same width in any terminal. A file with Windows line
//! endings (`\r\n`) shows its lines without the `\r`.
//!
//! One limit remains: a character that a terminal draws two columns wide
//! (most CJK characters and emoji) still counts as one, so a caret after
//! one sits a column early. Measuring display width needs Unicode width
//! tables, which Dijk Core does not carry.

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

    // Count the characters of the span that lie on its first line.
    let span_bytes = span.end.saturating_sub(span.start);
    let rest_of_line = &location.text[location.before.len()..];
    let width = rest_of_line
        .char_indices()
        .take_while(|(offset, _)| *offset < span_bytes)
        .count()
        .max(1);

    let mut rendered = header(file, &location, label, message);
    rendered.push_str(&marker_line(&location, width));
    let pad = " ".repeat(location.gutter());
    for note in notes {
        rendered.push_str(&format!("{pad} = {note}\n"));
    }
    rendered
}

/// The 1-based line and column of byte `offset` in `source`, as
/// diagnostics show them (`main.dk:2:18`).
pub fn line_and_column(source: &str, offset: usize) -> (usize, usize) {
    let location = locate(source, offset);
    (location.line, location.column)
}

/// Where an offset falls in the source, for display.
struct Location<'s> {
    /// 1-based line number.
    line: usize,
    /// 1-based column, counted in characters.
    column: usize,
    /// The full text of the line, without its line break (`\n` or `\r\n`).
    text: &'s str,
    /// The part of `text` before the offset.
    before: &'s str,
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
/// The indentation has one space per character before the problem, or a
/// tab where the source line has a tab, so the carets line up however
/// wide the terminal draws a tab.
fn marker_line(location: &Location, width: usize) -> String {
    let indent: String = location
        .before
        .chars()
        .map(|c| if c == '\t' { '\t' } else { ' ' })
        .collect();
    format!(
        "{pad} | {indent}{carets}\n",
        pad = " ".repeat(location.gutter()),
        carets = "^".repeat(width),
    )
}

/// Given a byte offset into `source`, find its 1-based line and column,
/// and the full text of the line it falls on.
fn locate(source: &str, offset: usize) -> Location<'_> {
    let offset = offset.min(source.len());
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

    // With Windows line endings, every line ends in `\r\n`. The `\r` is
    // part of the line break, not of the line.
    let full_line = &source[line_start..line_end];
    let text = full_line.strip_suffix('\r').unwrap_or(full_line);

    // Spans always start on a character boundary (the lexer makes sure of
    // it). Should one ever not, back up to the boundary before it rather
    // than fail.
    let mut prefix_len = (offset - line_start).min(text.len());
    while !text.is_char_boundary(prefix_len) {
        prefix_len -= 1;
    }
    let before = &text[..prefix_len];

    Location {
        line,
        column: before.chars().count() + 1,
        text,
        before,
    }
}
