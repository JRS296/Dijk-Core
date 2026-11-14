//! Tests for semantic analysis (`crate::semantic`): programs that parse
//! cleanly, checked for whether their names refer to anything.
//!
//! Every fixture here is syntactically valid. That is the point: these
//! are the mistakes a parser cannot see, because they are about meaning,
//! not structure.

// None of these modules are part of a library's public surface (dijk is a
// binary crate), so these tests include them directly by path, mirroring
// their layout as siblings under the crate root in src/main.rs. The
// lexer and parser are here only to turn source text into the AST that
// semantic analysis checks; their own diagnostic rendering goes unused,
// hence the `allow`.
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
#[path = "../src/semantic/mod.rs"]
mod semantic;

use semantic::error::{SemanticError, SemanticErrorKind};

/// Lex, parse and analyze `source`, asserting the first two stages found
/// nothing wrong. A failure there would mean the fixture is broken, not
/// semantic analysis.
fn analyze(source: &str) -> Vec<SemanticError> {
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
    semantic::analyze(&program)
}

fn kinds(source: &str) -> Vec<SemanticErrorKind> {
    analyze(source).into_iter().map(|e| e.kind).collect()
}

fn assert_valid(source: &str) {
    let errors = analyze(source);
    assert!(errors.is_empty(), "unexpected semantic errors: {errors:?}");
}

fn unknown_identifier(name: &str) -> SemanticErrorKind {
    SemanticErrorKind::UnknownIdentifier {
        name: name.to_string(),
    }
}

// --- valid programs ---

#[test]
fn the_milestone_program_is_valid() {
    assert_valid("fn main() {\n    let x = 10;\n    let y = x + 20;\n    return y;\n}\n");
}

#[test]
fn parameters_are_visible_in_the_body() {
    assert_valid(
        "fn add(a: i32, b: i32) -> i32 { return a + b; }\nfn main() { return add(1, 2); }\n",
    );
}

#[test]
fn semantic_analysis_does_not_check_types() {
    // A type checker would reject both lines. Name resolution has no
    // opinion about them: every name here refers to something.
    assert_valid(
        "fn main() {\n    let x: banana = \"hi\";\n    let y: i32 = x + true;\n    return y;\n}\n",
    );
}

// --- identifiers ---

#[test]
fn unknown_identifier_is_reported() {
    let errors = analyze("fn main() {\n    return x;\n}\n");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, unknown_identifier("x"));
    // The span is the identifier itself: `x` on line 2.
    assert_eq!((errors[0].span.start, errors[0].span.end), (23, 24));
}

#[test]
fn a_variable_cannot_be_used_before_its_let() {
    assert_eq!(
        kinds("fn main() {\n    let y = x;\n    let x = 1;\n}\n"),
        vec![unknown_identifier("x")]
    );
}

#[test]
fn a_let_initializer_cannot_refer_to_the_variable_it_declares() {
    assert_eq!(
        kinds("fn main() {\n    let x = x + 1;\n}\n"),
        vec![unknown_identifier("x")]
    );
}

#[test]
fn a_function_cannot_see_another_functions_variables() {
    assert_eq!(
        kinds("fn helper() {\n    return secret;\n}\nfn main() {\n    let secret = 1;\n    helper();\n}\n"),
        vec![unknown_identifier("secret")]
    );
}

#[test]
fn a_function_name_is_not_a_variable() {
    // Dijk has no first-class functions: `helper` without a call is
    // looked up as a variable, and there is none.
    assert_eq!(
        kinds("fn helper() {}\nfn main() {\n    return helper;\n}\n"),
        vec![unknown_identifier("helper")]
    );
}

#[test]
fn identifiers_are_resolved_inside_every_kind_of_expression() {
    assert_eq!(
        kinds("fn f(n: i32) {}\nfn main() {\n    let r = -a + (b * c);\n    f(d);\n    if e == 1 {}\n    while !g {}\n}\n"),
        vec![
            unknown_identifier("a"),
            unknown_identifier("b"),
            unknown_identifier("c"),
            unknown_identifier("d"),
            unknown_identifier("e"),
            unknown_identifier("g"),
        ]
    );
}

// --- assignment ---

#[test]
fn assignment_to_a_declared_variable_is_valid() {
    assert_valid("fn main() {\n    let x = 1;\n    x = x + 1;\n    return x;\n}\n");
}

#[test]
fn assignment_to_a_parameter_is_valid() {
    assert_valid("fn f(n: i32) { n = 0; }\nfn main() { f(1); }\n");
}

#[test]
fn assignment_to_an_undeclared_variable_is_reported() {
    assert_eq!(
        kinds("fn main() {\n    x = 1;\n}\n"),
        vec![SemanticErrorKind::AssignToUndeclared {
            name: "x".to_string()
        }]
    );
}

// --- functions and calls ---

#[test]
fn unknown_function_is_reported() {
    let errors = analyze("fn main() {\n    foo();\n}\n");
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].kind,
        SemanticErrorKind::UnknownFunction {
            name: "foo".to_string()
        }
    );
    assert_eq!(errors[0].span.start, 16);
}

#[test]
fn a_variable_cannot_be_called() {
    assert_eq!(
        kinds("fn main() {\n    let f = 1;\n    f();\n}\n"),
        vec![SemanticErrorKind::UnknownFunction {
            name: "f".to_string()
        }]
    );
}

#[test]
fn arguments_of_an_unknown_function_are_still_resolved() {
    assert_eq!(
        kinds("fn main() {\n    foo(x);\n}\n"),
        vec![
            SemanticErrorKind::UnknownFunction {
                name: "foo".to_string()
            },
            unknown_identifier("x"),
        ]
    );
}

#[test]
fn a_function_may_be_called_before_it_is_defined() {
    assert_valid(
        "fn main() {\n    return later(1);\n}\nfn later(n: i32) -> i32 {\n    return n;\n}\n",
    );
}

#[test]
fn a_function_may_call_itself() {
    assert_valid("fn f(n: i32) -> i32 {\n    return f(n - 1);\n}\nfn main() { return f(3); }\n");
}

#[test]
fn too_few_arguments_is_an_arity_mismatch() {
    assert_eq!(
        kinds("fn add(a: i32, b: i32) -> i32 { return a + b; }\nfn main() { return add(1); }\n"),
        vec![SemanticErrorKind::ArityMismatch {
            name: "add".to_string(),
            expected: 2,
            found: 1,
        }]
    );
}

#[test]
fn too_many_arguments_is_an_arity_mismatch() {
    assert_eq!(
        kinds("fn zero() {}\nfn main() { zero(1, 2); }\n"),
        vec![SemanticErrorKind::ArityMismatch {
            name: "zero".to_string(),
            expected: 0,
            found: 2,
        }]
    );
}

#[test]
fn duplicate_function_is_reported_at_the_second_definition() {
    let source = "fn main() {}\nfn twice() {}\nfn twice() {}\n";
    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].kind,
        SemanticErrorKind::DuplicateFunction {
            name: "twice".to_string()
        }
    );
    assert_eq!(errors[0].span.start, source.rfind("fn twice").unwrap());
}

#[test]
fn duplicate_parameter_is_reported() {
    assert_eq!(
        kinds("fn f(a: i32, a: i32) {}\nfn main() { f(1, 2); }\n"),
        vec![SemanticErrorKind::DuplicateParameter {
            name: "a".to_string()
        }]
    );
}

// --- main ---

#[test]
fn a_program_without_main_is_rejected() {
    assert_eq!(
        kinds("fn helper() {}\n"),
        vec![SemanticErrorKind::MissingMain]
    );
}

#[test]
fn an_empty_program_has_no_main() {
    let errors = analyze("");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, SemanticErrorKind::MissingMain);
    assert_eq!((errors[0].span.start, errors[0].span.end), (0, 0));
}

#[test]
fn missing_main_points_at_the_end_of_the_program() {
    let source = "fn helper() {\n}\n";
    let errors = analyze(source);
    assert_eq!(errors[0].span.start, source.rfind('}').unwrap() + 1);
}

#[test]
fn main_must_not_take_parameters() {
    assert_eq!(
        kinds("fn main(argc: i32) {}\n"),
        vec![SemanticErrorKind::MainHasParameters]
    );
}

// --- lexical scopes ---

fn duplicate_variable(name: &str) -> SemanticErrorKind {
    SemanticErrorKind::DuplicateVariable {
        name: name.to_string(),
    }
}

#[test]
fn a_block_variable_is_not_visible_after_the_block() {
    // Step 4's own example: `y` exists only inside the `if`.
    assert_eq!(
        kinds("fn main() {\n    let x = 10;\n    if true {\n        let y = 20;\n    }\n    return y;\n}\n"),
        vec![unknown_identifier("y")]
    );
}

#[test]
fn a_loop_body_variable_is_not_visible_after_the_loop() {
    assert_eq!(
        kinds("fn main() {\n    while false {\n        let t = 1;\n    }\n    t = 2;\n}\n"),
        vec![SemanticErrorKind::AssignToUndeclared {
            name: "t".to_string()
        }]
    );
}

#[test]
fn the_then_branch_and_else_branch_are_separate_scopes() {
    assert_eq!(
        kinds("fn main() {\n    if true {\n        let a = 1;\n    } else {\n        return a;\n    }\n}\n"),
        vec![unknown_identifier("a")]
    );
}

#[test]
fn outer_variables_are_visible_in_nested_blocks() {
    assert_valid(
        "fn main() {\n    let total = 0;\n    while total < 10 {\n        if total == 5 {\n            total = total + 2;\n        } else {\n            total = total + 1;\n        }\n    }\n    return total;\n}\n",
    );
}

#[test]
fn duplicate_let_in_the_same_scope_is_reported() {
    let source = "fn main() {\n    let x = 1;\n    let x = 2;\n}\n";
    let errors = analyze(source);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, duplicate_variable("x"));
    // Reported at the second `let`.
    assert_eq!(errors[0].span.start, source.rfind("let").unwrap());
}

#[test]
fn duplicate_let_in_the_same_block_is_reported() {
    assert_eq!(
        kinds("fn main() {\n    if true {\n        let x = 1;\n        let x = 2;\n    }\n}\n"),
        vec![duplicate_variable("x")]
    );
}

#[test]
fn a_let_cannot_redeclare_a_parameter() {
    // Parameters and the body's top-level `let`s share the function scope.
    assert_eq!(
        kinds("fn f(n: i32) {\n    let n = 2;\n}\nfn main() { f(1); }\n"),
        vec![duplicate_variable("n")]
    );
}

#[test]
fn shadowing_in_an_inner_scope_is_allowed() {
    assert_valid("fn main() {\n    let x = 1;\n    if true {\n        let x = \"inner\";\n        x = \"still inner\";\n    }\n    return x;\n}\n");
}

#[test]
fn a_parameter_may_be_shadowed_in_a_block() {
    assert_valid(
        "fn f(n: i32) {\n    while n > 0 {\n        let n = 0;\n    }\n}\nfn main() { f(1); }\n",
    );
}

#[test]
fn sibling_blocks_may_reuse_a_name() {
    assert_valid("fn main() {\n    if true {\n        let t = 1;\n    }\n    while false {\n        let t = 2;\n    }\n}\n");
}

#[test]
fn each_else_if_branch_has_its_own_scope() {
    assert_eq!(
        kinds("fn main() {\n    if false {\n        let a = 1;\n    } else if true {\n        let b = a;\n    } else {\n        let c = b;\n    }\n}\n"),
        vec![unknown_identifier("a"), unknown_identifier("b")]
    );
}

#[test]
fn scopes_nest_arbitrarily_deep() {
    assert_valid("fn main() {\n    let a = 1;\n    if true {\n        let b = a;\n        while false {\n            let c = a + b;\n            if true {\n                return a + b + c;\n            }\n        }\n    }\n}\n");
}

#[test]
fn a_shadowing_let_can_read_the_variable_it_shadows() {
    // The initializer is resolved before the new `x` exists, so it sees
    // the outer one.
    assert_valid("fn main() {\n    let x = 1;\n    if true {\n        let x = x + 1;\n    }\n}\n");
}

// --- reporting ---

#[test]
fn every_error_is_reported_in_source_order() {
    assert_eq!(
        kinds("fn main() {\n    let a = b;\n    c = 1;\n    d(a);\n}\n"),
        vec![
            unknown_identifier("b"),
            SemanticErrorKind::AssignToUndeclared {
                name: "c".to_string()
            },
            SemanticErrorKind::UnknownFunction {
                name: "d".to_string()
            },
        ]
    );
}

#[test]
fn semantic_errors_render_with_their_own_label() {
    let source = "fn main() {\n    return unknown;\n}\n";
    let errors = analyze(source);
    let rendered = errors[0].render("example.dk", source);
    assert!(rendered.starts_with("semantic error: unknown identifier `unknown`\n"));
    assert!(rendered.contains(" --> example.dk:2:12"));
    assert!(rendered.contains("2 |     return unknown;"));
    assert_eq!(errors[0].to_string(), "unknown identifier `unknown`");
}
