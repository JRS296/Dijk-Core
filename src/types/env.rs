//! The type environment: what type each variable in scope has.
//!
//! Written Γ (gamma) in typing rules, the environment is the "given" in
//! every judgment the checker makes. `Γ ⊢ x + 1 : i32` reads "in the
//! environment Γ, `x + 1` has type `i32`", and whether that holds depends
//! on what Γ says about `x`:
//!
//! ```text
//! let x = 10;          Γ
//! let ok = true;       ┌─────────────────┐
//!                      │ x  → i32        │
//!                      │ ok → bool       │
//!                      └─────────────────┘
//! ```
//!
//! Like the scopes semantic analysis keeps, it is a stack of tables, one
//! per open scope, innermost last, searched from the top down. This
//! makes it the third of three twins:
//!
//! ```text
//! semantic analysis   name → (exists)     which names are visible?
//! type checker        name → Type         what kind of value is each?
//! interpreter         name → Value        what value does each hold now?
//! ```
//!
//! All three follow exactly the same scoping rules, so a name that
//! resolves in one resolves to the same declaration in the others.

use super::ty::Type;
use std::collections::HashMap;

#[derive(Debug)]
pub struct TypeEnvironment {
    /// Open scopes, innermost last. Never empty: the first entry is the
    /// function scope.
    scopes: Vec<HashMap<String, Type>>,
}

impl TypeEnvironment {
    /// A new environment holding one empty scope: a function scope.
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// Enter a block: open a new, empty innermost scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Leave a block: forget every variable it declared. The function
    /// scope is never discarded.
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// How deeply nested the innermost scope is: 0 for the function
    /// scope, 1 for a block directly inside it, and so on.
    pub fn depth(&self) -> usize {
        self.scopes.len() - 1
    }

    /// Declare `name` with type `ty` in the innermost scope, as `let`
    /// does. An outer variable of the same name is shadowed, not changed.
    pub fn declare(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
    }

    /// The type of the variable `name` visible here, searching from the
    /// innermost scope outward.
    pub fn lookup(&self, name: &str) -> Option<Type> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}
