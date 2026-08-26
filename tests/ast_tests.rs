//! Tests for the AST itself (`crate::ast`), independent of the parser:
//! constructing nodes directly, checking their behavior, and walking
//! hand-built trees. See `docs/ast.md`.
//!
//! Contrast with `tests/parser_tests.rs`, which checks that the parser
//! builds the *right* AST from source text. These tests instead ask
//! whether the AST types themselves behave correctly once you already
//! have one — the distinction Step 3 draws between parsing and
//! representing a program.

// `ast` is not part of a library's public surface (dijk is a binary
// crate), so this test includes it directly by path, mirroring its
// layout as a sibling under the crate root in src/main.rs. `ast` depends
// on `lexer::token::Span` — since these tests exercise the AST in
// isolation from lexing/parsing, only `token.rs` (self-contained, no
// further dependencies) is pulled in, not the rest of the lexer.
#[path = "../src/ast/mod.rs"]
mod ast;
#[path = "../src/lexer"]
mod lexer {
    // token.rs bundles TokenKind and Token alongside Span; this file
    // only needs Span, so the other two are unavoidably unused here.
    #[allow(dead_code)]
    #[path = "token.rs"]
    pub mod token;
}

use ast::declaration::{Declaration, Function, Param, Program};
use ast::expression::{BinaryOperator, Expression, UnaryOperator};
use ast::statement::Statement;
use ast::types::Type;
use ast::visit::{self, Visitor};
use lexer::token::Span;

fn span(start: usize, end: usize) -> Span {
    Span::new(start, end)
}

// --- operator display ---

#[test]
fn binary_operator_display() {
    use BinaryOperator::*;
    let cases = [
        (Add, "+"),
        (Sub, "-"),
        (Mul, "*"),
        (Div, "/"),
        (Eq, "=="),
        (NotEq, "!="),
        (Lt, "<"),
        (Le, "<="),
        (Gt, ">"),
        (Ge, ">="),
    ];
    for (op, text) in cases {
        assert_eq!(op.to_string(), text);
    }
}

#[test]
fn unary_operator_display() {
    assert_eq!(UnaryOperator::Neg.to_string(), "-");
    assert_eq!(UnaryOperator::Not.to_string(), "!");
}

// --- direct construction ---

#[test]
fn construct_program_directly() {
    // Program
    // └── Function add(a: i32, b: i32) -> i32
    //     └── Return
    //         └── Binary(+)
    //             ├── Identifier(a)
    //             └── Identifier(b)
    let a = Expression::Identifier("a".to_string(), span(0, 0));
    let b = Expression::Identifier("b".to_string(), span(0, 0));
    let sum = Expression::Binary {
        left: Box::new(a),
        operator: BinaryOperator::Add,
        right: Box::new(b),
        span: span(0, 0),
    };
    let function = Function {
        name: "add".to_string(),
        params: vec![
            Param {
                name: "a".to_string(),
                ty: Type {
                    name: "i32".to_string(),
                    span: span(0, 0),
                },
                span: span(0, 0),
            },
            Param {
                name: "b".to_string(),
                ty: Type {
                    name: "i32".to_string(),
                    span: span(0, 0),
                },
                span: span(0, 0),
            },
        ],
        return_type: Some(Type {
            name: "i32".to_string(),
            span: span(0, 0),
        }),
        body: vec![Statement::Return {
            value: Some(sum),
            span: span(0, 0),
        }],
        span: span(0, 0),
    };
    let program = Program {
        declarations: vec![Declaration::Function(function)],
    };

    assert_eq!(program.declarations.len(), 1);
    let Declaration::Function(f) = &program.declarations[0];
    assert_eq!(f.name, "add");
    assert_eq!(f.params.len(), 2);
    assert_eq!(f.return_type.as_ref().unwrap().name, "i32");
    assert_eq!(f.body.len(), 1);
}

#[test]
fn equal_expressions_constructed_separately_are_equal() {
    let a = Expression::Integer(42, span(0, 2));
    let b = Expression::Integer(42, span(0, 2));
    assert_eq!(a, b);
}

#[test]
fn expressions_with_different_values_are_not_equal() {
    let a = Expression::Integer(42, span(0, 2));
    let b = Expression::Integer(7, span(0, 1));
    assert_ne!(a, b);
}

// --- spans ---

#[test]
fn expression_span_accessor_covers_every_variant() {
    let s = span(3, 9);
    assert_eq!(Expression::Integer(1, s).span(), s);
    assert_eq!(Expression::String("x".to_string(), s).span(), s);
    assert_eq!(Expression::Boolean(true, s).span(), s);
    assert_eq!(Expression::Identifier("x".to_string(), s).span(), s);
    assert_eq!(
        Expression::Unary {
            operator: UnaryOperator::Neg,
            operand: Box::new(Expression::Integer(1, s)),
            span: s
        }
        .span(),
        s
    );
    assert_eq!(
        Expression::Assign {
            name: "x".to_string(),
            value: Box::new(Expression::Integer(1, s)),
            span: s
        }
        .span(),
        s
    );
}

#[test]
fn statement_span_accessor_covers_every_variant() {
    let s = span(3, 9);
    let expr = Expression::Integer(1, s);
    assert_eq!(
        Statement::Let {
            name: "x".to_string(),
            type_annotation: None,
            value: expr.clone(),
            span: s
        }
        .span(),
        s
    );
    assert_eq!(
        Statement::Return {
            value: None,
            span: s
        }
        .span(),
        s
    );
    assert_eq!(
        Statement::Expression {
            expr: expr.clone(),
            span: s
        }
        .span(),
        s
    );
    assert_eq!(
        Statement::If {
            condition: expr.clone(),
            then_branch: vec![],
            else_branch: None,
            span: s
        }
        .span(),
        s
    );
    assert_eq!(
        Statement::While {
            condition: expr,
            body: vec![],
            span: s
        }
        .span(),
        s
    );
}

// --- traversal over hand-built trees ---

struct IntegerCollector(Vec<i64>);

impl Visitor for IntegerCollector {
    fn visit_expression(&mut self, expr: &Expression) {
        if let Expression::Integer(value, _) = expr {
            self.0.push(*value);
        }
        visit::walk_expression(self, expr);
    }
}

#[test]
fn traversal_reaches_every_leaf_in_a_nested_expression() {
    // 1 + (2 * 3) — built directly, not parsed, mirroring the classic
    // precedence example from docs/parser.md and docs/ast.md.
    let s = span(0, 0);
    let two_times_three = Expression::Binary {
        left: Box::new(Expression::Integer(2, s)),
        operator: BinaryOperator::Mul,
        right: Box::new(Expression::Integer(3, s)),
        span: s,
    };
    let tree = Expression::Binary {
        left: Box::new(Expression::Integer(1, s)),
        operator: BinaryOperator::Add,
        right: Box::new(two_times_three),
        span: s,
    };

    let mut collector = IntegerCollector(Vec::new());
    collector.visit_expression(&tree);
    assert_eq!(collector.0, vec![1, 2, 3]);
}

#[test]
fn traversal_reaches_expressions_inside_if_and_while_bodies() {
    let s = span(0, 0);
    let condition = Expression::Integer(1, s);
    let then_value = Expression::Integer(2, s);
    let while_value = Expression::Integer(3, s);

    let if_stmt = Statement::If {
        condition: condition.clone(),
        then_branch: vec![Statement::Expression {
            expr: then_value,
            span: s,
        }],
        else_branch: None,
        span: s,
    };
    let while_stmt = Statement::While {
        condition,
        body: vec![Statement::Expression {
            expr: while_value,
            span: s,
        }],
        span: s,
    };

    let mut collector = IntegerCollector(Vec::new());
    collector.visit_statement(&if_stmt);
    collector.visit_statement(&while_stmt);

    assert_eq!(collector.0, vec![1, 2, 1, 3]);
}

#[test]
fn visiting_a_program_reaches_every_function_and_its_nested_expressions() {
    // The full top-down chain — visit_program -> visit_declaration ->
    // visit_function -> visit_statement -> visit_expression — starting
    // from the root, not from an already-unwrapped node.
    let s = span(0, 0);
    let make_function = |name: &str, value: i64| Function {
        name: name.to_string(),
        params: vec![],
        return_type: None,
        body: vec![Statement::Return {
            value: Some(Expression::Integer(value, s)),
            span: s,
        }],
        span: s,
    };
    let program = Program {
        declarations: vec![
            Declaration::Function(make_function("a", 1)),
            Declaration::Function(make_function("b", 2)),
        ],
    };

    let mut collector = IntegerCollector(Vec::new());
    collector.visit_program(&program);
    assert_eq!(collector.0, vec![1, 2]);
}
