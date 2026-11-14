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
//! Resolution happens in two passes:
//!
//! ```text
//! Pass 1: collect every function's name and parameter count
//! Pass 2: walk every function body, checking each name against
//!         what pass 1 collected and what the body has declared
//! ```
//!
//! Collecting functions first is what makes a call to a function defined
//! *later* in the file resolve, and what lets a function call itself:
//! by the time any body is checked, every function is already known.
//!
//! Dijk has no first-class functions, so a name in call position
//! (`foo(...)`) can only mean a function, and a name anywhere else can
//! only mean a variable. The two are looked up in separate tables and
//! never interfere.

use super::error::{SemanticError, SemanticErrorKind};
use crate::ast::declaration::{Declaration, Function, Program};
use crate::ast::expression::Expression;
use crate::ast::statement::Statement;
use crate::ast::visit::{self, Visitor};
use crate::lexer::token::Span;
use std::collections::{HashMap, HashSet};

/// Resolve every name in `program`, returning the errors found in source
/// order.
pub fn resolve(program: &Program) -> Vec<SemanticError> {
    let mut resolver = Resolver {
        functions: HashMap::new(),
        variables: HashSet::new(),
        errors: Vec::new(),
    };

    resolver.collect_functions(program);
    resolver.check_main(program);
    resolver.visit_program(program);

    let mut errors = resolver.errors;
    errors.sort_by_key(|e| e.span.start);
    errors
}

struct Resolver {
    /// Every function in the program: name → number of parameters.
    /// Filled by pass 1, before any body is resolved.
    functions: HashMap<String, usize>,
    /// The variables declared so far in the function currently being
    /// resolved: its parameters, then each `let` in the order it appears.
    variables: HashSet<String>,
    errors: Vec<SemanticError>,
}

impl Resolver {
    fn error(&mut self, kind: SemanticErrorKind, span: Span) {
        self.errors.push(SemanticError::new(kind, span));
    }

    /// Pass 1. A second function with an already-used name is reported
    /// at its own `fn`; the first definition is the one calls resolve to.
    fn collect_functions(&mut self, program: &Program) {
        for declaration in &program.declarations {
            let Declaration::Function(function) = declaration;
            if self.functions.contains_key(&function.name) {
                self.error(
                    SemanticErrorKind::DuplicateFunction {
                        name: function.name.clone(),
                    },
                    function.span,
                );
            } else {
                self.functions
                    .insert(function.name.clone(), function.params.len());
            }
        }
    }

    /// Every program needs a `main` to start from, and `main` takes no
    /// parameters because nothing in a Dijk program calls it.
    ///
    /// A missing `main` has no source location of its own. The error is
    /// anchored at the end of the last declaration, where a `main` could
    /// be added, or at the very start of an empty file.
    fn check_main(&mut self, program: &Program) {
        let main = program.declarations.iter().find_map(|declaration| {
            let Declaration::Function(function) = declaration;
            (function.name == "main").then_some(function)
        });

        match main {
            None => {
                let end = program
                    .declarations
                    .last()
                    .map(|declaration| {
                        let Declaration::Function(function) = declaration;
                        function.span.end
                    })
                    .unwrap_or(0);
                self.error(SemanticErrorKind::MissingMain, Span::new(end, end));
            }
            Some(main) => {
                if let Some(first) = main.params.first() {
                    self.error(SemanticErrorKind::MainHasParameters, first.span);
                }
            }
        }
    }

    /// A variable use (not a call, not an assignment target).
    fn resolve_variable(&mut self, name: &str, span: Span) {
        if !self.variables.contains(name) {
            self.error(
                SemanticErrorKind::UnknownIdentifier {
                    name: name.to_string(),
                },
                span,
            );
        }
    }

    /// An assignment target must already exist: `x = 1;` changes `x`, it
    /// does not declare it.
    fn resolve_assignment_target(&mut self, name: &str, span: Span) {
        if !self.variables.contains(name) {
            self.error(
                SemanticErrorKind::AssignToUndeclared {
                    name: name.to_string(),
                },
                span,
            );
        }
    }

    /// A call must name a known function and pass it exactly as many
    /// arguments as it has parameters.
    fn resolve_call(&mut self, callee: &str, argument_count: usize, span: Span) {
        match self.functions.get(callee) {
            None => self.error(
                SemanticErrorKind::UnknownFunction {
                    name: callee.to_string(),
                },
                span,
            ),
            Some(&expected) if expected != argument_count => self.error(
                SemanticErrorKind::ArityMismatch {
                    name: callee.to_string(),
                    expected,
                    found: argument_count,
                },
                span,
            ),
            Some(_) => {}
        }
    }
}

impl Visitor for Resolver {
    /// Each function starts with no variables but its parameters: a body
    /// cannot see the variables of any other function.
    fn visit_function(&mut self, function: &Function) {
        self.variables.clear();

        for param in &function.params {
            if !self.variables.insert(param.name.clone()) {
                self.error(
                    SemanticErrorKind::DuplicateParameter {
                        name: param.name.clone(),
                    },
                    param.span,
                );
            }
        }

        visit::walk_function(self, function);
    }

    /// `let` is the one statement that introduces a name. Its initializer
    /// is resolved *before* the name is declared, so `let x = x;` refers
    /// to some earlier `x`, never to the variable being declared.
    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let { name, value, .. } => {
                self.visit_expression(value);
                self.variables.insert(name.clone());
            }
            _ => visit::walk_statement(self, stmt),
        }
    }

    fn visit_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Identifier(name, span) => self.resolve_variable(name, *span),
            Expression::Assign { name, span, .. } => self.resolve_assignment_target(name, *span),
            Expression::Call {
                callee,
                arguments,
                span,
            } => self.resolve_call(callee, arguments.len(), *span),
            Expression::Integer(..)
            | Expression::String(..)
            | Expression::Boolean(..)
            | Expression::Binary { .. }
            | Expression::Unary { .. } => {}
        }

        // Whatever this node was, its children (operands, the assigned
        // value, call arguments) contain names of their own.
        visit::walk_expression(self, expr);
    }
}
