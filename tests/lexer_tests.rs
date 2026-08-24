//! Tests for lexical analysis. See `docs/lexer.md` for the specification
//! these tests verify against.

// The lexer module is not part of the library's public surface (dijk is a
// binary crate), so these tests include it directly by path.
#[path = "../src/lexer/mod.rs"]
mod lexer;

use lexer::error::LexErrorKind;
use lexer::token::TokenKind;
use lexer::tokenize;

fn kinds(source: &str) -> Vec<TokenKind> {
    let (tokens, errors) = tokenize(source);
    assert!(errors.is_empty(), "unexpected lex errors: {errors:?}");
    tokens.into_iter().map(|t| t.kind).collect()
}

#[test]
fn empty_source_is_just_eof() {
    assert_eq!(kinds(""), vec![TokenKind::Eof]);
}

#[test]
fn identifiers() {
    let (tokens, errors) = tokenize("foo my_function User _private");
    assert!(errors.is_empty());
    let lexemes: Vec<&str> = tokens.iter().map(|t| t.lexeme.as_str()).collect();
    assert_eq!(lexemes, vec!["foo", "my_function", "User", "_private", ""]);
    for t in &tokens[..4] {
        assert_eq!(t.kind, TokenKind::Identifier);
    }
}

#[test]
fn integers() {
    let (tokens, _) = tokenize("0 42 12345");
    let lexemes: Vec<&str> = tokens.iter().map(|t| t.lexeme.as_str()).collect();
    assert_eq!(lexemes, vec!["0", "42", "12345", ""]);
    for t in &tokens[..3] {
        assert_eq!(t.kind, TokenKind::Integer);
    }
}

#[test]
fn string_literal() {
    let (tokens, errors) = tokenize("\"Hello, world!\"");
    assert!(errors.is_empty());
    assert_eq!(tokens[0].kind, TokenKind::String);
    assert_eq!(tokens[0].lexeme, "Hello, world!");
}

#[test]
fn keywords_are_not_identifiers() {
    use TokenKind::*;
    assert_eq!(
        kinds("fn let mut if else while for return struct enum true false"),
        vec![Fn, Let, Mut, If, Else, While, For, Return, Struct, Enum, True, False, Eof]
    );
}

#[test]
fn primitive_type_names_are_plain_identifiers() {
    // i32 is not a keyword at the lexical stage — see docs/lexer.md.
    assert_eq!(kinds("i32"), vec![TokenKind::Identifier, TokenKind::Eof]);
}

#[test]
fn single_character_operators() {
    use TokenKind::*;
    assert_eq!(
        kinds("+ - * / % = ! < > & |"),
        vec![Plus, Minus, Star, Slash, Percent, Equal, Bang, Less, Greater, Amp, Pipe, Eof]
    );
}

#[test]
fn multi_character_operators() {
    use TokenKind::*;
    assert_eq!(
        kinds("== != <= >= ->"),
        vec![EqualEqual, BangEqual, LessEqual, GreaterEqual, Arrow, Eof]
    );
}

#[test]
fn minus_is_not_swallowed_by_arrow() {
    use TokenKind::*;
    assert_eq!(kinds("- >"), vec![Minus, Greater, Eof]);
}

#[test]
fn punctuation() {
    use TokenKind::*;
    assert_eq!(
        kinds("( ) { } [ ] , : ; ."),
        vec![
            LeftParen, RightParen, LeftBrace, RightBrace, LeftBracket, RightBracket, Comma,
            Colon, Semicolon, Dot, Eof
        ]
    );
}

#[test]
fn whitespace_is_insignificant() {
    assert_eq!(kinds("  \t\r\n  "), vec![TokenKind::Eof]);
}

#[test]
fn line_comments_are_discarded() {
    assert_eq!(
        kinds("let x = 1; // this is a comment\nlet y = 2;"),
        kinds("let x = 1;\nlet y = 2;")
    );
}

#[test]
fn comment_at_end_of_file_with_no_trailing_newline() {
    assert_eq!(kinds("let x = 1; // trailing"), kinds("let x = 1;"));
}

#[test]
fn unexpected_character_is_an_error_not_a_panic() {
    let (_, errors) = tokenize("let x = 42 @ 10;");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::UnexpectedCharacter('@'));
}

#[test]
fn lexing_continues_after_an_error() {
    let (tokens, errors) = tokenize("@ let x = 1;");
    assert_eq!(errors.len(), 1);
    // The bad character is skipped, but everything after it still lexes.
    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Let));
}

#[test]
fn unterminated_string_is_an_error() {
    let (_, errors) = tokenize("\"never closed");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::UnterminatedString);
}

#[test]
fn unterminated_string_does_not_swallow_the_next_line() {
    let (_, errors) = tokenize("\"never closed\nlet x = 1;");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, LexErrorKind::UnterminatedString);
}

#[test]
fn token_spans_point_at_the_correct_source_range() {
    let (tokens, _) = tokenize("  foo");
    let foo = &tokens[0];
    assert_eq!(foo.span.start, 2);
    assert_eq!(foo.span.end, 5);
}

#[test]
fn diagnostic_rendering_points_at_the_offending_character() {
    let source = "let x = 42 @ 10;";
    let (_, errors) = tokenize(source);
    let rendered = errors[0].render("example.dk", source);
    assert!(rendered.contains("unexpected character '@'"));
    assert!(rendered.contains("example.dk:1:12"));
    assert!(rendered.contains("let x = 42 @ 10;"));
}

#[test]
fn full_function_example() {
    use TokenKind::*;
    let source = "fn add(a: i32, b: i32) -> i32 {\n    return a + b;\n}\n";
    assert_eq!(
        kinds(source),
        vec![
            Fn, Identifier, LeftParen, Identifier, Colon, Identifier, Comma, Identifier, Colon,
            Identifier, RightParen, Arrow, Identifier, LeftBrace, Return, Identifier, Plus,
            Identifier, Semicolon, RightBrace, Eof,
        ]
    );
}
