//! End-to-end tests that run the built `dijk` binary, rather than calling
//! the compiler's modules directly. These cover CLI-level concerns that
//! the module-level tests don't: file extension handling, the actual
//! `dijk lex` / `dijk parse` / `dijk check` / `dijk run` output a user
//! would see, and exit codes.

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
/// directory, for programs not worth keeping as a fixture in `examples/`,
/// and return its path.
fn scratch_file(name: &str, source: &str) -> String {
    let path = std::env::temp_dir().join(format!("dijk-cli-test-{name}.dk"));
    std::fs::write(&path, source).expect("failed to write a scratch source file");
    path.to_str().unwrap().to_string()
}

/// `dijk run` a scratch file holding `source`.
fn run_source(name: &str, source: &str) -> std::process::Output {
    run(&["run", &scratch_file(name, source)])
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

#[test]
fn parse_and_ast_show_which_let_bindings_are_mutable() {
    let path = scratch_file(
        "let-mut",
        "fn main() {\n    let mut count = 0;\n    let limit = 10;\n}\n",
    );
    for command in ["parse", "ast"] {
        let output = run(&[command, &path]);
        assert!(output.status.success());
        let stdout = stdout_of(&output);
        // Only the `let mut` gets the extra line, right after its name.
        let lines: Vec<&str> = stdout.lines().collect();
        let name = lines
            .iter()
            .position(|line| line.ends_with("Name: count"))
            .unwrap();
        assert!(lines[name + 1].ends_with("├── Mutable: true"));
        assert_eq!(stdout.matches("Mutable: true").count(), 1);
    }
}

// --- dijk check ---

const WELL_TYPED_REPORT: &str = "Lexical analysis       OK\nParsing                OK\nSemantic analysis      OK\nType checking          OK\n\nProgram is well-typed.\n";

#[test]
fn check_reports_every_stage_for_a_well_typed_program() {
    let output = run(&["check", "examples/add.dk"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout_of(&output),
        format!("Checking examples/add.dk...\n{WELL_TYPED_REPORT}")
    );
    assert_eq!(stderr_of(&output), "");
}

#[test]
fn check_accepts_dk_and_dijk_identically() {
    let dk = run(&["check", "examples/hello.dk"]);
    let dijk = run(&["check", "examples/hello.dijk"]);
    assert_eq!(dk.status.code(), Some(0));
    assert_eq!(dijk.status.code(), Some(0));
    assert!(stdout_of(&dk).ends_with(WELL_TYPED_REPORT));
    assert!(stdout_of(&dijk).ends_with(WELL_TYPED_REPORT));
}

#[test]
fn check_fails_with_exit_1_on_a_type_error() {
    let output = run(&["check", "examples/type_error.dk"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stdout_of(&output),
        "Checking examples/type_error.dk...\nLexical analysis       OK\nParsing                OK\nSemantic analysis      OK\nType checking          FAILED (1 error)\n"
    );
    assert_eq!(
        stderr_of(&output),
        "type error[E0001]: type mismatch\n --> examples/type_error.dk:2:18\n  |\n2 |     let x: i32 = \"hello\";\n  |                  ^^^^^^^\n  = expected `i32`\n  = found `string`\n"
    );
}

#[test]
fn check_stops_at_the_first_stage_that_fails() {
    for (fixture, last_line) in [
        (
            "examples/unexpected_character.dk",
            "Lexical analysis       FAILED (1 error)\n",
        ),
        (
            "examples/parse_error.dk",
            "Parsing                FAILED (1 error)\n",
        ),
        (
            "examples/semantic_error.dk",
            "Semantic analysis      FAILED (1 error)\n",
        ),
    ] {
        let output = run(&["check", fixture]);
        assert_eq!(output.status.code(), Some(1), "{fixture}");
        let stdout = stdout_of(&output);
        assert!(stdout.ends_with(last_line), "{fixture}: {stdout}");
        assert!(!stdout.contains("Type checking"), "{fixture}");
    }
}

#[test]
fn check_never_runs_the_program() {
    // Well-typed, but dividing by zero when run: `check` accepts it.
    let output = run(&["check", "examples/runtime_error.dk"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout_of(&output).ends_with("Program is well-typed.\n"));
    assert!(!stderr_of(&output).contains("runtime error"));
}

#[test]
fn check_reports_every_type_error() {
    let path = scratch_file(
        "two-type-errors",
        "fn main() -> i32 {\n    let a: bool = 1;\n    return \"a\" + 1;\n}\n",
    );
    let output = run(&["check", &path]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stdout_of(&output).ends_with("Type checking          FAILED (2 errors)\n"));
    let stderr = stderr_of(&output);
    assert_eq!(stderr.matches("type error[").count(), 2);
    assert!(stderr.starts_with("type error[E0001]: type mismatch\n"));
    assert!(stderr.contains("type error[E0002]: invalid binary operation: `+`\n"));
}

#[test]
fn check_requires_a_dijk_source_file() {
    let output = run(&["check"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_of(&output).contains("usage: dijk check [--types] <file>"));
    let output = run(&["check", "examples/hello.txt"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_of(&output).contains("is not a Dijk source file"));
}

#[test]
fn check_shows_windows_line_endings_without_a_carriage_return() {
    let path = scratch_file(
        "crlf-type-error",
        "fn main() -> i32 {\r\n    let x: i32 = \"hello\";\r\n    return x;\r\n}\r\n",
    );
    let output = run(&["check", &path]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(!stderr.contains('\r'), "{stderr:?}");
    assert!(stderr.contains("2 |     let x: i32 = \"hello\";\n  |                  ^^^^^^^\n"));
}

#[test]
fn a_non_ascii_character_is_reported_once() {
    let output = run_source("non-ascii", "fn main() {\n    let café = 1;\n}\n");
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert_eq!(stderr.matches("error:").count(), 1, "{stderr}");
    assert!(stderr.starts_with("error: unexpected character 'é' (U+00E9)\n"));
    assert!(stderr.contains(" --> "));
    assert!(stderr.ends_with("2 |     let café = 1;\n  |            ^\n"));
}

// --- dijk check --types ---

#[test]
fn check_types_prints_signatures_and_bindings() {
    let output = run(&["check", "--types", "examples/add.dk"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout_of(&output),
        format!(
            "Checking examples/add.dk...\n{WELL_TYPED_REPORT}\nfn add(a: i32, b: i32) -> i32\n    a      : i32   (1:8)\n    b      : i32   (1:16)\n    result : i32   (2:9)\nfn main() -> i32\n"
        )
    );
}

#[test]
fn check_types_accepts_dk_and_dijk_identically() {
    let dk = run(&["check", "--types", "examples/hello.dk"]);
    let dijk = run(&["check", "--types", "examples/hello.dijk"]);
    assert_eq!(dk.status.code(), Some(0));
    assert_eq!(dijk.status.code(), Some(0));
    let listing = "fn main() -> i32\n    x : i32   (2:9)\n    y : i32   (3:9)\n";
    assert!(stdout_of(&dk).ends_with(listing));
    assert!(stdout_of(&dijk).ends_with(listing));
}

#[test]
fn check_types_shows_the_types_literals_were_given() {
    // With no context a literal is an `i32`; an annotation, or an `i64`
    // on the other side of an operator, makes it an `i64`.
    let path = scratch_file(
        "inferred-literals",
        "fn main() -> i64 {\n    let small = 1;\n    let big: i64 = 2;\n    let sum = big + 3;\n    return sum;\n}\n",
    );
    let output = run(&["check", "--types", &path]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout_of(&output).ends_with(
        "fn main() -> i64\n    small : i32   (2:9)\n    big   : i64   (3:9)\n    sum   : i64   (4:9)\n"
    ));
}

#[test]
fn check_types_lists_shadowed_bindings_separately_and_marks_mut() {
    let path = scratch_file(
        "shadowing",
        "fn main() {\n    let x = 1;\n    if x > 0 {\n        let x = \"inner\";\n        let mut count = 0;\n        count = count + 1;\n    }\n}\n",
    );
    let output = run(&["check", "--types", &path]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout_of(&output).ends_with(
        "fn main() -> unit\n    x         : i32      (2:9)\n        x     : string   (4:13)\n        count : i32      (5:17)  mut\n"
    ));
}

#[test]
fn check_types_accepts_the_flag_after_the_file_too() {
    let before = run(&["check", "--types", "examples/add.dk"]);
    let after = run(&["check", "examples/add.dk", "--types"]);
    assert_eq!(after.status.code(), Some(0));
    assert_eq!(before.stdout, after.stdout);
}

#[test]
fn check_types_prints_no_listing_for_a_program_with_type_errors() {
    let output = run(&["check", "--types", "examples/type_error.dk"]);
    assert_eq!(output.status.code(), Some(1));
    let stdout = stdout_of(&output);
    assert!(stdout.ends_with("Type checking          FAILED (1 error)\n"));
    assert!(!stdout.contains("fn main"));
    assert!(stderr_of(&output).starts_with("type error[E0001]: type mismatch\n"));
}

#[test]
fn check_rejects_unknown_options() {
    let output = run(&["check", "--typo", "examples/add.dk"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_of(&output).contains("unknown option '--typo'"));
    assert_eq!(stdout_of(&output), "");
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
fn run_rejects_a_type_error_before_executing_anything() {
    let output = run(&["run", "examples/type_error.dk"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_of(&output), "");
    assert!(stderr_of(&output).starts_with("type error[E0001]: type mismatch\n"));
}

#[test]
fn run_does_not_execute_a_program_with_a_type_error_anywhere() {
    // Running this would divide by zero first. It is never run: the type
    // error two lines later stops it before execution begins.
    let output = run_source(
        "type-error-after-runtime-error",
        "fn main() -> i32 {\n    let x = 1 / 0;\n    let y: i32 = \"late\";\n    return x;\n}\n",
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr_of(&output);
    assert!(stderr.starts_with("type error[E0001]:"));
    assert!(!stderr.contains("runtime error"));
}

#[test]
fn run_executes_a_well_typed_program() {
    let output = run(&["run", "examples/add.dk"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout_of(&output), "30\n");
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
