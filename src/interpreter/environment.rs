//! The environment: where a running program's variables live.
//!
//! At its simplest, an environment is a table from names to values:
//!
//! ```text
//! let x = 10;        Environment
//! let y = x + 20;    ┌──────────────┐
//!                    │ x → 10       │
//!                    │ y → 30       │
//!                    └──────────────┘
//! ```
//!
//! Variables live in scopes, though, and scopes nest. So an environment
//! is really a *stack* of tables, one per scope that is currently open,
//! innermost last:
//!
//! ```text
//! Function scope   x → 10          ← bottom: opened when the call starts
//!     │
//!     └── Block scope   y → 20     ← top: opened when a block is entered,
//!                                     discarded when it ends
//! ```
//!
//! This is the run-time twin of the scope stack semantic analysis keeps
//! (`crate::semantic`). The resolver's scopes record only *which* names
//! exist, to check the program; these record each name's current
//! *value*, to run it. Because semantic analysis has already checked the
//! program against the same scoping rules, a name the interpreter looks
//! up is always found.
//!
//! An environment holds variables only, and its outermost scope is a
//! *function* scope, not a global one. Dijk has no global variables: its
//! only global names are functions, which are not values and are not
//! stored here.

use super::value::Value;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Environment {
    /// Open scopes, innermost last. Never empty: the first entry is the
    /// function scope, which lives as long as the environment does.
    scopes: Vec<HashMap<String, Value>>,
}

impl Environment {
    /// A new environment holding one empty scope: the function scope.
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// Enter a block: open a new, empty innermost scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Leave a block: discard the innermost scope, and with it every
    /// variable declared in it. The function scope is never discarded.
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Declare `name` in the innermost scope with an initial `value`, as
    /// `let` does. A variable of the same name in an outer scope is
    /// shadowed, not changed. (Declaring the same name twice in one scope
    /// simply replaces it here; semantic analysis rejects such programs
    /// before they run.)
    pub fn define(&mut self, name: &str, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), value);
        }
    }

    /// The current value of `name`, searching from the innermost scope
    /// outward so that a shadowing variable hides the one it shadows.
    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    /// Change the value of the existing variable `name`, as `x = ...`
    /// does. The change goes to the innermost scope that declares `name`,
    /// which is the same variable `lookup` would find. Returns `false`,
    /// changing nothing, if no open scope declares `name`: assignment
    /// never creates a variable.
    pub fn assign(&mut self, name: &str, value: Value) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(slot) = scope.get_mut(name) {
                *slot = value;
                return true;
            }
        }
        false
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
