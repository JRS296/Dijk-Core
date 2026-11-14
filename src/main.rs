//! Dijk Core CLI.
//!
//! There is no compiler yet. Dijk Core can lex, parse and build an AST,
//! check what names refer to (semantic analysis), check that every
//! operation is valid for the types it is applied to (type checking),
//! and run a program with a tree-walking interpreter. This binary is the
//! entry point for observing each of those stages as it is built.

mod ast;
mod diagnostics;
mod interpreter;
mod lexer;
mod parser;
mod pretty;
mod semantic;
// The expression types the type checker records have no consumer yet:
// the interpreter does not yet tell `i32` values from `i64` ones.
#[allow(dead_code)]
mod types;

use interpreter::value::Value;

const VERSION: &str = "0.0.4";

const HELP: &str = "\
Dijk 0.0.4

Dijk Core is building its compiler one stage at a time. See docs/ for the
philosophy, design constraints, architecture, and roadmap.

USAGE:
    dijk [COMMAND]

COMMANDS:
    --version    Print the Dijk version
    --help       Print this message
    doctor       Report the current state of the toolchain
    lex <file>   Print the token stream for a Dijk source file (.dk or .dijk)
    parse <file> Print the AST for a Dijk source file (.dk or .dijk)
    ast <file>   Print the AST plus node counts, for the same file
    check <file> Check a program through every stage up to type checking,
                 without running it
    run <file>   Check a program, then run it and print the value its
                 main returns
";

fn main() {
    let mut args = std::env::args().skip(1);

    match args.next().as_deref() {
        None | Some("--help") | Some("-h") => print!("{HELP}"),
        Some("--version") | Some("-V") => println!("Dijk {VERSION}"),
        Some("doctor") => doctor(),
        Some("lex") => {
            let Some(path) = args.next() else {
                eprintln!("dijk: 'lex' requires a file argument");
                eprintln!("usage: dijk lex <file>");
                std::process::exit(1);
            };
            lex(&path);
        }
        Some("parse") => {
            let Some(path) = args.next() else {
                eprintln!("dijk: 'parse' requires a file argument");
                eprintln!("usage: dijk parse <file>");
                std::process::exit(1);
            };
            parse(&path);
        }
        Some("ast") => {
            let Some(path) = args.next() else {
                eprintln!("dijk: 'ast' requires a file argument");
                eprintln!("usage: dijk ast <file>");
                std::process::exit(1);
            };
            ast(&path);
        }
        Some("check") => {
            let Some(path) = args.next() else {
                eprintln!("dijk: 'check' requires a file argument");
                eprintln!("usage: dijk check <file>");
                std::process::exit(1);
            };
            check(&path);
        }
        Some("run") => {
            let Some(path) = args.next() else {
                eprintln!("dijk: 'run' requires a file argument");
                eprintln!("usage: dijk run <file>");
                std::process::exit(1);
            };
            run(&path);
        }
        Some(other) => {
            eprintln!("dijk: unrecognized argument '{other}'");
            eprintln!("run 'dijk --help' for usage");
            std::process::exit(1);
        }
    }
}

fn doctor() {
    println!("Dijk toolchain");
    println!("Version: {VERSION}");
    println!(
        "Status: Bootstrap development (lexer, parser, AST, semantic analysis, type checker, and interpreter implemented)"
    );
    println!("Compiler: Not yet implemented (programs run on the interpreter: dijk run)");
}

/// File extensions recognized as Dijk source. `.dk` is the short form for
/// everyday use; `.dijk` is the unambiguous long form. Both name the same
/// language.
const SOURCE_EXTENSIONS: [&str; 2] = ["dk", "dijk"];

fn is_dijk_source_file(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| SOURCE_EXTENSIONS.contains(&ext))
}

fn read_source_file(path: &str) -> String {
    if !is_dijk_source_file(path) {
        eprintln!("dijk: '{path}' is not a Dijk source file (expected .dk or .dijk)");
        std::process::exit(1);
    }

    match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("dijk: could not read '{path}': {err}");
            std::process::exit(1);
        }
    }
}

fn lex(path: &str) {
    let source = read_source_file(path);
    let (tokens, errors) = lexer::tokenize(&source);

    for token in &tokens {
        println!("{token}");
    }

    if !errors.is_empty() {
        eprintln!();
        for error in &errors {
            eprint!("{}", error.render(path, &source));
        }
        std::process::exit(1);
    }
}

fn parse(path: &str) {
    let source = read_source_file(path);
    let (tokens, lex_errors) = lexer::tokenize(&source);

    if !lex_errors.is_empty() {
        for error in &lex_errors {
            eprint!("{}", error.render(path, &source));
        }
        std::process::exit(1);
    }

    let (program, parse_errors) = parser::parse(tokens);
    print!("{}", pretty::print_program(&program));

    if !parse_errors.is_empty() {
        eprintln!();
        for error in &parse_errors {
            eprint!("{}", error.render(path, &source));
        }
        std::process::exit(1);
    }
}

/// Same representation as `parse`, plus a small summary of how many
/// functions, statements, and expressions the program contains.
fn ast(path: &str) {
    let source = read_source_file(path);
    let (tokens, lex_errors) = lexer::tokenize(&source);

    if !lex_errors.is_empty() {
        for error in &lex_errors {
            eprint!("{}", error.render(path, &source));
        }
        std::process::exit(1);
    }

    let (program, parse_errors) = parser::parse(tokens);
    print!("{}", pretty::print_program(&program));

    let counts = pretty::count_nodes(&program);
    println!();
    println!(
        "{} function(s), {} statement(s), {} expression(s)",
        counts.functions, counts.statements, counts.expressions
    );

    if !parse_errors.is_empty() {
        eprintln!();
        for error in &parse_errors {
            eprint!("{}", error.render(path, &source));
        }
        std::process::exit(1);
    }
}

/// How wide the stage-name column of `dijk check`'s report is.
const STAGE_COLUMN: usize = 23;

/// Check a program through every stage before execution, reporting how
/// each one went, and never run it:
///
/// ```text
/// Checking main.dk...
/// Lexical analysis       OK
/// Parsing                OK
/// Semantic analysis      OK
/// Type checking          OK
///
/// Program is well-typed.
/// ```
///
/// The report stops at the first stage that fails, marked `FAILED` with
/// its number of errors, and that stage's diagnostics go to stderr, as
/// with every other command. Exit code 0 means the program is well-typed
/// (which says nothing about whether it will run without a runtime
/// error); 1 means some stage rejected it.
fn check(path: &str) {
    let source = read_source_file(path);
    println!("Checking {path}...");

    let (tokens, lex_errors) = lexer::tokenize(&source);
    if !lex_errors.is_empty() {
        let diagnostics = lex_errors.iter().map(|e| e.render(path, &source));
        fail_stage("Lexical analysis", diagnostics.collect());
    }
    pass_stage("Lexical analysis");

    let (program, parse_errors) = parser::parse(tokens);
    if !parse_errors.is_empty() {
        let diagnostics = parse_errors.iter().map(|e| e.render(path, &source));
        fail_stage("Parsing", diagnostics.collect());
    }
    pass_stage("Parsing");

    let semantic_errors = semantic::analyze(&program);
    if !semantic_errors.is_empty() {
        let diagnostics = semantic_errors.iter().map(|e| e.render(path, &source));
        fail_stage("Semantic analysis", diagnostics.collect());
    }
    pass_stage("Semantic analysis");

    if let Err(type_errors) = types::check(&program) {
        let diagnostics = type_errors.iter().map(|e| e.render(path, &source));
        fail_stage("Type checking", diagnostics.collect());
    }
    pass_stage("Type checking");

    println!();
    println!("Program is well-typed.");
}

fn pass_stage(stage: &str) {
    println!("{stage:<STAGE_COLUMN$}OK");
}

/// Report `stage` as failed with the given rendered diagnostics, print
/// them, and exit: no later stage runs.
fn fail_stage(stage: &str, diagnostics: Vec<String>) -> ! {
    let count = diagnostics.len();
    let noun = if count == 1 { "error" } else { "errors" };
    println!("{stage:<STAGE_COLUMN$}FAILED ({count} {noun})");
    for diagnostic in &diagnostics {
        eprint!("{diagnostic}");
    }
    std::process::exit(1);
}

/// Run a program through every stage in turn:
///
/// ```text
/// lex → parse → semantic analysis → type checking → interpret
/// ```
///
/// Each stage runs only if every stage before it succeeded. A program
/// with lexical or parse errors is never analyzed (its AST would be
/// partial), a program with semantic errors is never type-checked, and a
/// program with type errors is never executed. On success, the value
/// `main` returns is printed on its own line (nothing is printed if it
/// returns no value) and the exit code is 0. Any error, from any stage,
/// exits with 1.
fn run(path: &str) {
    let source = read_source_file(path);
    let (tokens, lex_errors) = lexer::tokenize(&source);

    if !lex_errors.is_empty() {
        for error in &lex_errors {
            eprint!("{}", error.render(path, &source));
        }
        std::process::exit(1);
    }

    let (program, parse_errors) = parser::parse(tokens);

    if !parse_errors.is_empty() {
        for error in &parse_errors {
            eprint!("{}", error.render(path, &source));
        }
        std::process::exit(1);
    }

    let semantic_errors = semantic::analyze(&program);

    if !semantic_errors.is_empty() {
        for error in &semantic_errors {
            eprint!("{}", error.render(path, &source));
        }
        std::process::exit(1);
    }

    // A program with type errors must not run at all: the type checker,
    // not the interpreter, decides whether its operations are valid.
    if let Err(type_errors) = types::check(&program) {
        for error in &type_errors {
            eprint!("{}", error.render(path, &source));
        }
        std::process::exit(1);
    }

    match interpreter::run(&program) {
        Ok(Value::Unit) => {}
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprint!("{}", error.render(path, &source));
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_dijk_source_file;

    #[test]
    fn accepts_dk_and_dijk() {
        assert!(is_dijk_source_file("hello.dk"));
        assert!(is_dijk_source_file("hello.dijk"));
        assert!(is_dijk_source_file("examples/hello.dk"));
        assert!(is_dijk_source_file("examples\\hello.dijk"));
    }

    #[test]
    fn rejects_anything_else() {
        assert!(!is_dijk_source_file("hello.txt"));
        assert!(!is_dijk_source_file("hello"));
        assert!(!is_dijk_source_file("hello.dk.bak"));
        assert!(!is_dijk_source_file("hello.DK"));
    }
}
