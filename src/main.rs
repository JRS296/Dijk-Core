//! Dijk Core CLI.
//!
//! There is no parser or compiler yet — only lexical analysis. This binary
//! is the entry point for observing each compiler stage as it is built.

mod lexer;

const VERSION: &str = "0.0.0";

const HELP: &str = "\
Dijk 0.0.0

Dijk Core is building its compiler one stage at a time. See docs/ for the
philosophy, design constraints, architecture, and roadmap.

USAGE:
    dijk [COMMAND]

COMMANDS:
    --version    Print the Dijk version
    --help       Print this message
    doctor       Report the current state of the toolchain
    lex <file>   Print the token stream for a Dijk source file
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
    println!("Status: Bootstrap development (lexer implemented)");
    println!("Compiler: Not yet implemented");
}

fn lex(path: &str) {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("dijk: could not read '{path}': {err}");
            std::process::exit(1);
        }
    };

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
