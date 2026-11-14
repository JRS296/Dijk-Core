//! The pipeline: type checking is a stage of its own, between semantic
//! analysis and execution, and a program with a type error never reaches
//! the interpreter.

use super::*;
use interpreter::error::RuntimeError;
use interpreter::value::Value;
use std::process::Command;

/// How far a program got through `dijk run`'s pipeline.
#[derive(Debug)]
enum Outcome {
    SemanticErrors,
    TypeErrors(Vec<&'static str>),
    Ran(Result<Value, RuntimeError>),
}

/// Take `source` through the same stages, in the same order and with the
/// same stopping rules, as `dijk run`.
fn run_like_dijk(source: &str) -> Outcome {
    let program = parse_syntax(source);
    if !semantic::analyze(&program).is_empty() {
        return Outcome::SemanticErrors;
    }
    match types::check(&program) {
        Err(errors) => Outcome::TypeErrors(errors.iter().map(|e| e.code()).collect()),
        Ok(typed) => Outcome::Ran(interpreter::run(&program, &typed)),
    }
}

/// A program that would fail at run time *first*, dividing by zero, if it
/// were ever run, followed by a type error.
const DIVIDES_THEN_MISTYPES: &str = "fn main() -> i32 {\n    let zero = 0;\n    let crash = 1 / zero;\n    let wrong: bool = 5;\n    return crash;\n}\n";

#[test]
fn a_type_invalid_program_never_reaches_the_interpreter() {
    // Had it run, the division would have failed before the type error's
    // line was reached. It did not run: the type error is all there is.
    match run_like_dijk(DIVIDES_THEN_MISTYPES) {
        Outcome::TypeErrors(codes) => assert_eq!(codes, vec!["E0001"]),
        other => panic!("expected type errors, got {other:?}"),
    }
}

#[test]
fn dijk_run_itself_stops_at_the_type_checker() {
    let path = std::env::temp_dir().join("dijk-type-tests-divides-then-mistypes.dk");
    std::fs::write(&path, DIVIDES_THEN_MISTYPES).expect("failed to write a scratch file");
    let output = Command::new(env!("CARGO_BIN_EXE_dijk"))
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect("failed to run the dijk binary");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.starts_with("type error[E0001]: type mismatch\n"));
    assert!(!stderr.contains("runtime error"));
}

#[test]
fn a_well_typed_program_runs() {
    let source = "fn add(a: i32, b: i32) -> i32 {\n    let result = a + b;\n    return result;\n}\n\nfn main() -> i32 {\n    return add(10, 20);\n}\n";
    match run_like_dijk(source) {
        Outcome::Ran(result) => assert_eq!(result, Ok(Value::I32(30))),
        other => panic!("expected the program to run, got {other:?}"),
    }
}

#[test]
fn a_well_typed_program_can_still_fail_at_run_time() {
    // Types say which operations are valid, not what values will appear.
    match run_like_dijk("fn main() -> i32 {\n    let zero = 0;\n    return 1 / zero;\n}\n") {
        Outcome::Ran(Err(error)) => assert_eq!(
            error.kind,
            interpreter::error::RuntimeErrorKind::DivisionByZero
        ),
        other => panic!("expected a runtime error, got {other:?}"),
    }
}

#[test]
fn a_program_with_semantic_errors_never_reaches_the_type_checker() {
    // Both an unknown name and a type mistake: only the first is reported,
    // by the stage that runs first.
    assert!(matches!(
        run_like_dijk("fn main() {\n    let x: i32 = \"s\";\n    return unknown;\n}\n"),
        Outcome::SemanticErrors
    ));
}
