//! Unit tests for the `types` module (`crate::types`): the building blocks
//! of the type checker, one section per piece, added as each piece is.
//!
//! Not to be confused with `tests/type_tests/`, the language-level suite:
//! that one asks "does Dijk accept or reject this program, and why?", one
//! file per typing rule. This one asks whether each *piece* of the
//! implementation (the `Type` representation, annotation resolution, and
//! so on) behaves as designed.

// None of these modules are part of a library's public surface (dijk is a
// binary crate), so these tests include them directly by path, mirroring
// their layout as siblings under the crate root in src/main.rs. The AST
// is here only for its `Type` annotation node, and only the lexer's
// `token.rs` (for `Span`) is needed, hence the `allow`s.
#[allow(dead_code)]
#[path = "../src/ast/mod.rs"]
mod ast;
#[path = "../src/diagnostics.rs"]
mod diagnostics;
#[path = "../src/lexer"]
mod lexer {
    #[allow(dead_code)]
    #[path = "token.rs"]
    pub mod token;
}
#[path = "../src/types/mod.rs"]
mod types;

use lexer::token::Span;
use types::error::TypeErrorKind;
use types::ty::{self, Type};

/// A type annotation as the parser would build it, e.g. the `i32` in
/// `let x: i32 = 1;`.
fn annotation(name: &str) -> ast::types::Type {
    ast::types::Type {
        name: name.to_string(),
        span: Span::new(7, 7 + name.len()),
    }
}

// --- the type representation ---

#[test]
fn a_type_is_equal_to_itself() {
    assert_eq!(Type::I32, Type::I32);
    assert_eq!(Type::I64, Type::I64);
    assert_eq!(Type::Bool, Type::Bool);
    assert_eq!(Type::String, Type::String);
    assert_eq!(Type::Unit, Type::Unit);
}

#[test]
fn different_types_are_not_equal() {
    // Both are integers, but they are different types.
    assert_ne!(Type::I32, Type::I64);
    assert_ne!(Type::Bool, Type::String);
    assert_ne!(Type::I32, Type::Bool);
    assert_ne!(Type::String, Type::Unit);
}

#[test]
fn types_display_as_they_are_written_in_dijk() {
    assert_eq!(Type::I32.to_string(), "i32");
    assert_eq!(Type::I64.to_string(), "i64");
    assert_eq!(Type::Bool.to_string(), "bool");
    assert_eq!(Type::String.to_string(), "string");
    assert_eq!(Type::Unit.to_string(), "unit");
}

#[test]
fn only_i32_and_i64_are_integers() {
    assert!(Type::I32.is_integer());
    assert!(Type::I64.is_integer());
    assert!(!Type::Bool.is_integer());
    assert!(!Type::String.is_integer());
    assert!(!Type::Unit.is_integer());
}

// --- compatibility ---

#[test]
fn compatibility_is_equality() {
    assert!(Type::compatible(Type::I32, Type::I32));
    assert!(Type::compatible(Type::String, Type::String));
    // No implicit conversions, not even the "safe" widening one.
    assert!(!Type::compatible(Type::I64, Type::I32));
    assert!(!Type::compatible(Type::I32, Type::I64));
    assert!(!Type::compatible(Type::Bool, Type::I32));
}

#[test]
fn the_internal_error_type_is_compatible_with_everything() {
    // So that an expression already reported as wrong is not reported
    // again by whatever uses it.
    for other in [Type::I32, Type::I64, Type::Bool, Type::String, Type::Unit] {
        assert!(Type::compatible(other, Type::Error));
        assert!(Type::compatible(Type::Error, other));
    }
}

// --- resolving annotations ---

#[test]
fn the_four_writable_type_names_resolve() {
    assert_eq!(ty::resolve(&annotation("i32")), Ok(Type::I32));
    assert_eq!(ty::resolve(&annotation("i64")), Ok(Type::I64));
    assert_eq!(ty::resolve(&annotation("bool")), Ok(Type::Bool));
    assert_eq!(ty::resolve(&annotation("string")), Ok(Type::String));
}

#[test]
fn an_unknown_type_name_is_a_type_error_at_the_annotation() {
    let error = ty::resolve(&annotation("banana")).unwrap_err();
    assert_eq!(
        error.kind,
        TypeErrorKind::UnknownType {
            name: "banana".to_string()
        }
    );
    assert_eq!(error.span, Span::new(7, 13));
}

#[test]
fn unit_cannot_be_written_in_an_annotation() {
    // `unit` is a type the checker uses, but not a name a program can use.
    assert_eq!(
        ty::resolve(&annotation("unit")).unwrap_err().kind,
        TypeErrorKind::UnknownType {
            name: "unit".to_string()
        }
    );
}

#[test]
fn type_names_are_case_sensitive() {
    assert!(ty::resolve(&annotation("I32")).is_err());
    assert!(ty::resolve(&annotation("String")).is_err());
}

#[test]
fn type_errors_render_with_their_own_label() {
    let source = "let x: banana = 1;";
    let error = ty::resolve(&annotation("banana")).unwrap_err();
    let rendered = error.render("example.dk", source);
    assert!(rendered.starts_with("type error: unknown type `banana`\n"));
    assert!(rendered.contains(" --> example.dk:1:8"));
    assert_eq!(error.to_string(), "unknown type `banana`");
}
