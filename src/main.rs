//! Dijk Core CLI.
//!
//! There is no compiler yet — only lexical analysis and parsing. This
//! binary is the entry point for observing each compiler stage as it is
//! built.

mod ast;
mod diagnostics;
mod lexer;
mod parser;
mod pretty;

const VERSION: &str = "0.0.2";

const HELP: &str = "\
Dijk 0.0.2

Dijk Core is building its compiler one stage at a time. See docs/ for the
philosophy, design constraints, architecture, and roadmap.

USAGE:
    dijk [COMMAND]

COMMANDS:
    --version    Print the Dijk version
    --help       Print this message
    doctor       Report the current state of the toolchain
    lex <file>   Print the token stream for a Dijk source file (.dk or .dijk)
    parse <file> Print the syntax tree for a Dijk source file (.dk or .dijk)
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
    println!("Status: Bootstrap development (lexer and parser implemented)");
    println!("Compiler: Not yet implemented");
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
