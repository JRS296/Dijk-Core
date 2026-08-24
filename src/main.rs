//! Dijk Core CLI — currently establishes the project only.
//!
//! There is no lexer, parser, or compiler yet. This binary exists to give
//! the repository a compilable entry point and a version to refer to.

const VERSION: &str = "0.0.0";

const HELP: &str = "\
Dijk 0.0.0

Dijk Core is at Milestone 0: the language charter. See docs/ for the
philosophy, design constraints, architecture, and roadmap. No compiler
subcommands are implemented yet.

USAGE:
    dijk [COMMAND]

COMMANDS:
    --version    Print the Dijk version
    --help       Print this message
    doctor       Report the current state of the toolchain
";

fn main() {
    let mut args = std::env::args().skip(1);

    match args.next().as_deref() {
        None | Some("--help") | Some("-h") => print!("{HELP}"),
        Some("--version") | Some("-V") => println!("Dijk {VERSION}"),
        Some("doctor") => doctor(),
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
    println!("Status: Bootstrap development (Milestone 0 — Language Charter)");
    println!("Compiler: Not yet implemented");
}
