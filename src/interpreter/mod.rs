//! The interpreter: gives a checked program behavior by walking its AST.
//!
//! Every stage before this one only *described* the program: the lexer
//! found its pieces, the parser its structure, and semantic analysis
//! checked that its names mean something. The interpreter is the first
//! stage that *does* what the program says.
//!
//! It is a **tree-walking** interpreter. It evaluates the AST directly,
//! node by node, with no further translation in between:
//!
//! ```text
//! AST (source)                 Runtime
//!
//! Binary(+)                    Integer(30)
//! ├── Integer(10)      ──→
//! └── Integer(20)
//! ```
//!
//! The left side is a description of a computation. The right side is
//! what that computation produced. AST nodes are source; evaluating them
//! produces runtime values (`value.rs`).
//!
//! See `docs/interpreter.md` for the execution model as a whole.

pub mod environment;
pub mod error;
mod eval;
pub mod value;

use crate::ast::declaration::Program;
use error::RuntimeError;
use value::Value;

/// How much stack the interpreter runs on: 256 MiB.
///
/// A tree-walking interpreter implements Dijk's call stack on top of
/// Rust's own: every Dijk call nests a dozen or so Rust calls, and an
/// unoptimized (debug) build spends tens of kilobytes of Rust stack on
/// each one. The default stack of a thread (a few MiB, and only 2 MiB
/// for each `cargo test` thread) runs out long before
/// [`eval::MAX_CALL_DEPTH`] Dijk calls, which would crash the
/// interpreter instead of reporting the runtime error the limit exists
/// to produce. So the interpreter gets a thread with a stack of a size
/// chosen here, comfortably larger than `MAX_CALL_DEPTH` calls need.
/// This is address space reserved, not memory allocated: the operating
/// system only backs the pages a program actually touches.
const STACK_SIZE: usize = 256 * 1024 * 1024;

/// Run `program` by calling its `main`, returning the value `main`
/// returns (`Unit` if it returns none).
///
/// The program must have passed semantic analysis. The interpreter
/// relies on it: a name it cannot find is reported as a runtime error
/// rather than crashing, but finding those is semantic analysis's job,
/// with better diagnostics.
///
/// The interpreter runs on a thread of its own with a [`STACK_SIZE`]
/// stack; this function waits for it to finish.
pub fn run(program: &Program) -> Result<Value, RuntimeError> {
    std::thread::scope(|scope| {
        let interpreter = std::thread::Builder::new()
            .name("dijk-interpreter".to_string())
            .stack_size(STACK_SIZE)
            .spawn_scoped(scope, || eval::Interpreter::new(program).run_main());

        match interpreter {
            Ok(thread) => match thread.join() {
                Ok(result) => result,
                // The interpreter itself panicked: a bug in Dijk, not in
                // the program. Let it surface as the panic it is.
                Err(panic) => std::panic::resume_unwind(panic),
            },
            // The operating system refused a stack that large. Run on
            // the current thread instead: programs that recurse only
            // modestly still work.
            Err(_) => eval::Interpreter::new(program).run_main(),
        }
    })
}
