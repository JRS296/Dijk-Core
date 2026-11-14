//! Literals: `42`, `true`, `"hello"`. Each has a type the checker knows
//! without running anything, and an integer literal must fit in the type
//! its context gives it (E0010).

use super::*;

// --- valid ---

#[test]
fn an_integer_literal_with_no_context_is_an_i32() {
    assert_eq!(type_of(&in_main("let x = 42;"), "42"), Type::I32);
}

#[test]
fn boolean_literals_are_bool() {
    let source = in_main("let t = true;\nlet f = false;");
    assert_eq!(type_of(&source, "true"), Type::Bool);
    assert_eq!(type_of(&source, "false"), Type::Bool);
}

#[test]
fn a_string_literal_is_a_string() {
    assert_eq!(
        type_of(&in_main("let s = \"hello\";"), "\"hello\""),
        Type::String
    );
}

#[test]
fn an_integer_literal_takes_the_type_its_context_expects() {
    // An annotation, a parameter, a return type: each is a context.
    let source = "fn wide(n: i64) -> i64 {\n    return 333;\n}\n\nfn main() {\n    let a: i64 = 111;\n    let b = wide(222);\n}\n";
    assert_eq!(type_of(source, "111"), Type::I64);
    assert_eq!(type_of(source, "222"), Type::I64);
    assert_eq!(type_of(source, "333"), Type::I64);
}

#[test]
fn the_extremes_of_i32_fit() {
    well_typed(&in_main("let max = 2147483647;\nlet min = -2147483648;"));
}

#[test]
fn the_extremes_of_i64_fit_in_an_i64_context() {
    well_typed(&in_main(
        "let max: i64 = 9223372036854775807;\nlet min: i64 = -9223372036854775807 - 1;",
    ));
}

#[test]
fn a_literal_too_big_for_i32_fits_in_an_i64_context() {
    well_typed(&in_main("let big: i64 = 3000000000;"));
}

// --- invalid ---

#[test]
fn one_past_the_largest_i32_is_out_of_range() {
    assert_eq!(codes(&in_main("let x = 2147483648;")), vec!["E0010"]);
}

#[test]
fn one_past_the_smallest_i32_is_out_of_range() {
    assert_eq!(codes(&in_main("let x = -2147483649;")), vec!["E0010"]);
}

#[test]
fn a_literal_is_range_checked_against_every_kind_of_context() {
    // An `i32` annotation, parameter, and return type.
    let source = "fn narrow(n: i32) -> i32 {\n    return 3000000000;\n}\n\nfn main() {\n    let a: i32 = 3000000001;\n    narrow(3000000002);\n}\n";
    assert_eq!(codes(source), vec!["E0010", "E0010", "E0010"]);
}

#[test]
fn a_literal_where_no_integer_is_wanted_is_one_mistake_not_two() {
    // The mismatch is reported; the literal's range is not checked
    // against the `i32` it falls back to.
    assert_eq!(codes(&in_main("let b: bool = 3000000000;")), vec!["E0001"]);
    assert_eq!(codes(&in_main("if 3000000000 {\n}")), vec!["E0006"]);
    assert_eq!(
        codes(&in_main("let s = \"a\" + 3000000000;")),
        vec!["E0002"]
    );
}
