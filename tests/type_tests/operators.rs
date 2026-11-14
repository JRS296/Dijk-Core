//! Operators: each has a table of typing rules (`src/types/rules.rs`), and
//! any combination of operand types without a rule is rejected (E0002 for
//! binary operators, E0003 for unary ones).

use super::*;

// --- valid ---

#[test]
fn arithmetic_on_two_i32s_is_an_i32() {
    let source = in_main("let x = 10 + 20 * 3 - 8 / 2;");
    assert_eq!(type_of(&source, "20 * 3"), Type::I32);
    assert_eq!(type_of(&source, "10 + 20 * 3 - 8 / 2"), Type::I32);
}

#[test]
fn arithmetic_on_two_i64s_is_an_i64() {
    let source = in_main("let a: i64 = 1;\nlet b: i64 = 2;\nlet c = a * b - a;");
    assert_eq!(type_of(&source, "a * b - a"), Type::I64);
}

#[test]
fn a_literal_operand_takes_the_other_operands_type() {
    let source = in_main("let a: i64 = 1;\nlet b = a + 1;\nlet c = 2 * a;");
    assert_eq!(
        binding_types(&source, "main"),
        vec![
            bound("a", Type::I64),
            bound("b", Type::I64),
            bound("c", Type::I64)
        ]
    );
}

#[test]
fn plus_concatenates_strings() {
    assert_eq!(
        type_of(&in_main("let s = \"a\" + \"b\";"), "\"a\" + \"b\""),
        Type::String
    );
}

#[test]
fn comparisons_of_same_type_integers_are_bool() {
    for operator in ["<", "<=", ">", ">="] {
        let source = in_main(&format!("let a: i64 = 1;\nlet b = a {operator} 2;"));
        assert_eq!(binding_types(&source, "main")[1].1, Type::Bool);
    }
}

#[test]
fn equality_works_on_every_type_but_unit() {
    well_typed(&in_main(
        "let a = 1 == 2;\nlet b: i64 = 1;\nlet c = b != b;\nlet d = true == false;\nlet e = \"x\" != \"y\";",
    ));
}

#[test]
fn unary_operators() {
    let source =
        in_main("let a = -5;\nlet b: i64 = 5;\nlet c = -b;\nlet d = !true;\nlet e = !(1 < 2);");
    assert_eq!(
        binding_types(&source, "main"),
        vec![
            bound("a", Type::I32),
            bound("b", Type::I64),
            bound("c", Type::I64),
            bound("d", Type::Bool),
            bound("e", Type::Bool),
        ]
    );
}

// --- invalid ---

#[test]
fn an_integer_plus_a_string_is_rejected() {
    assert_eq!(codes(&in_main("let x = 10 + \"hello\";")), vec!["E0002"]);
}

#[test]
fn a_boolean_times_an_integer_is_rejected() {
    assert_eq!(codes(&in_main("let x = true * 10;")), vec!["E0002"]);
}

#[test]
fn a_string_compared_with_a_boolean_is_rejected() {
    assert_eq!(codes(&in_main("let x = \"hello\" < false;")), vec!["E0002"]);
}

#[test]
fn i32_and_i64_never_mix() {
    assert_eq!(
        codes(&in_main(
            "let a: i64 = 1;\nlet b = 2;\nlet c = a + b;\nlet d = b < a;\nlet e = a == b;"
        )),
        vec!["E0002", "E0002", "E0002"]
    );
}

#[test]
fn only_plus_works_on_strings() {
    assert_eq!(
        codes(&in_main(
            "let a = \"a\" - \"b\";\nlet b = \"a\" * \"b\";\nlet c = \"a\" < \"b\";"
        )),
        vec!["E0002", "E0002", "E0002"]
    );
}

#[test]
fn booleans_are_not_ordered_or_added() {
    assert_eq!(
        codes(&in_main("let a = true < false;\nlet b = true + true;")),
        vec!["E0002", "E0002"]
    );
}

#[test]
fn no_operator_accepts_unit() {
    assert_eq!(
        codes("fn nothing() {}\n\nfn main() {\n    let a = nothing() == nothing();\n    let b = nothing() + 1;\n    let c = !nothing();\n}\n"),
        vec!["E0002", "E0002", "E0003"]
    );
}

#[test]
fn chained_comparisons_are_rejected() {
    // `1 < 2 < 3` is `(1 < 2) < 3`: a bool compared with an integer.
    assert_eq!(codes(&in_main("let x = 1 < 2 < 3;")), vec!["E0002"]);
}

#[test]
fn negating_a_non_integer_is_rejected() {
    assert_eq!(
        codes(&in_main("let a = -true;\nlet b = -\"s\";")),
        vec!["E0003", "E0003"]
    );
}

#[test]
fn not_on_a_non_boolean_is_rejected() {
    assert_eq!(
        codes(&in_main("let a = !5;\nlet b = !\"s\";")),
        vec!["E0003", "E0003"]
    );
}
