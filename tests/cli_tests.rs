//! End-to-end tests that run the built `dijk` binary, rather than calling
//! the lexer/parser modules directly. These cover CLI-level concerns that
//! the module-level tests don't: file extension handling and the actual
//! `dijk lex` / `dijk parse` output a user would see.

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_dijk"))
        .args(args)
        .output()
        .expect("failed to run the dijk binary")
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
