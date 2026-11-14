//! The resolver: a walk over the whole AST that checks what names refer
//! to.
//!
//! It is built on the AST's `Visitor` (`crate::ast::visit`). Semantic
//! analysis only needs to *look at* nodes, never to compute a value from
//! them, which is exactly what the visitor was designed for. A rule is
//! added by overriding the `visit_*` method for the node kind it cares
//! about. Everything else keeps descending through the default `walk_*`
//! functions.
//!
//! For now the resolver overrides nothing. It visits every function,
//! statement and expression in the program and enforces no rules yet.

use super::error::SemanticError;
use crate::ast::declaration::Program;
use crate::ast::visit::Visitor;

/// Resolve every name in `program`, returning the errors found in source
/// order.
pub fn resolve(program: &Program) -> Vec<SemanticError> {
    let mut resolver = Resolver { errors: Vec::new() };
    resolver.visit_program(program);

    let mut errors = resolver.errors;
    errors.sort_by_key(|e| e.span.start);
    errors
}

struct Resolver {
    errors: Vec<SemanticError>,
}

impl Visitor for Resolver {}
