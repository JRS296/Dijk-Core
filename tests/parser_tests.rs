//! Tests for parsing. See `docs/parser.md` for the grammar these tests
//! verify against.

// None of these modules are part of a library's public surface (dijk is a
// binary crate), so these tests include them directly by path, mirroring
// their layout as siblings under the crate root in src/main.rs.
#[path = "../src/diagnostics.rs"]
mod diagnostics;
#[path = "../src/lexer/mod.rs"]
mod lexer;
#[path = "../src/parser/mod.rs"]
mod parser;

use parser::error::{ParseError, ParseErrorKind};
use parser::syntax::{BinaryOp, ElseBranch, Expr, Program, Stmt};

/// Lex and parse `source`, asserting the lexer itself produced no errors
/// (a failure there would mean the test fixture is broken, not the parser).
fn parse_ok(source: &str) -> (Program, Vec<ParseError>) {
    let (tokens, lex_errors) = lexer::tokenize(source);
    assert!(
        lex_errors.is_empty(),
        "unexpected lex errors: {lex_errors:?}"
    );
    parser::parse(tokens)
}

fn single_function(source: &str) -> parser::syntax::Function {
    let (program, errors) = parse_ok(source);
    assert!(errors.is_empty(), "unexpected parse errors: {errors:?}");
    assert_eq!(program.functions.len(), 1, "expected exactly one function");
    program.functions.into_iter().next().unwrap()
}

// --- basic programs ---

#[test]
fn empty_program_has_no_functions() {
    let (program, errors) = parse_ok("");
    assert!(errors.is_empty());
    assert!(program.functions.is_empty());
}

#[test]
fn empty_function() {
    let f = single_function("fn main() {}");
    assert_eq!(f.name, "main");
    assert!(f.params.is_empty());
    assert!(f.return_type.is_none());
    assert!(f.body.statements.is_empty());
}

// --- functions ---

#[test]
fn function_with_parameters_and_return_type() {
    let f = single_function("fn add(a: i32, b: i32) -> i32 {\n    return a + b;\n}\n");
    assert_eq!(f.name, "add");
    assert_eq!(f.params.len(), 2);
    assert_eq!(f.params[0].name, "a");
    assert_eq!(f.params[0].ty.name, "i32");
    assert_eq!(f.params[1].name, "b");
    assert_eq!(f.return_type.as_ref().unwrap().name, "i32");
    assert_eq!(f.body.statements.len(), 1);
    assert!(matches!(
        &f.body.statements[0],
        Stmt::Return { value: Some(_), .. }
    ));
}

#[test]
fn multiple_functions() {
    let (program, errors) = parse_ok("fn a() {}\nfn b() {}\nfn c() {}\n");
    assert!(errors.is_empty());
    let names: Vec<&str> = program.functions.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b", "c"]);
}

// --- variables ---

#[test]
fn let_without_type_annotation() {
    let f = single_function("fn main() {\n    let x = 10;\n}\n");
    match &f.body.statements[0] {
        Stmt::Let {
            name, ty, value, ..
        } => {
            assert_eq!(name, "x");
            assert!(ty.is_none());
            assert!(matches!(value, Expr::Integer(lexeme, _) if lexeme == "10"));
        }
        other => panic!("expected Let, got {other:?}"),
    }
}

#[test]
fn let_with_type_annotation() {
    let f = single_function("fn main() {\n    let x: i32 = 10;\n    let y: i32 = x + 20;\n}\n");
    match &f.body.statements[1] {
        Stmt::Let {
            name, ty, value, ..
        } => {
            assert_eq!(name, "y");
            assert_eq!(ty.as_ref().unwrap().name, "i32");
            assert!(matches!(
                value,
                Expr::Binary {
                    op: BinaryOp::Add,
                    ..
                }
            ));
        }
        other => panic!("expected Let, got {other:?}"),
    }
}

// --- expressions and precedence ---

fn single_expr_statement(source: &str) -> Expr {
    let f = single_function(&format!("fn main() {{\n    {source};\n}}\n"));
    match f.body.statements.into_iter().next().unwrap() {
        Stmt::Expression { expr, .. } => expr,
        other => panic!("expected an expression statement, got {other:?}"),
    }
}

#[test]
fn addition_and_multiplication() {
    assert!(matches!(
        single_expr_statement("a + b"),
        Expr::Binary {
            op: BinaryOp::Add,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("a * b"),
        Expr::Binary {
            op: BinaryOp::Mul,
            ..
        }
    ));
}

#[test]
fn multiplication_binds_tighter_than_addition() {
    // a + b * c must parse as a + (b * c), not (a + b) * c.
    match single_expr_statement("a + b * c") {
        Expr::Binary {
            op: BinaryOp::Add,
            left,
            right,
            ..
        } => {
            assert!(matches!(*left, Expr::Identifier(ref n, _) if n == "a"));
            assert!(matches!(
                *right,
                Expr::Binary {
                    op: BinaryOp::Mul,
                    ..
                }
            ));
        }
        other => panic!("expected top-level Add, got {other:?}"),
    }
}

#[test]
fn parentheses_override_precedence() {
    // (a + b) * c must parse as (a + b) * c, with the addition grouped.
    match single_expr_statement("(a + b) * c") {
        Expr::Binary {
            op: BinaryOp::Mul,
            left,
            right,
            ..
        } => {
            assert!(matches!(*right, Expr::Identifier(ref n, _) if n == "c"));
            match *left {
                Expr::Grouping(inner, _) => {
                    assert!(matches!(
                        *inner,
                        Expr::Binary {
                            op: BinaryOp::Add,
                            ..
                        }
                    ));
                }
                other => panic!("expected a grouping on the left, got {other:?}"),
            }
        }
        other => panic!("expected top-level Mul, got {other:?}"),
    }
}

#[test]
fn unary_minus_and_not() {
    assert!(matches!(
        single_expr_statement("-a"),
        Expr::Unary {
            op: parser::syntax::UnaryOp::Neg,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("!a"),
        Expr::Unary {
            op: parser::syntax::UnaryOp::Not,
            ..
        }
    ));
}

#[test]
fn comparisons() {
    assert!(matches!(
        single_expr_statement("x == y"),
        Expr::Binary {
            op: BinaryOp::Eq,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("x != y"),
        Expr::Binary {
            op: BinaryOp::NotEq,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("x < y"),
        Expr::Binary {
            op: BinaryOp::Lt,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("x <= y"),
        Expr::Binary {
            op: BinaryOp::Le,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("x > y"),
        Expr::Binary {
            op: BinaryOp::Gt,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("x >= y"),
        Expr::Binary {
            op: BinaryOp::Ge,
            ..
        }
    ));
}

#[test]
fn boolean_literals() {
    assert!(matches!(single_expr_statement("true"), Expr::Bool(true, _)));
    assert!(matches!(
        single_expr_statement("false"),
        Expr::Bool(false, _)
    ));
}

#[test]
fn assignment_is_right_associative_and_low_precedence() {
    // x = y = 1 must parse as x = (y = 1).
    match single_expr_statement("x = y = 1") {
        Expr::Assign { name, value, .. } => {
            assert_eq!(name, "x");
            assert!(matches!(*value, Expr::Assign { .. }));
        }
        other => panic!("expected Assign, got {other:?}"),
    }
}

#[test]
fn invalid_assignment_target_is_an_error() {
    let f_source = "fn main() {\n    1 = 2;\n}\n";
    let (tokens, lex_errors) = lexer::tokenize(f_source);
    assert!(lex_errors.is_empty());
    let (_, errors) = parser::parse(tokens);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, ParseErrorKind::InvalidAssignmentTarget);
}

// --- control flow ---

#[test]
fn if_statement_without_else() {
    let f = single_function("fn main() {\n    if x > 10 {\n        return x;\n    }\n}\n");
    match &f.body.statements[0] {
        Stmt::If {
            else_branch,
            then_block,
            ..
        } => {
            assert!(else_branch.is_none());
            assert_eq!(then_block.statements.len(), 1);
        }
        other => panic!("expected If, got {other:?}"),
    }
}

#[test]
fn if_else_chain() {
    let f = single_function(
        "fn main() {\n    if x > 10 {\n        return 1;\n    } else if x > 0 {\n        return 2;\n    } else {\n        return 3;\n    }\n}\n",
    );
    match &f.body.statements[0] {
        Stmt::If {
            else_branch: Some(ElseBranch::If(nested)),
            ..
        } => match nested.as_ref() {
            Stmt::If {
                else_branch: Some(ElseBranch::Block(_)),
                ..
            } => {}
            other => panic!("expected nested If with a Block else, got {other:?}"),
        },
        other => panic!("expected If with an If else-branch, got {other:?}"),
    }
}

#[test]
fn while_statement() {
    let f = single_function("fn main() {\n    while x > 0 {\n        x = x - 1;\n    }\n}\n");
    match &f.body.statements[0] {
        Stmt::While { body, .. } => assert_eq!(body.statements.len(), 1),
        other => panic!("expected While, got {other:?}"),
    }
}

// --- errors ---

fn parse_errors(source: &str) -> Vec<ParseError> {
    let (tokens, lex_errors) = lexer::tokenize(source);
    assert!(
        lex_errors.is_empty(),
        "unexpected lex errors: {lex_errors:?}"
    );
    parser::parse(tokens).1
}

#[test]
fn missing_close_paren_in_parameter_list() {
    let errors = parse_errors("fn main( {\n}\n");
    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0].kind,
        ParseErrorKind::UnexpectedToken {
            found: lexer::token::TokenKind::LeftBrace,
            ..
        }
    ));
}

#[test]
fn missing_close_brace() {
    let errors = parse_errors("fn main() {\n    return 1;\n");
    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0].kind,
        ParseErrorKind::UnexpectedToken {
            found: lexer::token::TokenKind::Eof,
            ..
        }
    ));
}

#[test]
fn missing_semicolon() {
    let errors = parse_errors("fn main() {\n    let x = 1\n}\n");
    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0].kind,
        ParseErrorKind::UnexpectedToken { .. }
    ));
}

#[test]
fn invalid_expression_reports_the_offending_token() {
    let errors = parse_errors("fn main() {\n    let x = ;\n}\n");
    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0].kind,
        ParseErrorKind::UnexpectedToken {
            found: lexer::token::TokenKind::Semicolon,
            ..
        }
    ));
}

#[test]
fn invalid_declaration_at_top_level() {
    let errors = parse_errors("let x = 1;\n");
    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0].kind,
        ParseErrorKind::UnexpectedToken {
            found: lexer::token::TokenKind::Let,
            ..
        }
    ));
}

#[test]
fn unexpected_token_in_expression_position_does_not_panic() {
    // '+' lexes fine but cannot start an expression (only '-' and '!' can
    // lead a unary expression) — this should be a clean diagnostic, not
    // a parser crash.
    let errors = parse_errors("fn main() {\n    + 1;\n}\n");
    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0].kind,
        ParseErrorKind::UnexpectedToken {
            found: lexer::token::TokenKind::Plus,
            ..
        }
    ));
}

#[test]
fn multiple_statement_errors_are_all_reported() {
    let errors = parse_errors("fn foo(a: i32) {\n    let x = ;\n    return\n}\n");
    assert_eq!(
        errors.len(),
        2,
        "expected both broken statements to be reported: {errors:?}"
    );
}

#[test]
fn parse_error_rendering_points_at_the_offending_token() {
    let source = "fn main() {\n    let x = ;\n}\n";
    let errors = parse_errors(source);
    let rendered = errors[0].render("example.dk", source);
    assert!(rendered.contains("expected expression, found ';'"));
    assert!(rendered.contains("example.dk:2:13"));
}

#[test]
fn lexer_and_parser_diagnostics_share_the_same_rendering_shape() {
    // Both error types delegate to crate::diagnostics::render, so a
    // lexical error and a parse error should look like the same kind of
    // thing to a programmer reading them.
    let source = "let x = 42 @ 10;";
    let (_, lex_errors) = lexer::tokenize(source);
    let rendered = lex_errors[0].render("example.dk", source);
    assert!(rendered.starts_with("error: "));
    assert!(rendered.contains(" --> example.dk:1:12"));
}

#[test]
fn a_broken_function_does_not_prevent_the_next_one_from_parsing() {
    let (program, errors) = parse_ok("fn broken(\nfn ok() {}\n");
    assert_eq!(errors.len(), 1);
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "ok");
}
