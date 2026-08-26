//! A minimal recursive traversal over the AST.
//!
//! This is deliberately not an elaborate visitor framework — just one
//! method per node kind, each with a default implementation that walks
//! into that node's children via the matching `walk_*` function. A
//! consumer overrides only the methods it cares about; everything else
//! keeps descending on its own.
//!
//! Semantic analysis, type checking, and later compiler passes will all
//! need to walk this same AST, so this is the shared mechanism they'll
//! build on — not a demonstration with no consumer of its own yet.

use super::declaration::{Declaration, Function, Program};
use super::expression::Expression;
use super::statement::Statement;

pub trait Visitor {
    fn visit_program(&mut self, program: &Program) {
        walk_program(self, program);
    }

    fn visit_declaration(&mut self, decl: &Declaration) {
        walk_declaration(self, decl);
    }

    fn visit_function(&mut self, function: &Function) {
        walk_function(self, function);
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        walk_statement(self, stmt);
    }

    fn visit_expression(&mut self, expr: &Expression) {
        walk_expression(self, expr);
    }
}

pub fn walk_program<V: Visitor + ?Sized>(visitor: &mut V, program: &Program) {
    for decl in &program.declarations {
        visitor.visit_declaration(decl);
    }
}

pub fn walk_declaration<V: Visitor + ?Sized>(visitor: &mut V, decl: &Declaration) {
    match decl {
        Declaration::Function(function) => visitor.visit_function(function),
    }
}

pub fn walk_function<V: Visitor + ?Sized>(visitor: &mut V, function: &Function) {
    for stmt in &function.body {
        visitor.visit_statement(stmt);
    }
}

pub fn walk_statement<V: Visitor + ?Sized>(visitor: &mut V, stmt: &Statement) {
    match stmt {
        Statement::Let { value, .. } => visitor.visit_expression(value),
        Statement::Return { value, .. } => {
            if let Some(value) = value {
                visitor.visit_expression(value);
            }
        }
        Statement::Expression { expr, .. } => visitor.visit_expression(expr),
        Statement::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            visitor.visit_expression(condition);
            for stmt in then_branch {
                visitor.visit_statement(stmt);
            }
            if let Some(else_branch) = else_branch {
                for stmt in else_branch {
                    visitor.visit_statement(stmt);
                }
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            visitor.visit_expression(condition);
            for stmt in body {
                visitor.visit_statement(stmt);
            }
        }
    }
}

pub fn walk_expression<V: Visitor + ?Sized>(visitor: &mut V, expr: &Expression) {
    match expr {
        Expression::Integer(..)
        | Expression::String(..)
        | Expression::Boolean(..)
        | Expression::Identifier(..) => {}
        Expression::Unary { operand, .. } => visitor.visit_expression(operand),
        Expression::Binary { left, right, .. } => {
            visitor.visit_expression(left);
            visitor.visit_expression(right);
        }
        Expression::Assign { value, .. } => visitor.visit_expression(value),
    }
}
