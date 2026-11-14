//! The static type checking test suite: Dijk programs, valid and
//! invalid, organised by the typing rule they exercise.
//!
//! Every program here is source text taken through the real front end
//! (lexer → parser → semantic analysis → type checker), exactly as
//! `dijk check` takes it. A fixture that fails *before* the type checker
//! is a broken fixture, and the helpers below say so. What each test then
//! asserts is the language-level answer: is this program well-typed, and
//! if not, which rules does it break (by their stable diagnostic codes)?
//!
//! ```text
//! literals.rs      integer, bool and string literals; literal ranges (E0010)
//! operators.rs     binary and unary operators (E0002, E0003)
//! variables.rs     inference, annotations, unknown types (E0001, E0008)
//! assignments.rs   assignment compatibility (E0007), and what is not a
//!                  type rule (mutability)
//! functions.rs     signatures and calls (E0004, E0008)
//! returns.rs       return types and return paths (E0005, E0009)
//! conditions.rs    boolean conditions (E0006)
//! errors.rs        diagnostics: codes, rendering, reporting every error
//!                  once, in order
//! pipeline.rs      a program with type errors never reaches the
//!                  interpreter
//! ```
//!
//! `tests/types_tests.rs` is the other half: unit tests of the `types`
//! module's individual pieces.

// None of these modules are part of a library's public surface (dijk is a
// binary crate), so this suite includes them directly by path, mirroring
// their layout as siblings under the crate root in src/main.rs. Not every
// stage's every function is used by the suite, hence the `allow`s.
#[allow(dead_code)]
#[path = "../../src/ast/mod.rs"]
mod ast;
#[allow(dead_code)]
#[path = "../../src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../../src/interpreter/mod.rs"]
mod interpreter;
#[allow(dead_code)]
#[path = "../../src/lexer/mod.rs"]
mod lexer;
#[allow(dead_code)]
#[path = "../../src/parser/mod.rs"]
mod parser;
#[allow(dead_code)]
#[path = "../../src/semantic/mod.rs"]
mod semantic;
#[allow(dead_code)]
#[path = "../../src/types/mod.rs"]
mod types;

mod assignments;
mod conditions;
mod errors;
mod functions;
mod literals;
mod operators;
mod pipeline;
mod returns;
mod variables;

use ast::declaration::Program;
use lexer::token::Span;
use types::error::TypeError;
use types::ty::Type;
use types::TypedProgram;

/// Lex and parse `source`, asserting both stages accept it.
fn parse_syntax(source: &str) -> Program {
    let (tokens, lex_errors) = lexer::tokenize(source);
    assert!(
        lex_errors.is_empty(),
        "broken fixture, lex errors: {lex_errors:?}"
    );
    let (program, parse_errors) = parser::parse(tokens);
    assert!(
        parse_errors.is_empty(),
        "broken fixture, parse errors: {parse_errors:?}"
    );
    program
}

/// Lex, parse and analyze `source`, asserting that every stage before the
/// type checker accepts it.
fn front_end(source: &str) -> Program {
    let program = parse_syntax(source);
    let semantic_errors = semantic::analyze(&program);
    assert!(
        semantic_errors.is_empty(),
        "broken fixture, semantic errors: {semantic_errors:?}"
    );
    program
}

/// Assert that `source` is well-typed, and return what the checker
/// learned about it.
fn well_typed(source: &str) -> TypedProgram {
    match types::check(&front_end(source)) {
        Ok(typed) => typed,
        Err(errors) => panic!(
            "expected a well-typed program, got: {:?}",
            errors.iter().map(describe).collect::<Vec<_>>()
        ),
    }
}

/// Assert that `source` has type errors, and return them in source order.
fn type_errors(source: &str) -> Vec<TypeError> {
    match types::check(&front_end(source)) {
        Ok(_) => panic!("expected type errors, but the program is well-typed:\n{source}"),
        Err(errors) => errors,
    }
}

/// The diagnostic codes of `source`'s type errors, in source order.
fn codes(source: &str) -> Vec<&'static str> {
    type_errors(source).iter().map(|e| e.code()).collect()
}

/// `E0001 type mismatch`, for readable assertion failures.
fn describe(error: &TypeError) -> String {
    format!("{} {}", error.code(), error)
}

/// `body` as the body of a `main` that returns nothing.
fn in_main(body: &str) -> String {
    format!("fn main() {{\n{body}\n}}\n")
}

/// `body` as the body of `fn f() -> {return_type}`, next to an empty
/// `main`.
fn in_function(return_type: &str, body: &str) -> String {
    format!("fn f() -> {return_type} {{\n{body}\n}}\n\nfn main() {{}}\n")
}

/// The span of the first occurrence of `snippet` in `source`.
fn span_of(source: &str, snippet: &str) -> Span {
    let start = source
        .find(snippet)
        .unwrap_or_else(|| panic!("`{snippet}` does not occur in the fixture"));
    Span::new(start, start + snippet.len())
}

/// The type the checker gave the expression written as the first
/// occurrence of `snippet` in the well-typed program `source`.
fn type_of(source: &str, snippet: &str) -> Type {
    match well_typed(source)
        .expression_types
        .get(&span_of(source, snippet))
    {
        Some(ty) => *ty,
        None => panic!("`{snippet}` was given no type"),
    }
}

/// The type of each variable the function `function` declares, in order.
fn binding_types(source: &str, function: &str) -> Vec<(String, Type)> {
    let typed = well_typed(source);
    let function = typed
        .functions
        .iter()
        .find(|f| f.name == function)
        .unwrap_or_else(|| panic!("no function `{function}`"));
    function
        .bindings
        .iter()
        .map(|b| (b.name.clone(), b.ty))
        .collect()
}

fn bound(name: &str, ty: Type) -> (String, Type) {
    (name.to_string(), ty)
}
