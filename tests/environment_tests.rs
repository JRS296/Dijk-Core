//! Tests for the interpreter's environment (`crate::interpreter::environment`):
//! defining, looking up and assigning variables across nested scopes.
//!
//! The environment is a plain data structure, so these tests exercise it
//! directly, with no source code, parser or interpreter involved.

// The environment is not part of a library's public surface (dijk is a
// binary crate), so this test includes it directly by path. It depends
// only on `value.rs` (through `super::value`), so only those two files
// are pulled in, not the rest of the interpreter. `Value`'s helpers are
// unused here, hence the `allow`.
#[path = "../src/interpreter"]
mod interpreter {
    #[path = "environment.rs"]
    pub mod environment;
    #[allow(dead_code)]
    #[path = "value.rs"]
    pub mod value;
}

use interpreter::environment::Environment;
use interpreter::value::Value;

fn int(value: i64) -> Value {
    Value::Integer(value)
}

// --- definition and lookup ---

#[test]
fn a_defined_variable_can_be_looked_up() {
    let mut env = Environment::new();
    env.define("x", int(10));
    assert_eq!(env.lookup("x"), Some(&int(10)));
}

#[test]
fn an_undefined_variable_is_not_found() {
    let env = Environment::new();
    assert_eq!(env.lookup("x"), None);
}

#[test]
fn variables_hold_any_kind_of_value() {
    let mut env = Environment::new();
    env.define("n", int(1));
    env.define("b", Value::Boolean(true));
    env.define("s", Value::String("hi".to_string()));
    env.define("u", Value::Unit);
    assert_eq!(env.lookup("n"), Some(&int(1)));
    assert_eq!(env.lookup("b"), Some(&Value::Boolean(true)));
    assert_eq!(env.lookup("s"), Some(&Value::String("hi".to_string())));
    assert_eq!(env.lookup("u"), Some(&Value::Unit));
}

#[test]
fn redefining_in_the_same_scope_replaces_the_value() {
    let mut env = Environment::new();
    env.define("x", int(1));
    env.define("x", int(2));
    assert_eq!(env.lookup("x"), Some(&int(2)));
}

// --- assignment ---

#[test]
fn assignment_changes_an_existing_variable() {
    let mut env = Environment::new();
    env.define("x", int(1));
    assert!(env.assign("x", int(2)));
    assert_eq!(env.lookup("x"), Some(&int(2)));
}

#[test]
fn assignment_never_creates_a_variable() {
    let mut env = Environment::new();
    assert!(!env.assign("x", int(1)));
    assert_eq!(env.lookup("x"), None);
}

// --- nested scopes ---

#[test]
fn outer_variables_are_visible_from_an_inner_scope() {
    let mut env = Environment::new();
    env.define("x", int(10));
    env.push_scope();
    assert_eq!(env.lookup("x"), Some(&int(10)));
}

#[test]
fn an_inner_variable_disappears_when_its_scope_ends() {
    let mut env = Environment::new();
    env.push_scope();
    env.define("y", int(20));
    assert_eq!(env.lookup("y"), Some(&int(20)));
    env.pop_scope();
    assert_eq!(env.lookup("y"), None);
}

#[test]
fn assigning_to_an_outer_variable_from_an_inner_scope_outlives_the_scope() {
    let mut env = Environment::new();
    env.define("total", int(0));
    env.push_scope();
    assert!(env.assign("total", int(5)));
    env.pop_scope();
    assert_eq!(env.lookup("total"), Some(&int(5)));
}

#[test]
fn scopes_nest_several_levels_deep() {
    let mut env = Environment::new();
    env.define("a", int(1));
    env.push_scope();
    env.define("b", int(2));
    env.push_scope();
    env.define("c", int(3));

    assert_eq!(env.lookup("a"), Some(&int(1)));
    assert_eq!(env.lookup("b"), Some(&int(2)));
    assert_eq!(env.lookup("c"), Some(&int(3)));

    env.pop_scope();
    assert_eq!(env.lookup("c"), None);
    assert_eq!(env.lookup("b"), Some(&int(2)));

    env.pop_scope();
    assert_eq!(env.lookup("b"), None);
    assert_eq!(env.lookup("a"), Some(&int(1)));
}

#[test]
fn the_function_scope_is_never_popped() {
    let mut env = Environment::new();
    env.define("x", int(1));
    env.pop_scope();
    assert_eq!(env.lookup("x"), Some(&int(1)));
}

// --- shadowing ---

#[test]
fn an_inner_variable_shadows_an_outer_one() {
    let mut env = Environment::new();
    env.define("x", int(1));
    env.push_scope();
    env.define("x", Value::String("inner".to_string()));
    assert_eq!(env.lookup("x"), Some(&Value::String("inner".to_string())));
}

#[test]
fn the_shadowed_variable_reappears_unchanged_when_the_scope_ends() {
    let mut env = Environment::new();
    env.define("x", int(1));
    env.push_scope();
    env.define("x", int(2));
    env.pop_scope();
    assert_eq!(env.lookup("x"), Some(&int(1)));
}

#[test]
fn assignment_changes_the_innermost_variable_of_that_name() {
    let mut env = Environment::new();
    env.define("x", int(1));
    env.push_scope();
    env.define("x", int(2));
    assert!(env.assign("x", int(3)));
    assert_eq!(env.lookup("x"), Some(&int(3)));
    env.pop_scope();
    // The outer `x` was hidden, so the assignment never touched it.
    assert_eq!(env.lookup("x"), Some(&int(1)));
}
