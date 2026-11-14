//! Diagnostics: every rule has a stable code, every error is reported once
//! and in source order, and each renders with its source line, the whole
//! offending expression underlined, and what was expected and found.

use super::*;

/// The first type error in `source`, as `dijk check main.dk` prints it.
fn rendered(source: &str) -> String {
    type_errors(source)[0].render("main.dk", source)
}

#[test]
fn the_milestone_error_renders_exactly() {
    assert_eq!(
        rendered("fn main() -> i32 {\n    let x: i32 = \"hello\";\n    return x;\n}\n"),
        "type error[E0001]: type mismatch
 --> main.dk:2:18
  |
2 |     let x: i32 = \"hello\";
  |                  ^^^^^^^
  = expected `i32`
  = found `string`
"
    );
}

#[test]
fn codes_are_distinct_and_stable() {
    let programs = [
        in_main("let x: i32 = true;"),
        in_main("let x = 1 + true;"),
        in_main("let x = !1;"),
        "fn f(n: i32) {}\n\nfn main() {\n    f(\"n\");\n}\n".to_string(),
        in_function("i32", "return false;"),
        in_main("while 1 {\n}"),
        in_main("let mut x = 1;\nx = \"one\";"),
        in_main("let x: char = 1;"),
        in_function("i32", "let x = 1;"),
        in_main("let x = 2147483648;"),
    ];
    let found: Vec<&str> = programs
        .iter()
        .map(|program| {
            let codes = codes(program);
            assert_eq!(codes.len(), 1, "{program}");
            codes[0]
        })
        .collect();
    assert_eq!(
        found,
        vec![
            "E0001", "E0002", "E0003", "E0004", "E0005", "E0006", "E0007", "E0008", "E0009",
            "E0010"
        ]
    );
}

#[test]
fn every_error_is_reported_in_source_order() {
    assert_eq!(
        codes("fn f(n: i32) -> bool {\n    if n {\n        return 1;\n    }\n}\n\nfn main() {\n    let a: string = 1;\n    f(\"x\");\n}\n"),
        vec!["E0006", "E0005", "E0009", "E0001", "E0004"]
    );
}

#[test]
fn checking_continues_into_every_function_after_an_error() {
    assert_eq!(
        codes("fn first() {\n    let a: i32 = true;\n}\n\nfn second() {\n    let b: bool = 1;\n}\n\nfn main() {}\n"),
        vec!["E0001", "E0001"]
    );
}

#[test]
fn one_mistake_produces_one_diagnostic() {
    // `x` has an unknown type and `y` a broken initializer. Neither is
    // reported again where it is used.
    assert_eq!(
        codes(&in_main(
            "let x: banana = 1;\nlet y = 1 + true;\nlet z = x + y;\nif x == y {\n}\nlet w: i32 = y;"
        )),
        vec!["E0008", "E0002"]
    );
}

#[test]
fn the_whole_offending_expression_is_underlined() {
    let text = rendered(&in_main("let x = 1 + \"one\" * 2;"));
    // The invalid part is `"one" * 2`, not the whole initializer.
    assert!(text.contains("2 | let x = 1 + \"one\" * 2;\n  |             ^^^^^^^^^\n"));
}

#[test]
fn every_diagnostic_says_what_was_expected_and_found() {
    for program in [
        in_main("let x: i32 = true;"),
        in_main("let x = 1 + true;"),
        in_main("let x = !1;"),
        "fn f(n: i32) {}\n\nfn main() {\n    f(\"n\");\n}\n".to_string(),
        in_function("i32", "return false;"),
        in_main("while 1 {\n}"),
        in_main("let mut x = 1;\nx = \"one\";"),
        in_function("i32", "let x = 1;"),
        in_main("let x = 2147483648;"),
    ] {
        let text = rendered(&program);
        assert!(text.contains("  = expected "), "{text}");
        assert!(text.contains("  = found "), "{text}");
    }
}
