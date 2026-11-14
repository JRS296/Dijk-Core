//! End-to-end tests that run the built `dijk` binary, rather than calling
//! the compiler's modules directly. These cover CLI-level concerns that
//! the module-level tests don't: file extension handling, the actual
//! `dijk lex` / `dijk parse` / `dijk run` output a user would see, and
//! exit codes.

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dijk"))
        .args(args)
        .output()
        .expect("failed to run the dijk binary")
}

fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr_of(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

/// Write `source` to a scratch `.dk` file in the system's temporary
/// directory and `dijk run` it, for programs not worth keeping as a
/// fixture in `examples/`.
fn run_source(name: &str, source: &str) -> std::process::Output {
    let path = std::env::temp_dir().join(format!("dijk-cli-test-{name}.dk"));
    std::fs::write(&path, source).expect("failed to write a scratch source file");
    run(&["run", path.to_str().unwrap()])
}

#[test]
fn lex_accepts_dk_and_dijk_identically() {
    let dk = run(&["lex", "examples/hello.dk"]);
    let dijk = run(&["lex", "examples/hello.dijk"]);
    assert!(dk.status.success());
    assert!(dijk.status.success());
    assert_eq!(dk.stdout, dijk.stdout);
}

#[test]
fn parse_accepts_dk_and_dijk_identically() {
    let dk = run(&["parse", "examples/hello.dk"]);
    let dijk = run(&["parse", "examples/hello.dijk"]);
    assert!(dk.status.success());
    assert!(dijk.status.success());
    assert_eq!(dk.stdout, dijk.stdout);
}

#[test]
fn parse_rejects_files_with_other_extensions() {
    let output = run(&["parse", "examples/hello.dk.bak"]);
    assert!(!output.status.success());
}

#[test]
fn parse_prints_a_syntax_tree_for_a_valid_program() {
    let output = run(&["parse", "examples/hello.dk"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("Program\n"));
    assert!(stdout.contains("Function"));
    assert!(stdout.contains("Name: main"));
}

#[test]
fn parse_reports_a_diagnostic_and_exits_nonzero_on_a_broken_program() {
    let output = run(&["parse", "examples/parse_error.dk"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("error:"));
}

// --- dijk run ---

#[test]
fn run_prints_the_value_main_returns_for_dk_and_dijk() {
    let dk = run(&["run", "examples/hello.dk"]);
    let dijk = run(&["run", "examples/hello.dijk"]);
    assert_eq!(dk.status.code(), Some(0));
    assert_eq!(dijk.status.code(), Some(0));
    assert_eq!(stdout_of(&dk), "52\n");
    assert_eq!(dk.stdout, dijk.stdout);
}

#[test]
fn run_executes_recursion() {
    let output = run(&["run", "examples/factorial.dk"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_of(&output), "120\n");
}

#[test]
fn run_prints_nothing_when_main_returns_no_value() {
    let output = run_source("unit-main", "fn main() {\n    let x = 1;\n}\n");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_of(&output), "");
}

#[test]
fn run_reports_a_semantic_error_and_does_not_execute() {
    let output = run(&["run", "examples/semantic_error.dk"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_of(&output), "");
    let stderr = stderr_of(&output);
    assert!(stderr.starts_with("semantic error: unknown identifier `y`\n"));
    assert!(stderr.contains(" --> examples/semantic_error.dk:8:12"));
}

#[test]
fn run_reports_a_runtime_error_with_its_call_stack() {
    let output = run(&["run", "examples/runtime_error.dk"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_of(&output), "");
    let stderr = stderr_of(&output);
    assert!(stderr.starts_with("runtime error: division by zero\n"));
    assert!(stderr.contains(" --> examples/runtime_error.dk:2:16"));
    assert!(stderr.ends_with("note: call stack: divide ← main\n"));
}

#[test]
fn run_does_not_analyze_or_execute_a_program_with_parse_errors() {
    let output = run(&["run", "examples/parse_error.dk"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_of(&output), "");
    let stderr = stderr_of(&output);
    assert!(stderr.starts_with("error: expected parameter or ')'"));
    assert!(!stderr.contains("semantic error"));
}

#[test]
fn run_does_not_parse_a_program_with_lexical_errors() {
    let output = run(&["run", "examples/unexpected_character.dk"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_of(&output).starts_with("error: unexpected character '@'"));
}

#[test]
fn run_requires_a_file_argument() {
    let output = run(&["run"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_of(&output).contains("usage: dijk run <file>"));
}
