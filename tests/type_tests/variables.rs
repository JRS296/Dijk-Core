//! Variables: a `let` either infers its variable's type from the
//! initializer, or checks the initializer against an annotation (E0001).
//! Annotations must name a type Dijk has (E0008).

use super::*;

// --- valid ---

#[test]
fn a_let_infers_its_type_from_the_initializer() {
    // Step 5's own example.
    assert_eq!(
        binding_types(
            &in_main("let x = 10;\nlet y = true;\nlet name = \"Dijk\";"),
            "main"
        ),
        vec![
            bound("x", Type::I32),
            bound("y", Type::Bool),
            bound("name", Type::String)
        ]
    );
}

#[test]
fn an_annotation_gives_the_variable_its_type() {
    assert_eq!(
        binding_types(
            &in_main(
                "let a: i32 = 10;\nlet b: i64 = 10;\nlet c: bool = true;\nlet d: string = \"s\";"
            ),
            "main"
        ),
        vec![
            bound("a", Type::I32),
            bound("b", Type::I64),
            bound("c", Type::Bool),
            bound("d", Type::String)
        ]
    );
}

#[test]
fn a_variable_can_be_used_at_its_type() {
    // Step 5's example: `y` is inferred from `x + 20`.
    let source = in_main("let x: i32 = 10;\nlet y = x + 20;");
    assert_eq!(
        binding_types(&source, "main"),
        vec![bound("x", Type::I32), bound("y", Type::I32)]
    );
}

#[test]
fn an_inferred_variable_can_hold_a_call_result() {
    let source =
        "fn flag() -> bool {\n    return true;\n}\n\nfn main() {\n    let f = flag();\n}\n";
    assert_eq!(binding_types(source, "main"), vec![bound("f", Type::Bool)]);
}

#[test]
fn a_shadowing_variable_may_have_a_different_type() {
    let source = in_main(
        "let x = 1;\nif true {\n    let x = \"inner\";\n    let s = x + \"!\";\n}\nlet y = x * 2;",
    );
    assert_eq!(
        binding_types(&source, "main"),
        vec![
            bound("x", Type::I32),
            bound("x", Type::String),
            bound("s", Type::String),
            bound("y", Type::I32),
        ]
    );
}

#[test]
fn an_initializer_is_checked_before_its_variable_exists() {
    // The `x` on the right is the outer `i32`, so the inner `x` is a bool.
    let source = in_main("let x = 1;\nwhile false {\n    let x = x == 1;\n}");
    assert_eq!(
        binding_types(&source, "main"),
        vec![bound("x", Type::I32), bound("x", Type::Bool)]
    );
}

// --- invalid ---

#[test]
fn a_string_for_an_i32_annotation_is_a_mismatch() {
    assert_eq!(codes(&in_main("let x: i32 = \"hello\";")), vec!["E0001"]);
}

#[test]
fn every_wrong_initializer_is_a_mismatch() {
    assert_eq!(
        codes(&in_main(
            "let a: bool = 0;\nlet b: string = true;\nlet c: i32 = \"1\";\nlet d: i64 = false;"
        )),
        vec!["E0001", "E0001", "E0001", "E0001"]
    );
}

#[test]
fn an_i32_is_not_an_i64_even_though_it_would_fit() {
    // Step 5's compatibility example: no implicit conversion.
    assert_eq!(
        codes(&in_main("let x: i32 = 10;\nlet y: i64 = x;")),
        vec!["E0001"]
    );
}

#[test]
fn a_unit_call_result_does_not_fit_an_annotation() {
    assert_eq!(
        codes("fn nothing() {}\n\nfn main() {\n    let x: i32 = nothing();\n}\n"),
        vec!["E0001"]
    );
}

#[test]
fn an_unknown_type_name_is_rejected() {
    assert_eq!(codes(&in_main("let x: banana = 1;")), vec!["E0008"]);
    assert_eq!(codes(&in_main("let x: float = 1;")), vec!["E0008"]);
    // Type names are case-sensitive.
    assert_eq!(codes(&in_main("let x: I32 = 1;")), vec!["E0008"]);
}

#[test]
fn unit_is_not_a_type_name_a_program_can_write() {
    assert_eq!(codes(&in_main("let x: unit = 1;")), vec!["E0008"]);
}
