//! The type checker: a walk over the whole AST that works out the type
//! of every expression, without evaluating any of them.
//!
//! Like the interpreter, and unlike semantic analysis, this walk does not
//! use the AST's `Visitor`. A visitor only *looks* at nodes and returns
//! nothing, but checking an expression must *produce* its type. So each
//! kind of node is handled by an explicit `match`, and the central
//! operation is:
//!
//! ```text
//! check_expression(expression, expected) → Type
//! ```
//!
//! It is the static twin of the interpreter's `evaluate(expression) →
//! Value`. Both walk the same tree in the same shape; one computes what
//! kind of value an expression *would* produce, the other the value
//! itself.
//!
//! # The expected type
//!
//! The second argument is what the surrounding code expects the
//! expression's type to be, if it expects anything. Most expressions
//! ignore it: `true` is a `bool` whatever anyone expects. Integer
//! literals are the exception, because `42` could be an `i32` or an
//! `i64`. A literal takes its type from what is expected of it and, with
//! no expectation, defaults to `i32`:
//!
//! ```text
//! 42              expected nothing   → i32   (the default)
//! 42              expected i64       → i64
//! 42              expected bool      → i32   (a fallback: the mismatch
//!                                             is reported by whoever
//!                                             expected bool)
//! ```
//!
//! This is all the inference an integer literal gets: information flows
//! from the context *down* into the literal, once, and is never solved
//! for. There is no constraint solving and no Hindley–Milner.
//!
//! # Literal ranges
//!
//! Once a literal has a type, its value must fit in it. `3000000000` does
//! not fit in an `i32`, so it is a type error, found without running
//! anything. A minus sign written directly before a literal counts as
//! part of the literal, so `-2147483648`, the smallest `i32`, is accepted
//! even though `2147483648` on its own is not.
//!
//! # Operators
//!
//! An operator expression is typed from the inside out: first its
//! operands, then the operator's rule for those operand types
//! (`rules.rs`). The checker never computes anything:
//!
//! ```text
//! 10 + 20 * 3
//!
//! Binary(+)              check 10           → i32
//! ├── Integer(10)        check 20 * 3
//! └── Binary(*)              check 20       → i32
//!     ├── Integer(20)        check 3        → i32
//!     └── Integer(3)         i32 * i32      → i32
//!                        i32 + i32          → i32
//! ```
//!
//! The interpreter walks the same tree and gets `70`; the checker gets
//! `i32`, and would get it just the same for `10 + 20 / 0`.
//!
//! A literal operand takes its type from the *other* operand, so that
//! `x + 1` works whatever integer type `x` has. The left operand is
//! checked first and its type is what the right one is expected to have,
//! unless the left operand is a bare literal and the right one is not, in
//! which case the order is swapped (`1 + x`).
//!
//! # Variables
//!
//! A `let` is where a variable gets its type, in one of two ways:
//!
//! ```text
//! let x = 10;          inference: x gets the initializer's type, i32
//! let y: i64 = 10;     checking:  the initializer is checked against the
//!                                 annotation (and the literal becomes
//!                                 an i64); y gets the annotated type
//! ```
//!
//! Either way the variable is declared in the type environment Γ
//! (`env.rs`), and every later use of it has that type. Scopes follow
//! exactly the rules semantic analysis checked the program against: each
//! `if`/`else` branch and `while` body is a block scope of its own, an
//! inner `let` may shadow an outer variable, and an initializer is
//! checked *before* its own variable is declared. The checker does not
//! resolve names again: semantic analysis already has, so a name is
//! simply looked up in Γ.
//!
//! # Functions
//!
//! Before any body is checked, the checker collects every function's
//! *signature*: its parameter types, and its return type (`unit` if the
//! function has no `-> T`). Like semantic analysis's first pass, this is
//! what lets a body use a function declared later in the file, or
//! itself. Each body is then checked with its parameters already in the
//! function scope of Γ, with the types their annotations name.
//!
//! A call is checked against the callee's signature alone: each argument
//! must have its parameter's type (which is also what an integer literal
//! argument is expected to be), and the call expression has the callee's
//! return type. Whether the function exists, and whether the call passes
//! the right *number* of arguments, are questions about names that
//! semantic analysis has already answered.
//!
//! # Returns and conditions
//!
//! Inside a body, the signature's return type is what every `return`
//! must produce: `return value;` checks the value against it (so a
//! literal there takes the return type), a bare `return;` produces
//! `unit`, and so is only allowed in a function without `-> T`. The
//! condition of every `if` and `while` must be a `bool`: Dijk has no
//! "truthiness", so `if 1` is a type error, not "true because non-zero".
//!
//! # Return paths
//!
//! Checking each `return` is not the same as knowing that the function
//! *does* return. This passes the first test and fails the second:
//!
//! ```dijk
//! fn get_number() -> i32 {
//!     if true {
//!         return 10;
//!     }
//! }
//! ```
//!
//! The checker does not evaluate `true`, so as far as it can tell the
//! `if` might be skipped, and then the function would reach its end with
//! no value to return. A function that returns something other than
//! `unit` must *definitely return*, which is decided from the shape of
//! its body alone (see [`always_returns`]). This is the checker's first,
//! very small, piece of control-flow analysis.
//!
//! # Assignments
//!
//! `x = value` stores a new value in an existing variable, so the value
//! must have the type the variable already has. A variable's type is
//! fixed when it is declared and never changes, whatever is assigned to
//! it later. The variable's type is also what an integer literal on the
//! right is expected to be. Whether `x` may be assigned *at all* is a
//! different question, about the binding rather than the value, and
//! semantic analysis has already answered it (only a `let mut` may be).

use super::env::TypeEnvironment;
use super::error::{TypeError, TypeErrorKind};
use super::rules;
use super::ty::{self, Type};
use super::{Binding, FunctionSignature, TypedFunction, TypedProgram};
use crate::ast::declaration::{Declaration, Function, Program};
use crate::ast::expression::{BinaryOperator, Expression, UnaryOperator};
use crate::ast::statement::Statement;
use crate::ast::types::Type as TypeAnnotation;
use crate::lexer::token::Span;
use std::collections::HashMap;

/// Check every function in `program`, returning what was learned about
/// it, or every type error found, in source order.
pub fn check_program(program: &Program) -> Result<TypedProgram, Vec<TypeError>> {
    let mut checker = Checker {
        typed: TypedProgram::default(),
        signatures: HashMap::new(),
        env: TypeEnvironment::new(),
        bindings: Vec::new(),
        function: String::new(),
        return_type: Type::Unit,
        errors: Vec::new(),
    };

    // Pass 1: every signature, before any body.
    let signatures: Vec<FunctionSignature> = program
        .declarations
        .iter()
        .map(|declaration| {
            let Declaration::Function(function) = declaration;
            checker.collect_signature(function)
        })
        .collect();

    // Pass 2: every body, against its own signature.
    for (declaration, signature) in program.declarations.iter().zip(signatures) {
        let Declaration::Function(function) = declaration;
        checker.check_function(function, signature);
    }

    if checker.errors.is_empty() {
        Ok(checker.typed)
    } else {
        let mut errors = checker.errors;
        errors.sort_by_key(|e| e.span.start);
        Err(errors)
    }
}

struct Checker {
    /// What has been learned so far: grows as the walk proceeds.
    typed: TypedProgram,
    /// Every function's signature, by name, collected before any body is
    /// checked. If two functions share a name (a semantic error), the
    /// first one is the one calls are checked against, as in semantic
    /// analysis.
    signatures: HashMap<String, FunctionSignature>,
    /// Γ: the type of every variable visible at the current point of the
    /// function being checked.
    env: TypeEnvironment,
    /// Every variable the function being checked has declared so far.
    bindings: Vec<Binding>,
    /// The name and return type of the function being checked: what its
    /// `return` statements are checked against.
    function: String,
    return_type: Type,
    errors: Vec<TypeError>,
}

impl Checker {
    fn error(&mut self, kind: TypeErrorKind, span: Span) {
        self.errors.push(TypeError::new(kind, span));
    }

    // --- functions and statements ---

    /// Work out `function`'s signature from its annotations, and record it
    /// under the function's name for calls to be checked against. An
    /// unknown type name is reported here, once, and stands for
    /// `Type::Error` from then on.
    fn collect_signature(&mut self, function: &Function) -> FunctionSignature {
        let params = function
            .params
            .iter()
            .map(|param| (param.name.clone(), self.resolve(&param.ty)))
            .collect();

        // No `-> T` means the function returns no value: `unit`.
        let return_type = match &function.return_type {
            Some(annotation) => self.resolve(annotation),
            None => Type::Unit,
        };

        let signature = FunctionSignature {
            params,
            return_type,
        };
        self.signatures
            .entry(function.name.clone())
            .or_insert_with(|| signature.clone());
        signature
    }

    /// The type an annotation names, reporting an unknown name.
    fn resolve(&mut self, annotation: &TypeAnnotation) -> Type {
        match ty::resolve(annotation) {
            Ok(ty) => ty,
            Err(error) => {
                self.errors.push(error);
                Type::Error
            }
        }
    }

    /// Check `function`'s body against its `signature`. Each function is
    /// checked in a fresh environment holding only its parameters: a body
    /// cannot see the variables of any other function.
    fn check_function(&mut self, function: &Function, signature: FunctionSignature) {
        self.env = TypeEnvironment::new();
        self.bindings = Vec::new();
        self.function = function.name.clone();
        self.return_type = signature.return_type;

        for (param, (_, ty)) in function.params.iter().zip(&signature.params) {
            // A parameter's span starts at its name (`a: i32`).
            let name_span = Span::new(param.span.start, param.span.start + param.name.len());
            self.declare(&param.name, name_span, *ty, false);
        }

        // The body's top-level statements live in the function scope,
        // alongside the parameters.
        self.check_statements(&function.body);

        // A function that returns a value must return one on every path.
        // (With an unknown return type, already reported, there is
        // nothing to check against.)
        let returns_a_value = !matches!(self.return_type, Type::Unit | Type::Error);
        if returns_a_value && !always_returns(&function.body) {
            // Reported at the closing `}`: where execution would run out.
            let end = function.span.end;
            self.error(
                TypeErrorKind::MissingReturn {
                    function: function.name.clone(),
                    expected: self.return_type,
                },
                Span::new(end.saturating_sub(1), end),
            );
        }

        self.typed.functions.push(TypedFunction {
            name: function.name.clone(),
            signature,
            bindings: std::mem::take(&mut self.bindings),
        });
    }

    fn check_statements(&mut self, statements: &[Statement]) {
        for stmt in statements {
            self.check_statement(stmt);
        }
    }

    /// Check the statements of an `if`/`else` branch or a `while` body in
    /// a block scope of their own.
    fn check_block(&mut self, statements: &[Statement]) {
        self.env.push_scope();
        self.check_statements(statements);
        self.env.pop_scope();
    }

    /// Declare a variable in the innermost scope, and record it.
    fn declare(&mut self, name: &str, span: Span, ty: Type, mutable: bool) {
        self.env.declare(name, ty);
        self.bindings.push(Binding {
            name: name.to_string(),
            span,
            ty,
            mutable,
            depth: self.env.depth(),
        });
    }

    /// Statements have no type of their own. Checking one means checking
    /// the expressions it contains, and the statements nested in it.
    fn check_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let {
                mutable,
                name,
                name_span,
                type_annotation,
                value,
                ..
            } => self.check_let(name, *name_span, *mutable, type_annotation.as_ref(), value),
            Statement::Return { value, span } => self.check_return(value.as_ref(), *span),
            Statement::Expression { expr, .. } => {
                self.check_expression(expr, None);
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.check_condition(condition, "if");
                self.check_block(then_branch);
                if let Some(else_branch) = else_branch {
                    self.check_block(else_branch);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.check_condition(condition, "while");
                self.check_block(body);
            }
        }
    }

    /// `return value;` or `return;`, against the return type of the
    /// function it is in.
    fn check_return(&mut self, value: Option<&Expression>, span: Span) {
        let expected = self.return_type;
        let (found, at) = match value {
            Some(value) => (self.check_expression(value, Some(expected)), value.span()),
            // A bare `return;` returns no value.
            None => (Type::Unit, span),
        };

        if !Type::compatible(expected, found) {
            self.error(
                TypeErrorKind::InvalidReturn {
                    function: self.function.clone(),
                    expected,
                    found,
                },
                at,
            );
        }
    }

    /// The condition of an `if` or a `while` (named by `statement`) must
    /// be a `bool`.
    fn check_condition(&mut self, condition: &Expression, statement: &'static str) {
        let found = self.check_expression(condition, Some(Type::Bool));
        if !Type::compatible(Type::Bool, found) {
            self.error(
                TypeErrorKind::InvalidCondition { statement, found },
                condition.span(),
            );
        }
    }

    /// `let name: annotation = value;`, or `let name = value;`.
    ///
    /// With an annotation, the annotation decides the variable's type and
    /// the initializer is checked against it. Without one, the variable
    /// takes the initializer's type. The initializer is checked before
    /// the variable is declared, so `let x = x + 1;` reads an outer `x`.
    fn check_let(
        &mut self,
        name: &str,
        name_span: Span,
        mutable: bool,
        annotation: Option<&TypeAnnotation>,
        value: &Expression,
    ) {
        let ty = match annotation {
            None => self.check_expression(value, None),
            Some(annotation) => match self.resolve(annotation) {
                // An unknown type name (already reported): still check the
                // initializer on its own, and leave the variable untyped
                // so that its uses are not reported as well.
                Type::Error => {
                    self.check_expression(value, None);
                    Type::Error
                }
                declared => {
                    let found = self.check_expression(value, Some(declared));
                    if !Type::compatible(declared, found) {
                        self.error(
                            TypeErrorKind::Mismatch {
                                expected: declared,
                                found,
                            },
                            value.span(),
                        );
                    }
                    declared
                }
            },
        };
        self.declare(name, name_span, ty, mutable);
    }

    // --- expressions ---

    /// Work out the type of `expr`, given the type the surrounding code
    /// `expected` of it (if any), and record it in the side table.
    ///
    /// The result is `Type::Error` for an expression that could not be
    /// typed. Every such expression either has had its error reported
    /// already, or contains one that has, so callers never report it
    /// again.
    fn check_expression(&mut self, expr: &Expression, expected: Option<Type>) -> Type {
        let ty = match expr {
            Expression::Integer(value, _) => self.check_integer_literal(expr, *value, expected),
            Expression::String(..) => Type::String,
            Expression::Boolean(..) => Type::Bool,
            Expression::Unary {
                operator, operand, ..
            } => self.check_unary(expr, *operator, operand, expected),
            Expression::Binary {
                left,
                operator,
                right,
                ..
            } => self.check_binary(expr, left, *operator, right, expected),
            // Semantic analysis has already resolved every name, so the
            // variable is in Γ. (If the checker is ever given a program
            // that skipped analysis, an unknown name is simply untyped.)
            Expression::Identifier(name, _) => self.env.lookup(name).unwrap_or(Type::Error),
            Expression::Assign { name, value, .. } => self.check_assignment(name, value),
            Expression::Call {
                callee, arguments, ..
            } => self.check_call(callee, arguments),
        };
        self.record(expr.span(), ty);
        ty
    }

    /// `name = value`: the value must have the variable's type, and so
    /// does the assignment expression itself, since it evaluates to the
    /// value it stored (which is what makes `a = b = 0` work).
    fn check_assignment(&mut self, name: &str, value: &Expression) -> Type {
        let Some(declared) = self.env.lookup(name) else {
            // Unresolved: semantic analysis reports that, not the checker.
            self.check_expression(value, None);
            return Type::Error;
        };

        let found = self.check_expression(value, Some(declared));
        if !Type::compatible(declared, found) {
            self.error(
                TypeErrorKind::InvalidAssignment {
                    name: name.to_string(),
                    expected: declared,
                    found,
                },
                value.span(),
            );
        }
        declared
    }

    /// `callee(arguments)`: each argument is checked against its
    /// parameter's type, and the call has the callee's return type.
    fn check_call(&mut self, callee: &str, arguments: &[Expression]) -> Type {
        let Some(signature) = self.signatures.get(callee).cloned() else {
            // Unresolved: semantic analysis reports that, not the checker.
            for argument in arguments {
                self.check_expression(argument, None);
            }
            return Type::Error;
        };

        for (index, argument) in arguments.iter().enumerate() {
            // Semantic analysis has checked the argument count. An extra
            // argument can only come from a program that skipped it, and
            // is checked on its own.
            let Some((parameter, expected)) = signature.params.get(index) else {
                self.check_expression(argument, None);
                continue;
            };

            let found = self.check_expression(argument, Some(*expected));
            if !Type::compatible(*expected, found) {
                self.error(
                    TypeErrorKind::InvalidArgument {
                        function: callee.to_string(),
                        parameter: parameter.clone(),
                        position: index + 1,
                        expected: *expected,
                        found,
                    },
                    argument.span(),
                );
            }
        }

        signature.return_type
    }

    /// Record `ty` as the type of the expression at `span`. `Error` is
    /// never recorded: it is not a Dijk type, only the checker's marker
    /// for "already reported".
    fn record(&mut self, span: Span, ty: Type) {
        if ty != Type::Error {
            self.typed.expression_types.insert(span, ty);
        }
    }

    /// An integer literal (`42`, or `-42`) takes the integer type its
    /// context expects, or `i32` if the context expects no integer type,
    /// and its `value` must fit in that type.
    ///
    /// When the context expects something that is not an integer at all
    /// (`let b: bool = 3000000000;`), `i32` is only a fallback: the
    /// mistake is using an integer there, which whoever expected the
    /// `bool` reports. The literal's range is then not checked, so that
    /// one mistake produces one diagnostic.
    ///
    /// For `-42` both nodes, the `Unary(-)` and the `Integer(42)` inside
    /// it, get the literal's type: the minus sign is part of the literal.
    fn check_integer_literal(
        &mut self,
        expr: &Expression,
        value: i64,
        expected: Option<Type>,
    ) -> Type {
        let (ty, check_range) = match expected {
            Some(expected) if expected.is_integer() => (expected, true),
            // No context, or one that could not be typed: the default.
            None | Some(Type::Error) => (Type::I32, true),
            // Something other than an integer is wanted here.
            Some(_) => (Type::I32, false),
        };

        if check_range && !fits(value, ty) {
            self.error(
                TypeErrorKind::IntegerLiteralOutOfRange { value, ty },
                expr.span(),
            );
        }

        if let Expression::Unary { operand, .. } = expr {
            self.record(operand.span(), ty);
        }
        ty
    }

    /// A unary expression. A minus sign written directly before digits is
    /// not an operation at all: `-42` is a negative integer literal.
    /// Otherwise the operand is checked, then the operator's rule for its
    /// type is looked up.
    fn check_unary(
        &mut self,
        expr: &Expression,
        operator: UnaryOperator,
        operand: &Expression,
        expected: Option<Type>,
    ) -> Type {
        if let Some(value) = literal_value(expr) {
            return self.check_integer_literal(expr, value, expected);
        }

        // `-x` has the type of `x`, so what is expected of the result is
        // what is expected of the operand. `!x` is a `bool` whatever is
        // expected of it.
        let operand_expected = match operator {
            UnaryOperator::Neg => expected,
            UnaryOperator::Not => None,
        };
        let operand_ty = self.check_expression(operand, operand_expected);
        if operand_ty == Type::Error {
            return Type::Error;
        }

        match rules::unary_result(operator, operand_ty) {
            Some(result) => result,
            None => {
                self.error(
                    TypeErrorKind::InvalidUnaryOperation {
                        operator,
                        operand: operand_ty,
                    },
                    expr.span(),
                );
                Type::Error
            }
        }
    }

    /// A binary expression: check both operands, then look up the
    /// operator's rule for their types.
    fn check_binary(
        &mut self,
        expr: &Expression,
        left: &Expression,
        operator: BinaryOperator,
        right: &Expression,
        expected: Option<Type>,
    ) -> Type {
        // The result of `+ - * /` has the type of its operands, so what is
        // expected of the result is what is expected of them. A comparison
        // is a `bool` whatever its operands are, so nothing is passed down.
        let operand_expected = if rules::is_arithmetic(operator) {
            expected
        } else {
            None
        };

        // Check the operand that can decide the type first, and expect the
        // other one to have the same type. That is the left one, unless it
        // is a bare literal and the right one is not (`1 + x`).
        let (left_ty, right_ty) = if literal_value(left).is_some() && literal_value(right).is_none()
        {
            let right_ty = self.check_expression(right, operand_expected);
            let left_ty = self.check_expression(left, Some(right_ty));
            (left_ty, right_ty)
        } else {
            let left_ty = self.check_expression(left, operand_expected);
            let right_ty = self.check_expression(right, Some(left_ty));
            (left_ty, right_ty)
        };

        // An operand that could not be typed has already been reported.
        if left_ty == Type::Error || right_ty == Type::Error {
            return Type::Error;
        }

        match rules::binary_result(operator, left_ty, right_ty) {
            Some(result) => result,
            None => {
                self.error(
                    TypeErrorKind::InvalidBinaryOperation {
                        operator,
                        left: left_ty,
                        right: right_ty,
                    },
                    expr.span(),
                );
                Type::Error
            }
        }
    }
}

/// Whether running `statements` in order is certain to end in a `return`,
/// judged from their shape alone, without evaluating anything:
///
/// ```text
/// a block            returns if any of its statements returns (the
///                    statements after that one can never run)
/// return             returns
/// if … else …        returns if both branches return
/// if … (no else)     does not: the condition might be false
/// while …            does not: the condition might be false the first
///                    time, even `while true`, since the checker never
///                    evaluates a condition
/// let, expression    do not
/// ```
///
/// This is conservative: it never claims a function returns when it might
/// not, but it rejects some functions that always do in practice, such as
/// one whose only `return` is inside `while true { … }`.
fn always_returns(statements: &[Statement]) -> bool {
    statements.iter().any(statement_always_returns)
}

fn statement_always_returns(stmt: &Statement) -> bool {
    match stmt {
        Statement::Return { .. } => true,
        Statement::If {
            then_branch,
            else_branch: Some(else_branch),
            ..
        } => always_returns(then_branch) && always_returns(else_branch),
        Statement::If {
            else_branch: None, ..
        } => false,
        Statement::While { .. } | Statement::Let { .. } | Statement::Expression { .. } => false,
    }
}

/// The value of `expr` if it is an integer literal: digits (`42`), or a
/// minus sign written directly before digits (`-42`). Anything else,
/// including `-(-42)` or `0 - 42`, is an operation on literals, not a
/// literal.
///
/// The minus sign has to count as part of the literal. Otherwise
/// `-2147483648`, the smallest `i32`, could not be written: the parser
/// reads it as `-` applied to `2147483648`, which on its own is one more
/// than the largest `i32`.
///
/// The interpreter uses this same function, so that the two stages agree
/// on what a literal is: a literal the checker accepted as an `i32` is
/// also built as one at run time, without first building its digits.
pub fn literal_value(expr: &Expression) -> Option<i64> {
    match expr {
        Expression::Integer(value, _) => Some(*value),
        Expression::Unary {
            operator: UnaryOperator::Neg,
            operand,
            ..
        } => match operand.as_ref() {
            // The parser only produces non-negative literals, so the
            // negation always fits in an `i64`.
            Expression::Integer(value, _) => Some(-*value),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `value` can be represented in the integer type `ty`.
fn fits(value: i64, ty: Type) -> bool {
    match ty {
        Type::I32 => i32::try_from(value).is_ok(),
        // The parser has already rejected any literal beyond `i64`.
        _ => true,
    }
}
