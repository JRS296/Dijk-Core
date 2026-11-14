//! Tests for parsing. See `docs/parser.md` for the grammar these tests
//! verify against, and `docs/ast.md` for the AST shape they check.

// None of these modules are part of a library's public surface (dijk is a
// binary crate), so these tests include them directly by path, mirroring
// their layout as siblings under the crate root in src/main.rs.
#[path = "../src/ast/mod.rs"]
mod ast;
// Only the type checker renders with notes, so that part of the shared
// renderer goes unused here.
#[allow(dead_code)]
#[path = "../src/diagnostics.rs"]
mod diagnostics;
#[path = "../src/lexer/mod.rs"]
mod lexer;
#[path = "../src/parser/mod.rs"]
mod parser;

use ast::declaration::{Declaration, Function, Program};
use ast::expression::{BinaryOperator, Expression, UnaryOperator};
use ast::statement::Statement;
use ast::visit::{self, Visitor};
use parser::error::{ParseError, ParseErrorKind};

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

fn functions(program: &Program) -> Vec<&Function> {
    program
        .declarations
        .iter()
        .map(|d| match d {
            Declaration::Function(f) => f,
        })
        .collect()
}

fn single_function(source: &str) -> Function {
    let (program, errors) = parse_ok(source);
    assert!(errors.is_empty(), "unexpected parse errors: {errors:?}");
    assert_eq!(
        program.declarations.len(),
        1,
        "expected exactly one declaration"
    );
    match program.declarations.into_iter().next().unwrap() {
        Declaration::Function(f) => f,
    }
}

// --- basic programs ---

#[test]
fn empty_program_has_no_functions() {
    let (program, errors) = parse_ok("");
    assert!(errors.is_empty());
    assert!(program.declarations.is_empty());
}

#[test]
fn empty_function() {
    let f = single_function("fn main() {}");
    assert_eq!(f.name, "main");
    assert!(f.params.is_empty());
    assert!(f.return_type.is_none());
    assert!(f.body.is_empty());
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
    assert_eq!(f.body.len(), 1);
    assert!(matches!(
        &f.body[0],
        Statement::Return { value: Some(_), .. }
    ));
}

#[test]
fn multiple_functions() {
    let (program, errors) = parse_ok("fn a() {}\nfn b() {}\nfn c() {}\n");
    assert!(errors.is_empty());
    let names: Vec<&str> = functions(&program)
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    assert_eq!(names, vec!["a", "b", "c"]);
}

// --- variables ---

#[test]
fn let_without_type_annotation() {
    let f = single_function("fn main() {\n    let x = 10;\n}\n");
    match &f.body[0] {
        Statement::Let {
            mutable,
            name,
            type_annotation,
            value,
            ..
        } => {
            assert!(!mutable, "a plain `let` is immutable");
            assert_eq!(name, "x");
            assert!(type_annotation.is_none());
            assert!(matches!(value, Expression::Integer(10, _)));
        }
        other => panic!("expected Let, got {other:?}"),
    }
}

#[test]
fn let_with_type_annotation() {
    let f = single_function("fn main() {\n    let x: i32 = 10;\n    let y: i32 = x + 20;\n}\n");
    match &f.body[1] {
        Statement::Let {
            name,
            type_annotation,
            value,
            ..
        } => {
            assert_eq!(name, "y");
            assert_eq!(type_annotation.as_ref().unwrap().name, "i32");
            assert!(matches!(
                value,
                Expression::Binary {
                    operator: BinaryOperator::Add,
                    ..
                }
            ));
        }
        other => panic!("expected Let, got {other:?}"),
    }
}

#[test]
fn let_mut_declares_a_mutable_binding() {
    let f = single_function("fn main() {\n    let mut count = 0;\n}\n");
    match &f.body[0] {
        Statement::Let {
            mutable,
            name,
            type_annotation,
            ..
        } => {
            assert!(mutable);
            assert_eq!(name, "count");
            assert!(type_annotation.is_none());
        }
        other => panic!("expected Let, got {other:?}"),
    }
}

#[test]
fn let_mut_with_type_annotation() {
    let f = single_function("fn main() {\n    let mut total: i64 = 0;\n}\n");
    match &f.body[0] {
        Statement::Let {
            mutable,
            name,
            type_annotation,
            ..
        } => {
            assert!(mutable);
            assert_eq!(name, "total");
            assert_eq!(type_annotation.as_ref().unwrap().name, "i64");
        }
        other => panic!("expected Let, got {other:?}"),
    }
}

#[test]
fn a_let_records_where_its_name_was_written() {
    let source = "fn main() {\n    let a = 1;\n    let mut b = 2;\n}\n";
    let f = single_function(source);
    for (stmt, name) in f.body.iter().zip(["a", "b"]) {
        match stmt {
            Statement::Let { name_span, .. } => {
                assert_eq!(&source[name_span.start..name_span.end], name);
            }
            other => panic!("expected Let, got {other:?}"),
        }
    }
    // The statement's own span still covers all of it, `mut` included.
    let span = f.body[1].span();
    assert_eq!(&source[span.start..span.end], "let mut b = 2;");
}

#[test]
fn let_mut_without_a_name_is_an_error() {
    let errors = parse_errors("fn main() {\n    let mut = 1;\n}\n");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].to_string(), "expected identifier, found '='");
}

#[test]
fn mut_is_only_valid_directly_after_let() {
    // `mut` is not an expression, and `mut` cannot repeat.
    assert_eq!(
        parse_errors("fn main() {\n    mut x = 1;\n}\n")[0].to_string(),
        "expected expression, found 'mut'"
    );
    assert_eq!(
        parse_errors("fn main() {\n    let mut mut x = 1;\n}\n")[0].to_string(),
        "expected identifier, found 'mut'"
    );
}

// --- expressions and precedence ---

fn single_expr_statement(source: &str) -> Expression {
    let f = single_function(&format!("fn main() {{\n    {source};\n}}\n"));
    match f.body.into_iter().next().unwrap() {
        Statement::Expression { expr, .. } => expr,
        other => panic!("expected an expression statement, got {other:?}"),
    }
}

#[test]
fn addition_and_multiplication() {
    assert!(matches!(
        single_expr_statement("a + b"),
        Expression::Binary {
            operator: BinaryOperator::Add,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("a * b"),
        Expression::Binary {
            operator: BinaryOperator::Mul,
            ..
        }
    ));
}

#[test]
fn multiplication_binds_tighter_than_addition() {
    // a + b * c must parse as a + (b * c), not (a + b) * c.
    match single_expr_statement("a + b * c") {
        Expression::Binary {
            operator: BinaryOperator::Add,
            left,
            right,
            ..
        } => {
            assert!(matches!(*left, Expression::Identifier(ref n, _) if n == "a"));
            assert!(matches!(
                *right,
                Expression::Binary {
                    operator: BinaryOperator::Mul,
                    ..
                }
            ));
        }
        other => panic!("expected top-level Add, got {other:?}"),
    }
}

#[test]
fn parentheses_override_precedence() {
    // (a + b) * c must parse as (a + b) * c, with the addition grouped —
    // and, since the AST has no Grouping node, the left side is a plain
    // Binary(Add) directly, not wrapped in anything.
    match single_expr_statement("(a + b) * c") {
        Expression::Binary {
            operator: BinaryOperator::Mul,
            left,
            right,
            ..
        } => {
            assert!(matches!(*right, Expression::Identifier(ref n, _) if n == "c"));
            assert!(matches!(
                *left,
                Expression::Binary {
                    operator: BinaryOperator::Add,
                    ..
                }
            ));
        }
        other => panic!("expected top-level Mul, got {other:?}"),
    }
}

#[test]
fn parenthesized_and_unparenthesized_additions_produce_the_same_shape() {
    // The concrete illustration of "parse tree vs AST": once precedence
    // is resolved, parentheses leave no trace beyond their own span —
    // spans necessarily differ here (the source text differs by one
    // character), so this compares shape, not full equality.
    for source in ["(a + b)", "a + b"] {
        match single_expr_statement(source) {
            Expression::Binary {
                operator: BinaryOperator::Add,
                left,
                right,
                ..
            } => {
                assert!(matches!(*left, Expression::Identifier(ref n, _) if n == "a"));
                assert!(matches!(*right, Expression::Identifier(ref n, _) if n == "b"));
            }
            other => panic!("expected Binary(Add) for {source:?}, got {other:?}"),
        }
    }
}

#[test]
fn unary_minus_and_not() {
    assert!(matches!(
        single_expr_statement("-a"),
        Expression::Unary {
            operator: UnaryOperator::Neg,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("!a"),
        Expression::Unary {
            operator: UnaryOperator::Not,
            ..
        }
    ));
}

#[test]
fn comparisons() {
    assert!(matches!(
        single_expr_statement("x == y"),
        Expression::Binary {
            operator: BinaryOperator::Eq,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("x != y"),
        Expression::Binary {
            operator: BinaryOperator::NotEq,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("x < y"),
        Expression::Binary {
            operator: BinaryOperator::Lt,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("x <= y"),
        Expression::Binary {
            operator: BinaryOperator::Le,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("x > y"),
        Expression::Binary {
            operator: BinaryOperator::Gt,
            ..
        }
    ));
    assert!(matches!(
        single_expr_statement("x >= y"),
        Expression::Binary {
            operator: BinaryOperator::Ge,
            ..
        }
    ));
}

#[test]
fn boolean_literals() {
    assert!(matches!(
        single_expr_statement("true"),
        Expression::Boolean(true, _)
    ));
    assert!(matches!(
        single_expr_statement("false"),
        Expression::Boolean(false, _)
    ));
}

#[test]
fn integer_literals_are_parsed_to_values() {
    assert!(matches!(
        single_expr_statement("12345"),
        Expression::Integer(12345, _)
    ));
}

#[test]
fn integer_literal_out_of_range_is_an_error() {
    let errors = parse_errors("fn main() {\n    99999999999999999999;\n}\n");
    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0].kind,
        ParseErrorKind::InvalidIntegerLiteral { .. }
    ));
    assert_eq!(
        errors[0].to_string(),
        "integer literal '99999999999999999999' is too large: its value does not fit in 64 bits"
    );
}

#[test]
fn any_literal_that_fits_in_64_bits_parses() {
    // Including ones no signed type can hold: whether a value fits where
    // it is used is the type checker's question, not the parser's.
    assert!(matches!(
        single_expr_statement("9223372036854775808"),
        Expression::Integer(9_223_372_036_854_775_808, _)
    ));
    assert!(matches!(
        single_expr_statement("18446744073709551615"),
        Expression::Integer(u64::MAX, _)
    ));
    assert_eq!(
        parse_errors("fn main() {\n    18446744073709551616;\n}\n").len(),
        1
    );
}

#[test]
fn the_smallest_i64_parses_as_minus_applied_to_its_digits() {
    match single_expr_statement("-9223372036854775808") {
        Expression::Unary {
            operator: UnaryOperator::Neg,
            operand,
            ..
        } => assert!(matches!(
            *operand,
            Expression::Integer(9_223_372_036_854_775_808, _)
        )),
        other => panic!("expected Unary(-), got {other:?}"),
    }
}

#[test]
fn assignment_is_right_associative_and_low_precedence() {
    // x = y = 1 must parse as x = (y = 1).
    match single_expr_statement("x = y = 1") {
        Expression::Assign { name, value, .. } => {
            assert_eq!(name, "x");
            assert!(matches!(*value, Expression::Assign { .. }));
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
    match &f.body[0] {
        Statement::If {
            else_branch,
            then_branch,
            ..
        } => {
            assert!(else_branch.is_none());
            assert_eq!(then_branch.len(), 1);
        }
        other => panic!("expected If, got {other:?}"),
    }
}

#[test]
fn if_else_chain() {
    let f = single_function(
        "fn main() {\n    if x > 10 {\n        return 1;\n    } else if x > 0 {\n        return 2;\n    } else {\n        return 3;\n    }\n}\n",
    );
    match &f.body[0] {
        Statement::If {
            else_branch: Some(outer_else),
            ..
        } => {
            assert_eq!(
                outer_else.len(),
                1,
                "else-if desugars to a single-statement block"
            );
            match &outer_else[0] {
                Statement::If {
                    else_branch: Some(inner_else),
                    ..
                } => {
                    assert!(matches!(inner_else[0], Statement::Return { .. }));
                }
                other => panic!("expected the nested else-if as an If, got {other:?}"),
            }
        }
        other => panic!("expected If with an else branch, got {other:?}"),
    }
}

#[test]
fn while_statement() {
    let f = single_function("fn main() {\n    while x > 0 {\n        x = x - 1;\n    }\n}\n");
    match &f.body[0] {
        Statement::While { body, .. } => assert_eq!(body.len(), 1),
        other => panic!("expected While, got {other:?}"),
    }
}

// --- spans ---

#[test]
fn function_span_covers_the_whole_declaration() {
    let source = "fn add(a: i32, b: i32) -> i32 { return a + b; }";
    let f = single_function(source);
    assert_eq!(&source[f.span.start..f.span.end], source);
}

#[test]
fn parameter_and_type_spans_point_at_their_own_text() {
    let source = "fn add(a: i32, b: i32) -> i32 {}";
    let f = single_function(source);
    assert_eq!(
        &source[f.params[0].span.start..f.params[0].span.end],
        "a: i32"
    );
    assert_eq!(
        &source[f.params[0].ty.span.start..f.params[0].ty.span.end],
        "i32"
    );
    assert_eq!(
        &source[f.params[1].span.start..f.params[1].span.end],
        "b: i32"
    );
}

#[test]
fn expression_span_points_at_its_own_text_not_the_whole_statement() {
    let source = "fn main() {\n    return a + b;\n}\n";
    let f = single_function(source);
    match &f.body[0] {
        Statement::Return {
            value: Some(expr), ..
        } => {
            assert_eq!(&source[expr.span().start..expr.span().end], "a + b");
        }
        other => panic!("expected Return, got {other:?}"),
    }
}

#[test]
fn nested_binary_expression_spans_grow_outward() {
    // In `a + b * c`, the inner `b * c` span must be a strict subrange
    // of the outer `+` expression's span, not equal to it.
    match single_expr_statement("a + b * c") {
        Expression::Binary {
            right, span: outer, ..
        } => {
            let inner = right.span();
            assert!(inner.start > outer.start);
            assert_eq!(inner.end, outer.end);
        }
        other => panic!("expected top-level Binary, got {other:?}"),
    }
}

// --- calls ---

#[test]
fn call_with_no_arguments() {
    match single_expr_statement("foo()") {
        Expression::Call {
            callee, arguments, ..
        } => {
            assert_eq!(callee, "foo");
            assert!(arguments.is_empty());
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_with_arguments() {
    match single_expr_statement("add(1, 2 + 3)") {
        Expression::Call {
            callee, arguments, ..
        } => {
            assert_eq!(callee, "add");
            assert_eq!(arguments.len(), 2);
            assert!(matches!(arguments[0], Expression::Integer(1, _)));
            assert!(matches!(
                arguments[1],
                Expression::Binary {
                    operator: BinaryOperator::Add,
                    ..
                }
            ));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn call_binds_tighter_than_unary_minus() {
    // -foo() must mean -(foo()), not (-foo)() — the latter isn't even
    // expressible, since a callee must be a plain identifier.
    match single_expr_statement("-foo()") {
        Expression::Unary {
            operator: UnaryOperator::Neg,
            operand,
            ..
        } => {
            assert!(matches!(*operand, Expression::Call { .. }));
        }
        other => panic!("expected Unary(Neg) wrapping a Call, got {other:?}"),
    }
}

#[test]
fn nested_calls_as_arguments() {
    match single_expr_statement("outer(inner())") {
        Expression::Call {
            callee, arguments, ..
        } => {
            assert_eq!(callee, "outer");
            assert_eq!(arguments.len(), 1);
            assert!(matches!(&arguments[0], Expression::Call { callee, .. } if callee == "inner"));
        }
        other => panic!("expected Call, got {other:?}"),
    }
}

#[test]
fn identifier_not_followed_by_parens_is_not_a_call() {
    assert!(matches!(single_expr_statement("foo"), Expression::Identifier(ref n, _) if n == "foo"));
}

#[test]
fn call_missing_close_paren_is_an_error() {
    let errors = parse_errors("fn main() {\n    foo(1, 2;\n}\n");
    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0].kind,
        ParseErrorKind::UnexpectedToken {
            found: lexer::token::TokenKind::Semicolon,
            ..
        }
    ));
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
    let fns = functions(&program);
    assert_eq!(fns.len(), 1);
    assert_eq!(fns[0].name, "ok");
}

// --- traversal ---

/// Counts every function visited, deferring everything else to the
/// default (walking) implementation — confirming a parsed program is
/// actually walkable by `ast::visit::Visitor`, not just constructible.
struct FunctionCounter(usize);

impl Visitor for FunctionCounter {
    fn visit_function(&mut self, function: &Function) {
        self.0 += 1;
        visit::walk_function(self, function);
    }
}

#[test]
fn visitor_walks_every_function_in_a_parsed_program() {
    let (program, errors) = parse_ok("fn a() {}\nfn b() { let x = 1 + 2; }\n");
    assert!(errors.is_empty());
    let mut counter = FunctionCounter(0);
    counter.visit_program(&program);
    assert_eq!(counter.0, 2);
}
