//! End-to-end tests for execution: source text through every stage
//! (lexer → parser → semantic analysis → type checking → interpreter),
//! checking the value `main` returns, or the runtime error it stops with.
//!
//! These test the interpreter through the same pipeline `dijk run` uses,
//! so every program here has already passed semantic analysis and type
//! checking. The exceptions are the tests that deliberately skip those
//! stages, to check that the interpreter still reports, rather than
//! crashes on, a program nobody checked.

// None of these modules are part of a library's public surface (dijk is a
// binary crate), so these tests include them directly by path, mirroring
// their layout as siblings under the crate root in src/main.rs. The
// earlier stages are here only to get a program to the interpreter, so
// their diagnostic rendering goes unused, hence the `allow`s.
#[path = "../src/ast/mod.rs"]
mod ast;
#[allow(dead_code)]
#[path = "../src/diagnostics.rs"]
mod diagnostics;
#[path = "../src/interpreter/mod.rs"]
mod interpreter;
#[allow(dead_code)]
#[path = "../src/lexer/mod.rs"]
mod lexer;
#[allow(dead_code)]
#[path = "../src/parser/mod.rs"]
mod parser;
#[allow(dead_code)]
#[path = "../src/semantic/mod.rs"]
mod semantic;
#[allow(dead_code)]
#[path = "../src/types/mod.rs"]
mod types;

use ast::declaration::Program;
use interpreter::error::{RuntimeError, RuntimeErrorKind};
use interpreter::value::Value;
use semantic::error::SemanticErrorKind;
use types::TypedProgram;

/// Lex and parse `source`, asserting both stages succeed: a failure there
/// would mean the fixture is broken, not the interpreter.
fn parse(source: &str) -> Program {
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
    program
}

/// Run `source` exactly as `dijk run` would, after asserting it passes
/// semantic analysis and type checking.
fn run(source: &str) -> Result<Value, RuntimeError> {
    let program = parse(source);
    let semantic_errors = semantic::analyze(&program);
    assert!(
        semantic_errors.is_empty(),
        "unexpected semantic errors: {semantic_errors:?}"
    );
    let typed = match types::check(&program) {
        Ok(typed) => typed,
        Err(errors) => panic!("unexpected type errors: {errors:?}"),
    };
    interpreter::run(&program, &typed)
}

/// Run `source` with no semantic analysis and no type checking, the way
/// the interpreter's own guards see a program nobody checked. Its integer
/// literals are all `i32`s, the type the checker gives a literal with no
/// context.
fn run_unchecked(source: &str) -> Result<Value, RuntimeError> {
    interpreter::run(&parse(source), &TypedProgram::default())
}

/// The value `main` returns, asserting the program runs without error.
fn value(source: &str) -> Value {
    match run(source) {
        Ok(value) => value,
        Err(error) => panic!("unexpected runtime error: {error} ({error:?})"),
    }
}

/// The value returned by a `main` declared `-> {return_type}` whose body
/// is `body`.
fn main_returning(return_type: &str, body: &str) -> Value {
    value(&format!("fn main() -> {return_type} {{\n{body}\n}}\n"))
}

/// The value returned by a `main` declared `-> i32` whose body is `body`.
fn main_returns(body: &str) -> Value {
    main_returning("i32", body)
}

/// The runtime error `source` stops with, asserting that it does stop
/// with one.
fn runtime_error(source: &str) -> RuntimeError {
    match run(source) {
        Ok(value) => panic!("expected a runtime error, got the value {value:?}"),
        Err(error) => error,
    }
}

/// The kind of runtime error a `main` declared `-> {return_type}` whose
/// body is `body` stops with.
fn main_returning_fails_with(return_type: &str, body: &str) -> RuntimeErrorKind {
    runtime_error(&format!("fn main() -> {return_type} {{\n{body}\n}}\n")).kind
}

/// The kind of runtime error a `main` declared `-> i32` whose body is
/// `body` stops with.
fn main_fails_with(body: &str) -> RuntimeErrorKind {
    main_returning_fails_with("i32", body)
}

/// The kind of runtime error an *unchecked* `main` whose body is `body`
/// stops with.
fn unchecked_main_fails_with(body: &str) -> RuntimeErrorKind {
    match run_unchecked(&format!("fn main() {{\n{body}\n}}\n")) {
        Ok(value) => panic!("expected a runtime error, got the value {value:?}"),
        Err(error) => error.kind,
    }
}

fn int(value: i32) -> Value {
    Value::I32(value)
}

fn long(value: i64) -> Value {
    Value::I64(value)
}

fn string(value: &str) -> Value {
    Value::String(value.to_string())
}

fn invalid_binary(operator: &str, left: &'static str, right: &'static str) -> RuntimeErrorKind {
    RuntimeErrorKind::InvalidBinaryOperation {
        operator: operator.to_string(),
        left,
        right,
    }
}

// --- the milestone ---

#[test]
fn the_milestone_program_returns_30() {
    assert_eq!(
        value("fn main() -> i32 {\n    let x = 10;\n    let y = x + 20;\n\n    return y;\n}\n"),
        int(30)
    );
}

#[test]
fn the_manual_trace_exercise_returns_25() {
    // Step 4 §18: main → double(10) → 20, then 20 + 5.
    assert_eq!(
        value("fn double(x: i32) -> i32 {\n    return x * 2;\n}\n\nfn main() -> i32 {\n    let x = 10;\n    let y = double(x);\n\n    return y + 5;\n}\n"),
        int(25)
    );
}

// --- integer arithmetic ---

#[test]
fn the_tree_shape_decides_the_order_of_evaluation() {
    assert_eq!(main_returns("return 10 + 20 * 3;"), int(70));
    assert_eq!(main_returns("return (10 + 20) * 3;"), int(90));
}

#[test]
fn the_four_arithmetic_operators() {
    assert_eq!(main_returns("return 7 + 5;"), int(12));
    assert_eq!(main_returns("return 7 - 5;"), int(2));
    assert_eq!(main_returns("return 7 * 5;"), int(35));
    assert_eq!(main_returns("return 35 / 5;"), int(7));
}

#[test]
fn subtraction_is_left_associative() {
    assert_eq!(main_returns("return 10 - 3 - 2;"), int(5));
}

#[test]
fn integer_division_truncates_toward_zero() {
    assert_eq!(main_returns("return 7 / 2;"), int(3));
    assert_eq!(main_returns("return -7 / 2;"), int(-3));
}

#[test]
fn integer_literals_get_the_type_the_checker_gave_them() {
    // The same digits build an `i32` or an `i64`, depending on context.
    assert_eq!(main_returns("return 5;"), int(5));
    assert_eq!(main_returning("i64", "return 5;"), long(5));
    assert_ne!(int(5), long(5));
    assert_eq!(
        main_returning("i64", "let x: i64 = 1;\nreturn x + 2;"),
        long(3)
    );
    assert_eq!(
        value("fn wide(n: i64) -> i64 {\n    return n * 2;\n}\n\nfn main() -> i64 {\n    return wide(3000000000);\n}\n"),
        long(6_000_000_000)
    );
}

#[test]
fn unary_minus_negates() {
    assert_eq!(main_returns("return -5;"), int(-5));
    assert_eq!(main_returns("return --5;"), int(5));
    assert_eq!(main_returns("return 3 - -2;"), int(5));
}

// --- booleans ---

#[test]
fn boolean_literals() {
    assert_eq!(main_returning("bool", "return true;"), Value::Boolean(true));
    assert_eq!(
        main_returning("bool", "return false;"),
        Value::Boolean(false)
    );
}

#[test]
fn integer_comparisons() {
    assert_eq!(
        main_returning("bool", "return 1 < 2;"),
        Value::Boolean(true)
    );
    assert_eq!(
        main_returning("bool", "return 2 <= 2;"),
        Value::Boolean(true)
    );
    assert_eq!(
        main_returning("bool", "return 1 > 2;"),
        Value::Boolean(false)
    );
    assert_eq!(
        main_returning("bool", "return 1 >= 2;"),
        Value::Boolean(false)
    );
}

#[test]
fn equality_between_values_of_the_same_kind() {
    assert_eq!(
        main_returning("bool", "return 3 == 3;"),
        Value::Boolean(true)
    );
    assert_eq!(
        main_returning("bool", "return 3 != 3;"),
        Value::Boolean(false)
    );
    assert_eq!(
        main_returning("bool", "return true == false;"),
        Value::Boolean(false)
    );
    assert_eq!(
        main_returning("bool", "return true != false;"),
        Value::Boolean(true)
    );
}

#[test]
fn logical_not() {
    assert_eq!(
        main_returning("bool", "return !true;"),
        Value::Boolean(false)
    );
    assert_eq!(
        main_returning("bool", "return !(1 > 2);"),
        Value::Boolean(true)
    );
}

// --- strings ---

#[test]
fn a_string_literal_evaluates_to_its_contents() {
    assert_eq!(
        main_returning("string", "return \"hello\";"),
        string("hello")
    );
}

#[test]
fn plus_concatenates_strings() {
    assert_eq!(
        main_returning(
            "string",
            "let greeting = \"hello\";\nreturn greeting + \", \" + \"world\";"
        ),
        string("hello, world")
    );
}

#[test]
fn strings_compare_by_contents() {
    assert_eq!(
        main_returning("bool", "return \"ab\" == \"a\" + \"b\";"),
        Value::Boolean(true)
    );
    assert_eq!(
        main_returning("bool", "return \"ab\" != \"ba\";"),
        Value::Boolean(true)
    );
}

// --- variables and assignment ---

#[test]
fn a_variable_evaluates_to_its_value() {
    assert_eq!(main_returns("let x = 42;\nreturn x;"), int(42));
}

#[test]
fn a_program_with_a_type_error_never_reaches_the_interpreter() {
    // Until 0.0.5, `i32` was only a name and this ran, returning a string.
    // Now the type checker rejects it, and `dijk run` stops there.
    let program =
        parse("fn main() -> i32 {\n    let x: i32 = \"not an integer\";\n    return x;\n}\n");
    assert!(semantic::analyze(&program).is_empty());
    assert!(types::check(&program).is_err());
}

#[test]
fn assignment_changes_a_variable() {
    assert_eq!(
        main_returns("let mut x = 1;\nx = x + 1;\nx = x * 10;\nreturn x;"),
        int(20)
    );
}

#[test]
fn an_assignment_evaluates_to_the_assigned_value() {
    assert_eq!(main_returns("let mut x = 0;\nreturn x = 5;"), int(5));
    assert_eq!(
        main_returns("let mut a = 0;\nlet mut b = 0;\na = b = 7;\nreturn a + b;"),
        int(14)
    );
}

#[test]
fn an_assignment_in_a_block_changes_the_outer_variable() {
    assert_eq!(
        main_returns("let mut x = 1;\nif true {\n    x = 2;\n}\nreturn x;"),
        int(2)
    );
}

#[test]
fn a_shadowing_variable_disappears_with_its_block() {
    assert_eq!(
        main_returns("let x = 1;\nif true {\n    let mut x = 100;\n    x = x + 1;\n}\nreturn x;"),
        int(1)
    );
}

// --- if ---

#[test]
fn if_runs_the_then_branch_when_the_condition_is_true() {
    assert_eq!(
        main_returns("let mut r = 0;\nif 1 < 2 {\n    r = 1;\n}\nreturn r;"),
        int(1)
    );
}

#[test]
fn if_without_else_does_nothing_when_the_condition_is_false() {
    assert_eq!(
        main_returns("let mut r = 0;\nif 1 > 2 {\n    r = 1;\n}\nreturn r;"),
        int(0)
    );
}

#[test]
fn if_runs_the_else_branch_when_the_condition_is_false() {
    assert_eq!(
        main_returns("let mut r = 0;\nif false {\n    r = 1;\n} else {\n    r = 2;\n}\nreturn r;"),
        int(2)
    );
}

#[test]
fn else_if_chains_pick_the_first_true_branch() {
    let program = "fn classify(n: i32) -> string {\n    if n < 0 {\n        return \"negative\";\n    } else if n == 0 {\n        return \"zero\";\n    } else if n < 10 {\n        return \"small\";\n    } else {\n        return \"large\";\n    }\n}\n";
    for (n, expected) in [
        ("-5", "negative"),
        ("0", "zero"),
        ("3", "small"),
        ("50", "large"),
    ] {
        assert_eq!(
            value(&format!(
                "{program}fn main() -> string {{\n    return classify({n});\n}}\n"
            )),
            string(expected)
        );
    }
}

// --- while ---

#[test]
fn while_repeats_until_its_condition_is_false() {
    assert_eq!(
        main_returns("let mut x = 10;\nwhile x > 0 {\n    x = x - 1;\n}\nreturn x;"),
        int(0)
    );
}

#[test]
fn while_sums_a_range() {
    assert_eq!(
        main_returns("let mut sum = 0;\nlet mut i = 1;\nwhile i <= 100 {\n    sum = sum + i;\n    i = i + 1;\n}\nreturn sum;"),
        int(5050)
    );
}

#[test]
fn a_while_whose_condition_starts_false_never_runs() {
    assert_eq!(
        main_returns("let mut r = 0;\nwhile false {\n    r = 1;\n}\nreturn r;"),
        int(0)
    );
}

#[test]
fn a_let_in_a_loop_body_is_fresh_on_every_iteration() {
    assert_eq!(
        main_returns("let mut total = 0;\nlet mut i = 0;\nwhile i < 3 {\n    let square = i * i;\n    total = total + square;\n    i = i + 1;\n}\nreturn total;"),
        int(5)
    );
}

// --- functions and arguments ---

#[test]
fn a_call_evaluates_to_what_the_function_returns() {
    assert_eq!(
        value("fn add(a: i32, b: i32) -> i32 {\n    return a + b;\n}\n\nfn main() -> i32 {\n    let x = add(10, 20);\n    return x;\n}\n"),
        int(30)
    );
}

#[test]
fn a_function_may_be_called_before_it_is_defined() {
    assert_eq!(
        value("fn main() -> i32 {\n    return triple(4);\n}\n\nfn triple(n: i32) -> i32 {\n    return n * 3;\n}\n"),
        int(12)
    );
}

#[test]
fn arguments_are_bound_to_parameters_in_order() {
    assert_eq!(
        value("fn sub(a: i32, b: i32) -> i32 {\n    return a - b;\n}\n\nfn main() -> i32 {\n    return sub(10, 3);\n}\n"),
        int(7)
    );
}

#[test]
fn arguments_are_evaluated_left_to_right_before_the_call() {
    // The first argument's assignment runs before the second's.
    assert_eq!(
        value("fn pair(a: i32, b: i32) -> i32 {\n    return a * 100 + b;\n}\n\nfn main() -> i32 {\n    let mut x = 0;\n    return pair(x = x + 1, x = x * 10);\n}\n"),
        int(110)
    );
}

#[test]
fn arguments_can_be_any_expression_including_calls() {
    assert_eq!(
        value("fn add(a: i32, b: i32) -> i32 {\n    return a + b;\n}\n\nfn main() -> i32 {\n    return add(add(1, 2), add(3 * 4, -2));\n}\n"),
        int(13)
    );
}

#[test]
fn arguments_are_passed_by_value() {
    // The callee changes its own copy of the argument, in its own frame;
    // the caller's variable is untouched. (Parameters are immutable, so
    // the copy is made explicit with a `let mut`.)
    assert_eq!(
        value("fn clobber(n: i32) -> i32 {\n    let mut copy = n;\n    copy = 0;\n    return copy;\n}\n\nfn main() -> i32 {\n    let n = 5;\n    clobber(n);\n    return n;\n}\n"),
        int(5)
    );
}

#[test]
fn each_call_gets_fresh_local_variables() {
    assert_eq!(
        value("fn counter() -> i32 {\n    let mut count = 0;\n    count = count + 1;\n    return count;\n}\n\nfn main() -> i32 {\n    return counter() + counter() + counter();\n}\n"),
        int(3)
    );
}

#[test]
fn a_call_used_as_a_statement_discards_its_value() {
    assert_eq!(
        value(
            "fn five() -> i32 {\n    return 5;\n}\n\nfn main() -> i32 {\n    five();\n    return 1;\n}\n"
        ),
        int(1)
    );
}

// --- return values ---

#[test]
fn a_function_without_return_produces_unit() {
    assert_eq!(
        value("fn nothing() {\n    let x = 1;\n}\n\nfn main() {\n    return nothing();\n}\n"),
        Value::Unit
    );
    assert_eq!(value("fn main() {\n    let x = 1;\n}\n"), Value::Unit);
}

#[test]
fn a_bare_return_produces_unit() {
    assert_eq!(value("fn main() {\n    return;\n}\n"), Value::Unit);
}

#[test]
fn main_can_return_any_kind_of_value() {
    assert_eq!(
        main_returning("bool", "return 1 == 1;"),
        Value::Boolean(true)
    );
    assert_eq!(main_returning("string", "return \"done\";"), string("done"));
    assert_eq!(main_returning("i64", "return 7;"), long(7));
}

#[test]
fn code_after_return_never_runs() {
    // If the division ran, this would be a runtime error.
    assert_eq!(main_returns("return 1;\nlet x = 1 / 0;"), int(1));
}

#[test]
fn return_leaves_nested_blocks_and_loops() {
    assert_eq!(
        value("fn first_multiple_of_seven(start: i32) -> i32 {\n    let mut i = start;\n    while true {\n        if i / 7 * 7 == i {\n            return i;\n        }\n        i = i + 1;\n    }\n    return 0 - 1;\n}\n\nfn main() -> i32 {\n    return first_multiple_of_seven(30);\n}\n"),
        int(35)
    );
}

#[test]
fn return_ends_only_the_call_it_is_in() {
    assert_eq!(
        value("fn inner() -> i32 {\n    return 1;\n}\n\nfn main() -> i32 {\n    let a = inner();\n    let b = inner();\n    return a + b + 10;\n}\n"),
        int(12)
    );
}

#[test]
fn a_return_inside_an_else_branch_skips_the_rest_of_the_function() {
    assert_eq!(
        value("fn pick(flag: bool) -> i32 {\n    if flag {\n        let ignored = 0;\n    } else {\n        return 2;\n    }\n    return 1;\n}\n\nfn main() -> i32 {\n    return pick(true) * 10 + pick(false);\n}\n"),
        int(12)
    );
}

// --- runtime errors ---

#[test]
fn division_by_zero_is_a_runtime_error() {
    let source = "fn main() -> i32 {\n    let zero = 0;\n    return 10 / zero;\n}\n";
    let error = runtime_error(source);
    assert_eq!(error.kind, RuntimeErrorKind::DivisionByZero);
    // Reported at the divisor.
    assert_eq!(error.span.start, source.find("zero;").unwrap());
}

fn overflow(operator: &str, ty: &'static str) -> RuntimeErrorKind {
    RuntimeErrorKind::IntegerOverflow {
        operator: operator.to_string(),
        ty,
    }
}

#[test]
fn i32_overflow_is_a_runtime_error_not_a_wrap() {
    assert_eq!(
        main_fails_with("return 2147483647 + 1;"),
        overflow("+", "i32")
    );
    assert_eq!(
        main_fails_with("return -2147483647 - 2;"),
        overflow("-", "i32")
    );
    assert_eq!(
        main_fails_with("return 65536 * 65536;"),
        overflow("*", "i32")
    );
    // The most negative `i32` has no positive counterpart.
    assert_eq!(
        main_fails_with("let min = -2147483648;\nreturn -min;"),
        overflow("-", "i32")
    );
    assert_eq!(
        main_fails_with("let min = -2147483648;\nreturn min / -1;"),
        overflow("/", "i32")
    );
}

#[test]
fn i64_overflow_is_a_runtime_error_not_a_wrap() {
    let i64_fails_with = |body| main_returning_fails_with("i64", body);
    assert_eq!(
        i64_fails_with("let max: i64 = 9223372036854775807;\nreturn max + 1;"),
        overflow("+", "i64")
    );
    assert_eq!(
        i64_fails_with("return -9223372036854775807 - 2;"),
        overflow("-", "i64")
    );
    assert_eq!(
        i64_fails_with("let big: i64 = 3037000500;\nreturn big * big;"),
        overflow("*", "i64")
    );
    assert_eq!(
        i64_fails_with("let min: i64 = -9223372036854775807 - 1;\nreturn -min;"),
        overflow("-", "i64")
    );
    assert_eq!(
        i64_fails_with("let min: i64 = -9223372036854775807 - 1;\nreturn min / -1;"),
        overflow("/", "i64")
    );
}

#[test]
fn a_sum_that_overflows_i32_fits_in_i64() {
    assert_eq!(
        main_returning("i64", "return 2147483647 + 1;"),
        long(2_147_483_648)
    );
}

#[test]
fn the_most_negative_integers_are_reachable() {
    // `-2147483648` is a single literal: its digits alone would not fit.
    assert_eq!(main_returns("return -2147483648;"), int(i32::MIN));
    assert_eq!(
        main_returning("i64", "return -9223372036854775807 - 1;"),
        long(i64::MIN)
    );
    // And the smallest `i64` is a single literal too.
    assert_eq!(
        main_returning("i64", "return -9223372036854775808;"),
        long(i64::MIN)
    );
    assert_eq!(
        main_returning_fails_with(
            "i64",
            "let min: i64 = -9223372036854775808;\nreturn min / -1;"
        ),
        overflow("/", "i64")
    );
}

#[test]
fn division_by_zero_is_a_runtime_error_for_both_integer_types() {
    assert_eq!(
        main_returning_fails_with("i64", "let zero: i64 = 0;\nreturn 10 / zero;"),
        RuntimeErrorKind::DivisionByZero
    );
}

#[test]
fn a_runtime_error_stops_the_program() {
    // The error in `fail` ends the whole run: `main` never returns 1.
    let error = runtime_error(
        "fn fail() -> i32 {\n    return 1 / 0;\n}\n\nfn main() -> i32 {\n    fail();\n    return 1;\n}\n",
    );
    assert_eq!(error.kind, RuntimeErrorKind::DivisionByZero);
}

#[test]
fn a_runtime_error_records_the_call_stack() {
    let error = runtime_error("fn divide(a: i32, b: i32) -> i32 {\n    return a / b;\n}\n\nfn main() -> i32 {\n    return divide(10, 0);\n}\n");
    assert_eq!(
        error.call_stack,
        vec!["divide".to_string(), "main".to_string()]
    );
}

#[test]
fn runtime_errors_render_with_their_own_label_and_call_stack() {
    let source = "fn main() -> i32 {\n    return 2147483647 + 1;\n}\n";
    let rendered = runtime_error(source).render("example.dk", source);
    assert!(rendered
        .starts_with("runtime error: integer overflow: the result of `+` does not fit in `i32`\n"));
    assert!(rendered.contains(" --> example.dk:2:12"));
    assert!(rendered.ends_with("note: call stack: main\n"));
}

// --- the interpreter's own guards ---
//
// Applying an operator to the wrong kinds of value, or branching on
// something that is not a boolean, is a type error: the type checker
// rejects such programs, so `dijk run` never runs them. The interpreter
// does not rely on that. Handed such a program directly, skipping the
// earlier stages, it reports a runtime error instead of crashing.

const TYPE_ERRORS: [&str; 8] = [
    "return 1 + true;",
    "return \"a\" - \"b\";",
    "return 1 == true;",
    "return \"a\" < \"b\";",
    "return -true;",
    "return !1;",
    "if 5 {\n    return 1;\n}",
    "while \"yes\" {\n    return 1;\n}",
];

#[test]
fn operations_on_the_wrong_kinds_of_value_are_type_errors_first() {
    for body in TYPE_ERRORS {
        let program = parse(&format!("fn main() {{\n{body}\n}}\n"));
        assert!(types::check(&program).is_err(), "`{body}` type-checks");
    }
}

#[test]
fn unchecked_adding_an_integer_and_a_boolean_is_an_invalid_operation() {
    assert_eq!(
        unchecked_main_fails_with(TYPE_ERRORS[0]),
        invalid_binary("+", "i32", "bool")
    );
}

#[test]
fn unchecked_subtracting_strings_is_an_invalid_operation() {
    assert_eq!(
        unchecked_main_fails_with(TYPE_ERRORS[1]),
        invalid_binary("-", "string", "string")
    );
}

#[test]
fn unchecked_comparing_values_of_different_kinds_is_an_invalid_operation() {
    assert_eq!(
        unchecked_main_fails_with(TYPE_ERRORS[2]),
        invalid_binary("==", "i32", "bool")
    );
    assert_eq!(
        unchecked_main_fails_with("return \"1\" != 1;"),
        invalid_binary("!=", "string", "i32")
    );
}

#[test]
fn unchecked_ordering_is_only_defined_for_integers() {
    assert_eq!(
        unchecked_main_fails_with(TYPE_ERRORS[3]),
        invalid_binary("<", "string", "string")
    );
    // `1 < 2 < 3` parses as `(1 < 2) < 3`: a boolean compared with an integer.
    assert_eq!(
        unchecked_main_fails_with("return 1 < 2 < 3;"),
        invalid_binary("<", "bool", "i32")
    );
}

#[test]
fn unchecked_invalid_unary_operations() {
    assert_eq!(
        unchecked_main_fails_with(TYPE_ERRORS[4]),
        RuntimeErrorKind::InvalidUnaryOperation {
            operator: "-".to_string(),
            operand: "bool"
        }
    );
    assert_eq!(
        unchecked_main_fails_with(TYPE_ERRORS[5]),
        RuntimeErrorKind::InvalidUnaryOperation {
            operator: "!".to_string(),
            operand: "i32"
        }
    );
}

#[test]
fn unchecked_using_a_unit_value_is_an_invalid_operation() {
    let error = run_unchecked("fn nothing() {}\n\nfn main() {\n    return nothing() + 1;\n}\n")
        .unwrap_err();
    assert_eq!(error.kind, invalid_binary("+", "unit", "i32"));
}

#[test]
fn unchecked_a_condition_must_be_a_boolean() {
    assert_eq!(
        unchecked_main_fails_with(TYPE_ERRORS[6]),
        RuntimeErrorKind::InvalidCondition { found: "i32" }
    );
    assert_eq!(
        unchecked_main_fails_with(TYPE_ERRORS[7]),
        RuntimeErrorKind::InvalidCondition { found: "string" }
    );
}

#[test]
fn mixing_i32_and_i64_values_is_an_invalid_operation() {
    // No checked program can do this. Hand the interpreter type records
    // that make the left literal an `i32` and the right one an `i64`.
    let source = "fn main() {\n    return 1 + 2;\n}\n";
    let program = parse(source);
    let mut types = TypedProgram::default();
    let two = source.find('2').unwrap();
    types
        .expression_types
        .insert(lexer::token::Span::new(two, two + 1), types::ty::Type::I64);
    assert_eq!(
        interpreter::run(&program, &types).unwrap_err().kind,
        invalid_binary("+", "i32", "i64")
    );
}

#[test]
fn unchecked_an_integer_literal_that_does_not_fit_is_reported() {
    assert_eq!(
        unchecked_main_fails_with("return 3000000000;"),
        RuntimeErrorKind::IntegerLiteralOutOfRange {
            value: 3000000000,
            ty: "i32"
        }
    );
}

// --- errors caught before the program runs ---
//
// A missing function or an unknown variable is a *semantic* error: the
// program is rejected before it runs, so it can never reach the
// interpreter through `dijk run`.

#[test]
fn a_missing_function_is_a_semantic_error_not_a_runtime_error() {
    let program = parse("fn main() -> i32 {\n    return missing(1);\n}\n");
    assert_eq!(
        semantic::analyze(&program)
            .into_iter()
            .map(|e| e.kind)
            .collect::<Vec<_>>(),
        vec![SemanticErrorKind::UnknownFunction {
            name: "missing".to_string()
        }]
    );
}

#[test]
fn an_unknown_variable_is_a_semantic_error_not_a_runtime_error() {
    let program = parse("fn main() -> i32 {\n    return unknown;\n}\n");
    assert_eq!(
        semantic::analyze(&program)
            .into_iter()
            .map(|e| e.kind)
            .collect::<Vec<_>>(),
        vec![SemanticErrorKind::UnknownIdentifier {
            name: "unknown".to_string()
        }]
    );
}

// The interpreter does not rely on never seeing such a program. Handed
// one directly, skipping semantic analysis (and type checking, so with
// no type records at all), it reports a runtime error instead of
// crashing.

#[test]
fn unchecked_program_missing_function_is_still_reported() {
    let program = parse("fn main() -> i32 {\n    return missing(1);\n}\n");
    assert_eq!(
        interpreter::run(&program, &TypedProgram::default())
            .unwrap_err()
            .kind,
        RuntimeErrorKind::UndefinedFunction {
            name: "missing".to_string()
        }
    );
}

#[test]
fn unchecked_program_invalid_variable_access_is_still_reported() {
    let program = parse(
        "fn main() -> i32 {\n    if true {\n        let inner = 1;\n    }\n    return inner;\n}\n",
    );
    assert_eq!(
        interpreter::run(&program, &TypedProgram::default())
            .unwrap_err()
            .kind,
        RuntimeErrorKind::UndefinedVariable {
            name: "inner".to_string()
        }
    );
}

#[test]
fn unchecked_program_wrong_argument_count_is_still_reported() {
    let program = parse(
        "fn one(a: i32) -> i32 {\n    return a;\n}\n\nfn main() -> i32 {\n    return one();\n}\n",
    );
    assert_eq!(
        interpreter::run(&program, &TypedProgram::default())
            .unwrap_err()
            .kind,
        RuntimeErrorKind::ArityMismatch {
            name: "one".to_string(),
            expected: 1,
            found: 0,
        }
    );
}

#[test]
fn unchecked_program_without_main_is_still_reported() {
    let program = parse("fn helper() {}\n");
    assert_eq!(
        interpreter::run(&program, &TypedProgram::default())
            .unwrap_err()
            .kind,
        RuntimeErrorKind::MissingMain
    );
}

// --- recursion ---
//
// Each call runs in a frame of its own, so a function that calls itself
// needs no special machinery: every level of the recursion simply gets
// its own copy of every parameter and local.

const FACTORIAL: &str =
    "fn factorial(n: i32) -> i32 {\n    if n <= 1 {\n        return 1;\n    }\n\n    return n * factorial(n - 1);\n}\n";

fn factorial_of(n: i64) -> Result<Value, RuntimeError> {
    run(&format!(
        "{FACTORIAL}\nfn main() -> i32 {{\n    return factorial({n});\n}}\n"
    ))
}

#[test]
fn factorial_of_5_is_120() {
    // Step 4 §16, verbatim.
    assert_eq!(factorial_of(5), Ok(int(120)));
}

#[test]
fn factorial_base_cases() {
    assert_eq!(factorial_of(0), Ok(int(1)));
    assert_eq!(factorial_of(1), Ok(int(1)));
}

#[test]
fn factorial_of_12_is_the_largest_that_fits_in_i32() {
    assert_eq!(factorial_of(12), Ok(int(479_001_600)));
}

#[test]
fn factorial_of_13_overflows_i32_instead_of_wrapping() {
    let error = factorial_of(13).unwrap_err();
    assert_eq!(error.kind, overflow("*", "i32"));
    // The overflowing multiplication is `13 * factorial(12)`, in the
    // outermost call to `factorial`.
    assert_eq!(
        error.call_stack,
        vec!["factorial".to_string(), "main".to_string()]
    );
}

/// The same factorial, over `i64`.
const FACTORIAL_I64: &str =
    "fn factorial(n: i64) -> i64 {\n    if n <= 1 {\n        return 1;\n    }\n\n    return n * factorial(n - 1);\n}\n";

fn factorial_i64_of(n: i64) -> Result<Value, RuntimeError> {
    run(&format!(
        "{FACTORIAL_I64}\nfn main() -> i64 {{\n    return factorial({n});\n}}\n"
    ))
}

#[test]
fn factorial_of_20_is_the_largest_that_fits_in_i64() {
    assert_eq!(factorial_i64_of(20), Ok(long(2_432_902_008_176_640_000)));
}

#[test]
fn factorial_of_21_overflows_i64_instead_of_wrapping() {
    assert_eq!(factorial_i64_of(21).unwrap_err().kind, overflow("*", "i64"));
}

const FIBONACCI: &str = "fn fib(n: i32) -> i32 {\n    if n < 2 {\n        return n;\n    }\n    return fib(n - 1) + fib(n - 2);\n}\n";

#[test]
fn fibonacci() {
    for (n, expected) in [(0, 0), (1, 1), (2, 1), (10, 55)] {
        assert_eq!(
            value(&format!(
                "{FIBONACCI}\nfn main() -> i32 {{\n    return fib({n});\n}}\n"
            )),
            int(expected)
        );
    }
}

#[test]
fn fibonacci_makes_many_calls_but_never_many_at_once() {
    // fib(20) makes 21,891 calls in total, but at most 21 are in
    // progress at any moment. The depth limit counts the latter.
    assert_eq!(
        value(&format!(
            "{FIBONACCI}\nfn main() -> i32 {{\n    return fib(20);\n}}\n"
        )),
        int(6765)
    );
}

#[test]
fn mutual_recursion() {
    let program = "fn is_even(n: i32) -> bool {\n    if n == 0 {\n        return true;\n    }\n    return is_odd(n - 1);\n}\n\nfn is_odd(n: i32) -> bool {\n    if n == 0 {\n        return false;\n    }\n    return is_even(n - 1);\n}\n";
    for (call, expected) in [
        ("is_even(10)", true),
        ("is_odd(7)", true),
        ("is_even(101)", false),
    ] {
        assert_eq!(
            value(&format!(
                "{program}\nfn main() -> bool {{\n    return {call};\n}}\n"
            )),
            Value::Boolean(expected)
        );
    }
}

#[test]
fn each_recursive_call_keeps_its_own_variables() {
    // `n` is read again *after* the recursive call returns: if the
    // calls shared one variable, every level would see the deepest `n`.
    assert_eq!(
        value("fn sum_to(n: i32) -> i32 {\n    if n == 0 {\n        return 0;\n    }\n    let rest = sum_to(n - 1);\n    return n + rest;\n}\n\nfn main() -> i32 {\n    return sum_to(100);\n}\n"),
        int(5050)
    );
}

#[test]
fn recursion_through_a_loop_and_a_branch() {
    // Counts the nodes of a complete binary tree of the given height,
    // recursing from inside a `while` nested in an `if`.
    assert_eq!(
        value("fn nodes(height: i32) -> i32 {\n    if height == 0 {\n        return 1;\n    } else {\n        let mut total = 1;\n        let mut child = 0;\n        while child < 2 {\n            total = total + nodes(height - 1);\n            child = child + 1;\n        }\n        return total;\n    }\n}\n\nfn main() -> i32 {\n    return nodes(5);\n}\n"),
        int(63)
    );
}

// --- the call-depth limit ---
//
// At most 1000 calls may be in progress at once, `main` included (the
// interpreter's `MAX_CALL_DEPTH`).

const COUNTDOWN: &str = "fn down(n: i32) -> i32 {\n    if n == 0 {\n        return 0;\n    }\n    return down(n - 1);\n}\n";

fn countdown_from(n: i64) -> Result<Value, RuntimeError> {
    run(&format!(
        "{COUNTDOWN}\nfn main() -> i32 {{\n    return down({n});\n}}\n"
    ))
}

#[test]
fn recursion_up_to_the_depth_limit_succeeds() {
    // main + down(998) … down(0) = 1000 calls in progress.
    assert_eq!(countdown_from(998), Ok(int(0)));
}

#[test]
fn recursion_past_the_depth_limit_is_a_runtime_error() {
    // One more level would be the 1001st call in progress.
    let error = countdown_from(999).unwrap_err();
    assert_eq!(
        error.kind,
        RuntimeErrorKind::CallDepthExceeded { limit: 1000 }
    );
}

#[test]
fn infinite_recursion_hits_the_depth_limit_instead_of_crashing() {
    let source = "fn forever(n: i32) -> i32 {\n    return forever(n + 1);\n}\n\nfn main() -> i32 {\n    return forever(0);\n}\n";
    let error = runtime_error(source);
    assert_eq!(
        error.kind,
        RuntimeErrorKind::CallDepthExceeded { limit: 1000 }
    );
    // Reported at the call that would have gone one level too deep.
    assert_eq!(error.span.start, source.find("forever(n + 1)").unwrap());
    // Every frame in progress is recorded: 999 `forever`s, then `main`.
    assert_eq!(error.call_stack.len(), 1000);
    assert_eq!(
        error.call_stack.first().map(String::as_str),
        Some("forever")
    );
    assert_eq!(error.call_stack.last().map(String::as_str), Some("main"));
}

#[test]
fn infinite_mutual_recursion_hits_the_depth_limit() {
    let error = runtime_error("fn ping() -> i32 {\n    return pong();\n}\n\nfn pong() -> i32 {\n    return ping();\n}\n\nfn main() -> i32 {\n    return ping();\n}\n");
    assert_eq!(
        error.kind,
        RuntimeErrorKind::CallDepthExceeded { limit: 1000 }
    );
}

#[test]
fn a_deep_call_stack_is_abbreviated_when_rendered() {
    let source = "fn forever() -> i32 {\n    return forever();\n}\n\nfn main() -> i32 {\n    return forever();\n}\n";
    let rendered = runtime_error(source).render("example.dk", source);
    assert!(rendered
        .starts_with("runtime error: stack overflow: more than 1000 nested function calls\n"));
    assert!(rendered.ends_with(
        "note: call stack: forever ← forever ← forever ← forever ← … 995 more … ← main\n"
    ));
}

#[test]
fn deep_recursion_within_the_limit_unwinds_correctly() {
    // Deep, but within the limit, and with work on the way back up.
    assert_eq!(
        value("fn depth_sum(n: i32) -> i32 {\n    if n == 0 {\n        return 0;\n    }\n    return n + depth_sum(n - 1);\n}\n\nfn main() -> i32 {\n    return depth_sum(900);\n}\n"),
        int(405_450)
    );
}
