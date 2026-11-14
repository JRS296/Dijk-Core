//! Assignments: `x = value` must store a value of the type `x` already
//! has (E0007). Whether `x` may be assigned at all is a different rule,
//! about the binding rather than the value, enforced by semantic analysis.

use super::*;

// --- valid ---

#[test]
fn assigning_a_value_of_the_same_type() {
    well_typed(&in_main("let mut x: i32 = 10;\nx = 20;"));
}

#[test]
fn an_assigned_literal_takes_the_variables_type() {
    let source = in_main("let mut total: i64 = 0;\ntotal = 3000000000;");
    assert_eq!(type_of(&source, "3000000000"), Type::I64);
}

#[test]
fn assigning_in_a_nested_block_to_an_outer_variable() {
    well_typed(&in_main(
        "let mut count = 0;\nwhile count < 10 {\n    if count == 5 {\n        count = count + 2;\n    } else {\n        count = count + 1;\n    }\n}",
    ));
}

#[test]
fn assigning_to_a_shadowing_variable_uses_its_own_type() {
    well_typed(&in_main(
        "let mut x = 1;\nif true {\n    let mut x = \"inner\";\n    x = \"changed\";\n}\nx = 2;",
    ));
}

#[test]
fn a_chained_assignment_has_the_variables_type() {
    let source = in_main("let mut a: i64 = 0;\nlet mut b: i64 = 0;\na = b = 7;");
    assert_eq!(type_of(&source, "b = 7"), Type::I64);
    assert_eq!(type_of(&source, "a = b = 7"), Type::I64);
}

// --- invalid ---

#[test]
fn assigning_a_value_of_another_type_is_rejected() {
    assert_eq!(
        codes(&in_main("let mut x: i32 = 10;\nx = \"hello\";")),
        vec!["E0007"]
    );
}

#[test]
fn an_inferred_type_cannot_change_later() {
    assert_eq!(codes(&in_main("let mut x = 1;\nx = true;")), vec!["E0007"]);
}

#[test]
fn an_i32_cannot_be_assigned_to_an_i64_variable() {
    assert_eq!(
        codes(&in_main(
            "let mut wide: i64 = 0;\nlet narrow = 1;\nwide = narrow;"
        )),
        vec!["E0007"]
    );
}

#[test]
fn assigning_a_wrong_value_in_a_nested_block_is_rejected() {
    assert_eq!(
        codes(&in_main(
            "let mut x = 0;\nwhile x < 3 {\n    if true {\n        x = \"three\";\n    }\n}"
        )),
        vec!["E0007"]
    );
}

#[test]
fn assignments_check_against_the_variable_visible_at_that_point() {
    // Inside the block `x` is the string; after it, the outer i32.
    assert_eq!(
        codes(&in_main(
            "let mut x = 1;\nif true {\n    let mut x = \"inner\";\n    x = 2;\n}\nx = \"outer\";"
        )),
        vec!["E0007", "E0007"]
    );
}

#[test]
fn each_link_of_a_chained_assignment_is_checked() {
    assert_eq!(
        codes(&in_main("let mut s = \"a\";\nlet mut n = 0;\ns = n = 1;")),
        vec!["E0007"]
    );
}

#[test]
fn assigning_to_an_immutable_variable_is_a_semantic_error_not_a_type_error() {
    // The types are fine: `2` is an `i32`, like `x`. The binding is not
    // `mut`, and semantic analysis, which runs first, reports that.
    let program = parse_syntax(&in_main("let x = 1;\nx = 2;"));
    let semantic_errors: Vec<_> = semantic::analyze(&program)
        .into_iter()
        .map(|e| e.kind)
        .collect();
    assert_eq!(
        semantic_errors,
        vec![semantic::error::SemanticErrorKind::AssignToImmutable {
            name: "x".to_string()
        }]
    );
    assert!(types::check(&program).is_ok());
}
