//! Unit tests for the `types` module (`crate::types`): the building blocks
//! of the type checker, one section per piece, added as each piece is.
//!
//! Not to be confused with `tests/type_tests/`, the language-level suite:
//! that one asks "does Dijk accept or reject this program, and why?", one
//! file per typing rule. This one asks whether each *piece* of the
//! implementation (the `Type` representation, annotation resolution, and
//! so on) behaves as designed.

// None of these modules are part of a library's public surface (dijk is a
// binary crate), so these tests include them directly by path, mirroring
// their layout as siblings under the crate root in src/main.rs. The lexer,
// parser and semantic analysis are here only to turn source text into a
// checked AST; their own diagnostic rendering goes unused, hence the
// `allow`s.
#[path = "../src/ast/mod.rs"]
mod ast;
#[path = "../src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../src/lexer/mod.rs"]
mod lexer;
#[allow(dead_code)]
#[path = "../src/parser/mod.rs"]
mod parser;
#[allow(dead_code)]
#[path = "../src/semantic/mod.rs"]
mod semantic;
#[path = "../src/types/mod.rs"]
mod types;

use ast::declaration::Program;
use ast::expression::{BinaryOperator, Expression, UnaryOperator};
use ast::visit::{self, Visitor};
use lexer::token::Span;
use types::env::TypeEnvironment;
use types::error::{TypeError, TypeErrorKind};
use types::rules;
use types::ty::{self, Type};
use types::TypedProgram;

/// Lex and parse `source`, asserting both stages accept it.
fn parse_syntax(source: &str) -> Program {
    let (tokens, lex_errors) = lexer::tokenize(source);
    assert!(
        lex_errors.is_empty(),
        "unexpected lex errors: {lex_errors:?}"
    );
    let (program, parse_errors) = parser::parse(tokens);
    assert!(
        parse_errors.is_empty(),
        "unexpected parse errors: {parse_errors:?}"
    );
    program
}

/// Lex, parse and analyze `source`, asserting that every stage before the
/// type checker accepts it: a failure there would mean the fixture is
/// broken, not the type checker.
fn parse(source: &str) -> Program {
    let program = parse_syntax(source);
    let semantic_errors = semantic::analyze(&program);
    assert!(
        semantic_errors.is_empty(),
        "unexpected semantic errors: {semantic_errors:?}"
    );
    program
}

/// What the checker learned about `source`, asserting it found no errors.
fn typed(source: &str) -> TypedProgram {
    match types::check(&parse(source)) {
        Ok(typed) => typed,
        Err(errors) => panic!("unexpected type errors: {errors:?}"),
    }
}

/// The type errors in `source`, asserting there is at least one.
fn type_errors(source: &str) -> Vec<TypeError> {
    match types::check(&parse(source)) {
        Ok(_) => panic!("expected type errors, but the program type-checks"),
        Err(errors) => errors,
    }
}

fn error_kinds(source: &str) -> Vec<TypeErrorKind> {
    type_errors(source).into_iter().map(|e| e.kind).collect()
}

/// The span of the first occurrence of `snippet` in `source`.
fn span_of(source: &str, snippet: &str) -> Span {
    let start = source
        .find(snippet)
        .unwrap_or_else(|| panic!("`{snippet}` does not occur in the fixture"));
    Span::new(start, start + snippet.len())
}

/// The type the checker recorded for the expression written as the first
/// occurrence of `snippet` in `source`.
fn type_at(source: &str, snippet: &str) -> Option<Type> {
    typed(source)
        .expression_types
        .get(&span_of(source, snippet))
        .copied()
}

/// `body` as the body of a `main` that returns nothing.
fn in_main(body: &str) -> String {
    format!("fn main() {{\n{body}\n}}\n")
}

/// A type annotation as the parser would build it, e.g. the `i32` in
/// `let x: i32 = 1;`.
fn annotation(name: &str) -> ast::types::Type {
    ast::types::Type {
        name: name.to_string(),
        span: Span::new(7, 7 + name.len()),
    }
}

// --- the type representation ---

#[test]
fn a_type_is_equal_to_itself() {
    assert_eq!(Type::I32, Type::I32);
    assert_eq!(Type::I64, Type::I64);
    assert_eq!(Type::Bool, Type::Bool);
    assert_eq!(Type::String, Type::String);
    assert_eq!(Type::Unit, Type::Unit);
}

#[test]
fn different_types_are_not_equal() {
    // Both are integers, but they are different types.
    assert_ne!(Type::I32, Type::I64);
    assert_ne!(Type::Bool, Type::String);
    assert_ne!(Type::I32, Type::Bool);
    assert_ne!(Type::String, Type::Unit);
}

#[test]
fn types_display_as_they_are_written_in_dijk() {
    assert_eq!(Type::I32.to_string(), "i32");
    assert_eq!(Type::I64.to_string(), "i64");
    assert_eq!(Type::Bool.to_string(), "bool");
    assert_eq!(Type::String.to_string(), "string");
    assert_eq!(Type::Unit.to_string(), "unit");
}

#[test]
fn only_i32_and_i64_are_integers() {
    assert!(Type::I32.is_integer());
    assert!(Type::I64.is_integer());
    assert!(!Type::Bool.is_integer());
    assert!(!Type::String.is_integer());
    assert!(!Type::Unit.is_integer());
}

// --- compatibility ---

#[test]
fn compatibility_is_equality() {
    assert!(Type::compatible(Type::I32, Type::I32));
    assert!(Type::compatible(Type::String, Type::String));
    // No implicit conversions, not even the "safe" widening one.
    assert!(!Type::compatible(Type::I64, Type::I32));
    assert!(!Type::compatible(Type::I32, Type::I64));
    assert!(!Type::compatible(Type::Bool, Type::I32));
}

#[test]
fn the_internal_error_type_is_compatible_with_everything() {
    // So that an expression already reported as wrong is not reported
    // again by whatever uses it.
    for other in [Type::I32, Type::I64, Type::Bool, Type::String, Type::Unit] {
        assert!(Type::compatible(other, Type::Error));
        assert!(Type::compatible(Type::Error, other));
    }
}

// --- resolving annotations ---

#[test]
fn the_four_writable_type_names_resolve() {
    assert_eq!(ty::resolve(&annotation("i32")), Ok(Type::I32));
    assert_eq!(ty::resolve(&annotation("i64")), Ok(Type::I64));
    assert_eq!(ty::resolve(&annotation("bool")), Ok(Type::Bool));
    assert_eq!(ty::resolve(&annotation("string")), Ok(Type::String));
}

#[test]
fn an_unknown_type_name_is_a_type_error_at_the_annotation() {
    let error = ty::resolve(&annotation("banana")).unwrap_err();
    assert_eq!(
        error.kind,
        TypeErrorKind::UnknownType {
            name: "banana".to_string()
        }
    );
    assert_eq!(error.span, Span::new(7, 13));
}

#[test]
fn unit_cannot_be_written_in_an_annotation() {
    // `unit` is a type the checker uses, but not a name a program can use.
    assert_eq!(
        ty::resolve(&annotation("unit")).unwrap_err().kind,
        TypeErrorKind::UnknownType {
            name: "unit".to_string()
        }
    );
}

#[test]
fn type_names_are_case_sensitive() {
    assert!(ty::resolve(&annotation("I32")).is_err());
    assert!(ty::resolve(&annotation("String")).is_err());
}

#[test]
fn type_errors_render_with_their_own_label() {
    let source = "let x: banana = 1;";
    let error = ty::resolve(&annotation("banana")).unwrap_err();
    let rendered = error.render("example.dk", source);
    assert!(rendered.starts_with("type error: unknown type `banana`\n"));
    assert!(rendered.contains(" --> example.dk:1:8"));
    assert_eq!(error.to_string(), "unknown type `banana`");
}

// --- literals ---

#[test]
fn an_integer_literal_defaults_to_i32() {
    assert_eq!(type_at(&in_main("let x = 42;"), "42"), Some(Type::I32));
}

#[test]
fn a_type_is_looked_up_by_the_expression_node_itself() {
    // Later stages hold AST nodes, not source snippets: `type_of` finds an
    // expression's type from the node.
    let program = parse(&in_main("let x = 42;"));
    let typed = types::check(&program).unwrap();
    let ast::declaration::Declaration::Function(main) = &program.declarations[0];
    let ast::statement::Statement::Let { value, .. } = &main.body[0] else {
        panic!("expected a let statement");
    };
    assert_eq!(typed.type_of(value), Some(Type::I32));
}

#[test]
fn boolean_literals_are_bool() {
    let source = in_main("let t = true;\nlet f = false;");
    assert_eq!(type_at(&source, "true"), Some(Type::Bool));
    assert_eq!(type_at(&source, "false"), Some(Type::Bool));
}

#[test]
fn a_string_literal_is_a_string() {
    // The literal's span includes its quotes.
    assert_eq!(
        type_at(&in_main("let s = \"hello\";"), "\"hello\""),
        Some(Type::String)
    );
}

#[test]
fn literals_are_typed_wherever_they_appear() {
    let source =
        in_main("if true {\n    let a = 1;\n}\nwhile false {\n    let b = \"s\";\n}\n7;\nreturn;");
    let typed = typed(&source);
    for (snippet, ty) in [
        ("true", Type::Bool),
        ("1", Type::I32),
        ("false", Type::Bool),
        ("\"s\"", Type::String),
        ("7", Type::I32),
    ] {
        assert_eq!(
            typed.expression_types.get(&span_of(&source, snippet)),
            Some(&ty),
            "for `{snippet}`"
        );
    }
}

// --- literal ranges ---

fn out_of_range(value: i64, ty: Type) -> TypeErrorKind {
    TypeErrorKind::IntegerLiteralOutOfRange { value, ty }
}

#[test]
fn the_largest_i32_literal_fits() {
    assert_eq!(
        type_at(&in_main("let x = 2147483647;"), "2147483647"),
        Some(Type::I32)
    );
}

#[test]
fn one_more_than_the_largest_i32_is_out_of_range() {
    let source = in_main("let x = 2147483648;");
    let errors = type_errors(&source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, out_of_range(2147483648, Type::I32));
    assert_eq!(errors[0].span, span_of(&source, "2147483648"));
}

#[test]
fn the_smallest_i32_literal_fits_because_its_minus_sign_is_part_of_it() {
    // Parsed as `-` applied to 2147483648, which alone would not fit.
    let source = in_main("let x = -2147483648;");
    assert_eq!(type_at(&source, "-2147483648"), Some(Type::I32));
    // The digits inside get the literal's type too.
    assert_eq!(type_at(&source, "2147483648"), Some(Type::I32));
}

#[test]
fn one_less_than_the_smallest_i32_is_out_of_range() {
    let source = in_main("let x = -2147483649;");
    let errors = type_errors(&source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, out_of_range(-2147483649, Type::I32));
    // Reported at the whole literal, minus sign included.
    assert_eq!(errors[0].span, span_of(&source, "-2147483649"));
}

#[test]
fn every_out_of_range_literal_is_reported_in_source_order() {
    assert_eq!(
        error_kinds(&in_main(
            "let a = 5000000000;\nlet b = 1;\nlet c = -9000000000;"
        )),
        vec![
            out_of_range(5000000000, Type::I32),
            out_of_range(-9000000000, Type::I32),
        ]
    );
}

#[test]
fn range_errors_render_as_type_errors() {
    let source = in_main("let x = 3000000000;");
    let rendered = type_errors(&source)[0].render("example.dk", &source);
    assert!(rendered
        .starts_with("type error: integer literal `3000000000` is out of range for `i32`\n"));
    assert!(rendered.contains(" --> example.dk:2:9"));
}

// --- the side table ---

/// Collects the span of every expression in a program.
#[derive(Default)]
struct ExpressionSpans(Vec<Span>);

impl Visitor for ExpressionSpans {
    fn visit_expression(&mut self, expr: &Expression) {
        self.0.push(expr.span());
        visit::walk_expression(self, expr);
    }
}

#[test]
fn no_two_expressions_share_a_span() {
    // The side table is keyed by span, which is only sound if a span
    // identifies one expression. Nested operators, parentheses, unary
    // chains, calls and assignments are where two nodes could collide.
    // (Only the shape matters here, so the program is not analyzed.)
    let program = parse_syntax(
        "fn f(a: i32, b: i32) -> i32 {\n    return (a + b) * -(-a) / f((a), (b + 1));\n}\n\nfn main() {\n    let x = f(1, 2) == --3;\n    let y = ((1));\n    if !(x == true) {\n        x = y = (f(y, y));\n    }\n}\n",
    );
    let mut spans = ExpressionSpans::default();
    spans.visit_program(&program);
    let mut sorted = spans.0.clone();
    sorted.sort_by_key(|span| (span.start, span.end));
    sorted.dedup();
    assert_eq!(sorted.len(), spans.0.len());
}

// --- the operator rules table ---

#[test]
fn addition_is_defined_for_same_type_integers_and_for_strings() {
    let add = |l, r| rules::binary_result(BinaryOperator::Add, l, r);
    assert_eq!(add(Type::I32, Type::I32), Some(Type::I32));
    assert_eq!(add(Type::I64, Type::I64), Some(Type::I64));
    assert_eq!(add(Type::String, Type::String), Some(Type::String));
    assert_eq!(add(Type::I32, Type::I64), None);
    assert_eq!(add(Type::I64, Type::I32), None);
    assert_eq!(add(Type::I32, Type::String), None);
    assert_eq!(add(Type::Bool, Type::Bool), None);
}

#[test]
fn other_arithmetic_is_integers_only() {
    for operator in [
        BinaryOperator::Sub,
        BinaryOperator::Mul,
        BinaryOperator::Div,
    ] {
        assert_eq!(
            rules::binary_result(operator, Type::I64, Type::I64),
            Some(Type::I64)
        );
        assert_eq!(
            rules::binary_result(operator, Type::String, Type::String),
            None
        );
        assert_eq!(rules::binary_result(operator, Type::I32, Type::I64), None);
    }
}

#[test]
fn ordering_compares_same_type_integers_to_a_bool() {
    for operator in [
        BinaryOperator::Lt,
        BinaryOperator::Le,
        BinaryOperator::Gt,
        BinaryOperator::Ge,
    ] {
        assert_eq!(
            rules::binary_result(operator, Type::I32, Type::I32),
            Some(Type::Bool)
        );
        assert_eq!(
            rules::binary_result(operator, Type::String, Type::String),
            None
        );
        assert_eq!(rules::binary_result(operator, Type::Bool, Type::Bool), None);
    }
}

#[test]
fn equality_needs_two_operands_of_the_same_type_other_than_unit() {
    for operator in [BinaryOperator::Eq, BinaryOperator::NotEq] {
        for ty in [Type::I32, Type::I64, Type::Bool, Type::String] {
            assert_eq!(rules::binary_result(operator, ty, ty), Some(Type::Bool));
        }
        assert_eq!(rules::binary_result(operator, Type::Unit, Type::Unit), None);
        assert_eq!(rules::binary_result(operator, Type::I32, Type::I64), None);
        assert_eq!(rules::binary_result(operator, Type::I32, Type::Bool), None);
    }
}

#[test]
fn unary_minus_negates_integers_and_not_negates_booleans() {
    assert_eq!(
        rules::unary_result(UnaryOperator::Neg, Type::I32),
        Some(Type::I32)
    );
    assert_eq!(
        rules::unary_result(UnaryOperator::Neg, Type::I64),
        Some(Type::I64)
    );
    assert_eq!(rules::unary_result(UnaryOperator::Neg, Type::Bool), None);
    assert_eq!(
        rules::unary_result(UnaryOperator::Not, Type::Bool),
        Some(Type::Bool)
    );
    assert_eq!(rules::unary_result(UnaryOperator::Not, Type::I32), None);
}

#[test]
fn no_operator_accepts_unit() {
    for operator in [
        BinaryOperator::Add,
        BinaryOperator::Sub,
        BinaryOperator::Lt,
        BinaryOperator::Eq,
    ] {
        assert_eq!(rules::binary_result(operator, Type::Unit, Type::Unit), None);
    }
    assert_eq!(rules::unary_result(UnaryOperator::Neg, Type::Unit), None);
    assert_eq!(rules::unary_result(UnaryOperator::Not, Type::Unit), None);
}

// --- operators in programs ---

fn invalid_binary(operator: BinaryOperator, left: Type, right: Type) -> TypeErrorKind {
    TypeErrorKind::InvalidBinaryOperation {
        operator,
        left,
        right,
    }
}

fn invalid_unary(operator: UnaryOperator, operand: Type) -> TypeErrorKind {
    TypeErrorKind::InvalidUnaryOperation { operator, operand }
}

#[test]
fn nested_arithmetic_is_typed_from_the_inside_out() {
    let source = in_main("let x = 10 + 20 * 3;");
    assert_eq!(type_at(&source, "20 * 3"), Some(Type::I32));
    assert_eq!(type_at(&source, "10 + 20 * 3"), Some(Type::I32));
}

#[test]
fn comparisons_and_equality_are_bool() {
    let source = in_main("let a = 1 < 2;\nlet b = \"x\" == \"y\";\nlet c = true != false;");
    assert_eq!(type_at(&source, "1 < 2"), Some(Type::Bool));
    assert_eq!(type_at(&source, "\"x\" == \"y\""), Some(Type::Bool));
    assert_eq!(type_at(&source, "true != false"), Some(Type::Bool));
}

#[test]
fn strings_concatenate_with_plus() {
    let source = in_main("let s = \"a\" + \"b\";");
    assert_eq!(type_at(&source, "\"a\" + \"b\""), Some(Type::String));
}

#[test]
fn unary_operators() {
    let source = in_main("let a = !false;\nlet b = --5;");
    assert_eq!(type_at(&source, "!false"), Some(Type::Bool));
    assert_eq!(type_at(&source, "--5"), Some(Type::I32));
}

#[test]
fn adding_an_integer_and_a_string_is_rejected() {
    let source = in_main("let x = 10 + \"hello\";");
    let errors = type_errors(&source);
    assert_eq!(
        errors[0].kind,
        invalid_binary(BinaryOperator::Add, Type::I32, Type::String)
    );
    // Reported at the whole operation.
    assert_eq!(errors[0].span, span_of(&source, "10 + \"hello\""));
}

#[test]
fn multiplying_a_boolean_is_rejected() {
    assert_eq!(
        error_kinds(&in_main("let x = true * 10;")),
        vec![invalid_binary(BinaryOperator::Mul, Type::Bool, Type::I32)]
    );
}

#[test]
fn ordering_a_string_and_a_boolean_is_rejected() {
    assert_eq!(
        error_kinds(&in_main("let x = \"hello\" < false;")),
        vec![invalid_binary(BinaryOperator::Lt, Type::String, Type::Bool)]
    );
}

#[test]
fn chained_comparisons_compare_a_bool_with_an_integer() {
    // `1 < 2 < 3` is `(1 < 2) < 3`.
    assert_eq!(
        error_kinds(&in_main("let x = 1 < 2 < 3;")),
        vec![invalid_binary(BinaryOperator::Lt, Type::Bool, Type::I32)]
    );
}

#[test]
fn invalid_unary_operations_are_rejected() {
    assert_eq!(
        error_kinds(&in_main("let a = -true;\nlet b = !1;")),
        vec![
            invalid_unary(UnaryOperator::Neg, Type::Bool),
            invalid_unary(UnaryOperator::Not, Type::I32),
        ]
    );
}

#[test]
fn one_mistake_is_reported_once() {
    // `1 + true` is wrong; the `* 2` and `== 3` around it are not
    // reported again.
    assert_eq!(
        error_kinds(&in_main("let x = (1 + true) * 2 == 3;")),
        vec![invalid_binary(BinaryOperator::Add, Type::I32, Type::Bool)]
    );
}

#[test]
fn independent_mistakes_are_all_reported() {
    assert_eq!(
        error_kinds(&in_main("let a = 1 + true;\nlet b = \"s\" * 2;")),
        vec![
            invalid_binary(BinaryOperator::Add, Type::I32, Type::Bool),
            invalid_binary(BinaryOperator::Mul, Type::String, Type::I32),
        ]
    );
}

#[test]
fn the_checker_never_evaluates_anything() {
    // Division by zero and overflow are facts about values, found only
    // by running the program. Both expressions are perfectly well-typed.
    let source = in_main("let a = 1 / 0;\nlet b = 2147483647 + 1;");
    assert_eq!(type_at(&source, "1 / 0"), Some(Type::I32));
    assert_eq!(type_at(&source, "2147483647 + 1"), Some(Type::I32));
}

#[test]
fn a_literal_out_of_range_inside_an_operation_is_reported() {
    assert_eq!(
        error_kinds(&in_main("let x = 1 + 3000000000;")),
        vec![out_of_range(3000000000, Type::I32)]
    );
}

// --- the type environment ---

#[test]
fn a_declared_variable_has_its_type() {
    let mut env = TypeEnvironment::new();
    env.declare("x", Type::I32);
    assert_eq!(env.lookup("x"), Some(Type::I32));
    assert_eq!(env.lookup("y"), None);
}

#[test]
fn an_inner_scope_shadows_and_then_forgets() {
    let mut env = TypeEnvironment::new();
    env.declare("x", Type::I32);
    env.push_scope();
    assert_eq!(env.depth(), 1);
    env.declare("x", Type::String);
    assert_eq!(env.lookup("x"), Some(Type::String));
    env.pop_scope();
    assert_eq!(env.depth(), 0);
    assert_eq!(env.lookup("x"), Some(Type::I32));
}

#[test]
fn outer_variables_are_visible_in_inner_scopes() {
    let mut env = TypeEnvironment::new();
    env.declare("x", Type::Bool);
    env.push_scope();
    env.push_scope();
    assert_eq!(env.lookup("x"), Some(Type::Bool));
}

#[test]
fn the_function_scope_is_never_popped() {
    let mut env = TypeEnvironment::new();
    env.declare("x", Type::I64);
    env.pop_scope();
    assert_eq!(env.depth(), 0);
    assert_eq!(env.lookup("x"), Some(Type::I64));
}

// --- variables ---

/// The bindings the checker recorded for `main`: name, type and depth.
fn bindings_of_main(source: &str) -> Vec<(String, Type, usize)> {
    let typed = typed(source);
    let main = typed
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("the fixture has a main");
    main.bindings
        .iter()
        .map(|binding| (binding.name.clone(), binding.ty, binding.depth))
        .collect()
}

fn binding(name: &str, ty: Type, depth: usize) -> (String, Type, usize) {
    (name.to_string(), ty, depth)
}

fn mismatch(expected: Type, found: Type) -> TypeErrorKind {
    TypeErrorKind::Mismatch { expected, found }
}

#[test]
fn a_let_without_annotation_takes_the_initializers_type() {
    assert_eq!(
        bindings_of_main(&in_main("let x = 10;\nlet y = true;\nlet name = \"Dijk\";")),
        vec![
            binding("x", Type::I32, 0),
            binding("y", Type::Bool, 0),
            binding("name", Type::String, 0),
        ]
    );
}

#[test]
fn a_let_with_an_annotation_has_the_annotated_type() {
    assert_eq!(
        bindings_of_main(&in_main("let x: i32 = 10;\nlet y: bool = true;")),
        vec![binding("x", Type::I32, 0), binding("y", Type::Bool, 0)]
    );
}

#[test]
fn an_annotation_is_what_an_integer_literal_is_expected_to_be() {
    let source = in_main("let big: i64 = 10;");
    assert_eq!(type_at(&source, "10"), Some(Type::I64));
    assert_eq!(
        bindings_of_main(&source),
        vec![binding("big", Type::I64, 0)]
    );
}

#[test]
fn the_expected_type_flows_through_arithmetic_into_every_literal() {
    let source = in_main("let x: i64 = 1 + 2 * -3;");
    for literal in ["1", "2", "-3", "2 * -3"] {
        assert_eq!(type_at(&source, literal), Some(Type::I64), "`{literal}`");
    }
}

#[test]
fn a_literal_that_only_fits_in_i64_needs_an_i64_context() {
    assert!(types::check(&parse(&in_main("let big: i64 = 3000000000;"))).is_ok());
    assert_eq!(
        error_kinds(&in_main("let small = 3000000000;")),
        vec![out_of_range(3000000000, Type::I32)]
    );
}

#[test]
fn a_string_initializer_for_an_i32_is_a_mismatch() {
    let source = in_main("let x: i32 = \"hello\";");
    let errors = type_errors(&source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, mismatch(Type::I32, Type::String));
    // Reported at the initializer, the part that has the wrong type.
    assert_eq!(errors[0].span, span_of(&source, "\"hello\""));
}

#[test]
fn an_integer_initializer_for_a_bool_is_a_mismatch() {
    assert_eq!(
        error_kinds(&in_main("let b: bool = 1;")),
        vec![mismatch(Type::Bool, Type::I32)]
    );
}

#[test]
fn an_i32_variable_is_not_an_i64() {
    // No implicit conversion, not even the widening one.
    assert_eq!(
        error_kinds(&in_main("let x: i32 = 10;\nlet y: i64 = x;")),
        vec![mismatch(Type::I64, Type::I32)]
    );
}

#[test]
fn a_variable_use_has_the_variables_type() {
    let source = in_main("let x: i64 = 5;\nlet y = x + 1;");
    assert_eq!(type_at(&source, "x + 1"), Some(Type::I64));
    // The literal took its type from `x`.
    assert_eq!(type_at(&source, "1"), Some(Type::I64));
    assert_eq!(
        bindings_of_main(&source),
        vec![binding("x", Type::I64, 0), binding("y", Type::I64, 0)]
    );
}

#[test]
fn a_literal_on_the_left_takes_its_type_from_the_right() {
    let source = in_main("let x: i64 = 5;\nlet y = 1 + x;");
    assert_eq!(type_at(&source, "1 + x"), Some(Type::I64));
}

#[test]
fn mixing_i32_and_i64_variables_is_rejected() {
    assert_eq!(
        error_kinds(&in_main("let a: i64 = 1;\nlet b = 2;\nlet c = a + b;")),
        vec![invalid_binary(BinaryOperator::Add, Type::I64, Type::I32)]
    );
}

#[test]
fn an_unknown_type_name_is_reported_once() {
    // `x`'s later use is not reported again.
    let source = in_main("let x: banana = 1;\nlet y = x + 1;");
    let errors = type_errors(&source);
    assert_eq!(
        errors.iter().map(|e| e.kind.clone()).collect::<Vec<_>>(),
        vec![TypeErrorKind::UnknownType {
            name: "banana".to_string()
        }]
    );
    assert_eq!(errors[0].span, span_of(&source, "banana"));
}

#[test]
fn a_variable_from_a_mistaken_initializer_is_not_reported_again() {
    assert_eq!(
        error_kinds(&in_main("let x = 1 + true;\nlet y = x * 2;")),
        vec![invalid_binary(BinaryOperator::Add, Type::I32, Type::Bool)]
    );
}

#[test]
fn a_shadowing_variable_has_its_own_type_and_disappears_with_its_block() {
    let source = in_main(
        "let x = 1;\nif true {\n    let x = \"inner\";\n    let s = x + \"!\";\n}\nlet y = x + 1;",
    );
    assert_eq!(
        bindings_of_main(&source),
        vec![
            binding("x", Type::I32, 0),
            binding("x", Type::String, 1),
            binding("s", Type::String, 1),
            binding("y", Type::I32, 0),
        ]
    );
}

#[test]
fn a_shadowing_initializer_reads_the_outer_variable() {
    let source = in_main("let x = 1;\nwhile false {\n    let x = x < 2;\n}");
    assert_eq!(
        bindings_of_main(&source),
        vec![binding("x", Type::I32, 0), binding("x", Type::Bool, 1)]
    );
}

#[test]
fn bindings_record_the_depth_of_their_block() {
    let source = in_main("let a = 1;\nif true {\n    let b = 2;\n    while false {\n        let c = 3;\n    }\n} else {\n    let d = 4;\n}");
    assert_eq!(
        bindings_of_main(&source),
        vec![
            binding("a", Type::I32, 0),
            binding("b", Type::I32, 1),
            binding("c", Type::I32, 2),
            binding("d", Type::I32, 1),
        ]
    );
}

#[test]
fn each_function_has_its_own_bindings() {
    let typed =
        typed("fn helper() {\n    let h = true;\n}\n\nfn main() {\n    let m = \"m\";\n}\n");
    let names: Vec<_> = typed
        .functions
        .iter()
        .map(|f| {
            (
                f.name.as_str(),
                f.bindings
                    .iter()
                    .map(|b| b.name.as_str())
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(names, vec![("helper", vec!["h"]), ("main", vec!["m"])]);
}

#[test]
fn a_binding_records_where_its_name_is_and_whether_it_is_mutable() {
    let source = in_main("let a = 1;\nlet mut b: i64 = 2;");
    let typed = typed(&source);
    let bindings = &typed.functions[0].bindings;
    let a = source.find("a = 1").unwrap();
    assert_eq!(bindings[0].span, Span::new(a, a + 1));
    assert!(!bindings[0].mutable);
    // The name itself, not the `let` or the `mut` before it.
    let b = source.find("b: i64").unwrap();
    assert_eq!(bindings[1].span, Span::new(b, b + 1));
    assert!(bindings[1].mutable);
    assert_eq!(bindings[1].ty, Type::I64);
}
