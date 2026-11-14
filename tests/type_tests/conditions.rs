//! Conditions: the condition of every `if`, `else if` and `while` must be
//! a `bool` (E0006). Dijk has no "truthiness".

use super::*;

// --- valid ---

#[test]
fn boolean_conditions_are_accepted() {
    // Step 5's example, and every way of producing a `bool`.
    well_typed("fn ready() -> bool {\n    return true;\n}\n\nfn f() -> i32 {\n    if true {\n        return 10;\n    }\n    let flag = false;\n    let n: i64 = 3;\n    if flag {\n    } else if !flag {\n    }\n    while n > 5 {\n    }\n    if ready() == (n != 0) {\n    }\n    return 0;\n}\n\nfn main() {}\n");
}

// --- invalid ---

#[test]
fn an_integer_condition_is_rejected() {
    // Step 5's example.
    assert_eq!(
        codes(&in_function("i32", "if 42 {\n    return 10;\n}\nreturn 0;")),
        vec!["E0006"]
    );
}

#[test]
fn the_same_rule_applies_to_while() {
    assert_eq!(codes(&in_main("while 1 {\n}")), vec!["E0006"]);
}

#[test]
fn every_non_boolean_type_is_rejected() {
    assert_eq!(
        codes("fn nothing() {}\n\nfn main() {\n    let wide: i64 = 1;\n    if wide {\n    }\n    if \"yes\" {\n    }\n    while nothing() {\n    }\n}\n"),
        vec!["E0006", "E0006", "E0006"]
    );
}

#[test]
fn else_if_conditions_are_checked_too() {
    assert_eq!(
        codes(&in_main("if false {\n} else if 0 {\n} else if \"s\" {\n}")),
        vec!["E0006", "E0006"]
    );
}

#[test]
fn a_nested_condition_is_checked() {
    assert_eq!(
        codes(&in_main("while true {\n    if 1 {\n    }\n}")),
        vec!["E0006"]
    );
}
