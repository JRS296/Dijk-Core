//! Functions: signatures are collected before any body, each argument
//! must have its parameter's type (E0004), and parameter and return type
//! names must exist (E0008). Whether a called function exists, and how
//! many arguments it takes, are semantic analysis's rules.

use super::*;

const ADD: &str = "fn add(a: i32, b: i32) -> i32 {\n    return a + b;\n}\n";

// --- valid ---

#[test]
fn a_call_with_the_right_argument_types_has_the_return_type() {
    // Step 5's example: `add(10, 20) → i32`.
    let source = format!("{ADD}\nfn main() -> i32 {{\n    return add(10, 20);\n}}\n");
    assert_eq!(type_of(&source, "add(10, 20)"), Type::I32);
}

#[test]
fn the_signature_is_known_from_the_declaration() {
    let source = "fn add(a: i32, b: i32) -> i32 {\n    let result = a + b;\n    return result;\n}\n\nfn main() {}\n";
    let typed = well_typed(source);
    let add = &typed.functions[0];
    assert_eq!(
        add.signature.params,
        vec![bound("a", Type::I32), bound("b", Type::I32)]
    );
    assert_eq!(add.signature.return_type, Type::I32);
    assert_eq!(
        binding_types(source, "add"),
        vec![
            bound("a", Type::I32),
            bound("b", Type::I32),
            bound("result", Type::I32)
        ]
    );
}

#[test]
fn a_function_may_be_called_before_it_is_declared() {
    // Step 5's forward-reference example.
    well_typed("fn main() -> i32 {\n    return add(10, 20);\n}\n\nfn add(a: i32, b: i32) -> i32 {\n    return a + b;\n}\n");
}

#[test]
fn recursion_and_mutual_recursion_are_checked_against_the_signatures() {
    well_typed("fn factorial(n: i64) -> i64 {\n    if n <= 1 {\n        return 1;\n    }\n    return n * factorial(n - 1);\n}\n\nfn is_even(n: i32) -> bool {\n    if n == 0 {\n        return true;\n    }\n    return is_odd(n - 1);\n}\n\nfn is_odd(n: i32) -> bool {\n    if n == 0 {\n        return false;\n    }\n    return is_even(n - 1);\n}\n\nfn main() {}\n");
}

#[test]
fn literal_arguments_take_their_parameters_types() {
    let source = "fn mix(a: i64, b: i32) {}\n\nfn main() {\n    mix(3000000000, 7);\n}\n";
    assert_eq!(type_of(source, "3000000000"), Type::I64);
    assert_eq!(type_of(source, "7"), Type::I32);
}

#[test]
fn calls_nest_and_feed_each_other() {
    well_typed(&format!(
        "{ADD}\nfn main() {{\n    let x = add(add(1, 2), add(3 * 4, -2));\n}}\n"
    ));
}

#[test]
fn every_type_can_be_passed_and_returned() {
    well_typed("fn echo(n: i32, w: i64, b: bool, s: string) -> string {\n    if b {\n        return s;\n    }\n    return \"no\";\n}\n\nfn main() {\n    let big: i64 = 1;\n    let r = echo(1, big, true, \"yes\");\n}\n");
}

// --- invalid ---

#[test]
fn an_argument_of_the_wrong_type_is_rejected() {
    assert_eq!(
        codes(&format!(
            "{ADD}\nfn main() {{\n    add(10, \"hello\");\n}}\n"
        )),
        vec!["E0004"]
    );
}

#[test]
fn every_wrong_argument_is_reported() {
    assert_eq!(
        codes(&format!(
            "{ADD}\nfn main() {{\n    add(true, \"two\");\n}}\n"
        )),
        vec!["E0004", "E0004"]
    );
}

#[test]
fn an_i32_argument_for_an_i64_parameter_is_rejected() {
    assert_eq!(
        codes("fn wide(n: i64) {}\n\nfn main() {\n    let small = 1;\n    wide(small);\n}\n"),
        vec!["E0004"]
    );
}

#[test]
fn a_call_result_must_be_used_at_its_return_type() {
    assert_eq!(
        codes(&format!(
            "{ADD}\nfn main() {{\n    let ok: bool = add(1, 2);\n    let sum = add(1, 2) + \"three\";\n}}\n"
        )),
        vec!["E0001", "E0002"]
    );
}

#[test]
fn an_unknown_parameter_type_is_rejected() {
    assert_eq!(
        codes("fn f(x: number) {}\n\nfn main() {\n    f(1);\n}\n"),
        vec!["E0008"]
    );
}

#[test]
fn an_unknown_return_type_is_rejected() {
    assert_eq!(
        codes("fn f() -> number {\n    return 1;\n}\n\nfn main() {}\n"),
        vec!["E0008"]
    );
}

#[test]
fn existence_and_argument_count_are_semantic_errors() {
    // Step 5 lists `add(10)` and `unknown_function()` among the calls to
    // reject. They are rejected, before the type checker ever runs.
    let program = parse_syntax(&format!(
        "{ADD}\nfn main() {{\n    add(10);\n    unknown_function();\n}}\n"
    ));
    let kinds: Vec<_> = semantic::analyze(&program)
        .into_iter()
        .map(|e| e.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            semantic::error::SemanticErrorKind::ArityMismatch {
                name: "add".to_string(),
                expected: 2,
                found: 1,
            },
            semantic::error::SemanticErrorKind::UnknownFunction {
                name: "unknown_function".to_string()
            },
        ]
    );
}
