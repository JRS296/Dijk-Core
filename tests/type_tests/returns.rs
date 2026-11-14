//! Returns: every `return` must produce the function's return type
//! (E0005), and a function that returns something other than `unit` must
//! return on every path through its body (E0009).

use super::*;

// --- valid ---

#[test]
fn returning_the_declared_type() {
    // Step 5's example.
    well_typed("fn get_number() -> i32 {\n    return 42;\n}\n\nfn main() {}\n");
}

#[test]
fn a_returned_literal_takes_the_return_type() {
    let source = in_function("i64", "return 3000000000;");
    assert_eq!(type_of(&source, "3000000000"), Type::I64);
}

#[test]
fn a_unit_function_may_return_early_or_not_at_all() {
    well_typed("fn early(stop: bool) {\n    if stop {\n        return;\n    }\n    let x = 1;\n}\n\nfn main() {\n    early(true);\n}\n");
}

#[test]
fn main_may_return_any_type() {
    for (ty, value) in [
        ("i32", "0"),
        ("i64", "0"),
        ("bool", "true"),
        ("string", "\"done\""),
    ] {
        well_typed(&format!("fn main() -> {ty} {{\n    return {value};\n}}\n"));
    }
}

#[test]
fn an_if_else_returns_when_both_branches_do() {
    well_typed(&in_function(
        "i32",
        "if true {\n    return 1;\n} else {\n    return 2;\n}",
    ));
}

#[test]
fn an_else_if_chain_returns_when_it_ends_in_an_else() {
    well_typed(&in_function(
        "string",
        "if false {\n    return \"a\";\n} else if true {\n    return \"b\";\n} else {\n    return \"c\";\n}",
    ));
}

#[test]
fn a_return_after_a_loop_makes_the_function_return() {
    well_typed(&in_function(
        "i32",
        "let mut i = 0;\nwhile i < 10 {\n    if i == 5 {\n        return i;\n    }\n    i = i + 1;\n}\nreturn -1;",
    ));
}

#[test]
fn statements_after_a_return_are_allowed() {
    well_typed(&in_function("i32", "return 1;\nlet unreachable = 2;"));
}

// --- invalid: return types ---

#[test]
fn returning_another_type_is_rejected() {
    // Step 5's example.
    assert_eq!(
        codes("fn get_number() -> i32 {\n    return \"hello\";\n}\n\nfn main() {}\n"),
        vec!["E0005"]
    );
}

#[test]
fn a_bare_return_in_a_function_with_a_return_type_is_rejected() {
    assert_eq!(codes(&in_function("i32", "return;")), vec!["E0005"]);
}

#[test]
fn a_value_returned_from_a_unit_function_is_rejected() {
    assert_eq!(
        codes("fn log() {\n    return 1;\n}\n\nfn main() {}\n"),
        vec!["E0005"]
    );
}

#[test]
fn the_0_0_4_style_main_is_rejected() {
    // `fn main()` without `-> T` returns `unit`.
    assert_eq!(
        codes("fn main() {\n    let x = 10;\n    let y = x + 20;\n    return y;\n}\n"),
        vec!["E0005"]
    );
}

#[test]
fn an_i32_is_not_returned_where_an_i64_is_declared() {
    assert_eq!(
        codes(&in_function("i64", "let small = 1;\nreturn small;")),
        vec!["E0005"]
    );
}

// --- invalid: return paths ---

#[test]
fn an_if_without_else_may_not_return() {
    // Step 5's return-completeness example.
    assert_eq!(
        codes("fn get_number() -> i32 {\n    if true {\n        return 10;\n    }\n}\n\nfn main() {}\n"),
        vec!["E0009"]
    );
}

#[test]
fn a_body_without_any_return_is_rejected() {
    assert_eq!(codes(&in_function("bool", "let b = true;")), vec!["E0009"]);
    assert_eq!(codes(&in_function("string", "")), vec!["E0009"]);
}

#[test]
fn a_return_only_inside_a_loop_is_not_enough() {
    // Not even `while true`: the checker never evaluates a condition.
    assert_eq!(
        codes(&in_function("i32", "while true {\n    return 1;\n}")),
        vec!["E0009"]
    );
}

#[test]
fn an_else_if_chain_without_a_final_else_may_not_return() {
    assert_eq!(
        codes(&in_function(
            "i32",
            "if false {\n    return 1;\n} else if true {\n    return 2;\n}"
        )),
        vec!["E0009"]
    );
}

#[test]
fn every_function_is_checked_for_its_own_return_paths() {
    assert_eq!(
        codes("fn a() -> i32 {\n}\n\nfn b() -> bool {\n    if true {\n        return true;\n    }\n}\n\nfn main() {}\n"),
        vec!["E0009", "E0009"]
    );
}
