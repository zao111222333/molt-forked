//! Variable Scope Stack
//!
//! A scope contains the variables for a given level in the call stack.  New scopes are pushed
//! onto the stack by procedure on entry and popped on exit.  Variables in the current scope
//! can be mapped to variables in higher scopes (e.g., scope 0, the `global` scope) using
//! the `upvar` method.
//!
//! Scopes are numbered starting at `0`, the `global` scope.  Scopes with lower indices than
//! the current are said to be higher in the stack, following Standard TCL practice (e.g.,
//! `upvar`, `uplevel`).
//!
//! Molt clients do not interact with this mechanism directly, but via the
//! `Interp` (or the Molt language itself).

use crate::types::Exception;
use crate::types::MoltList;
use crate::value::Value;
use std::collections::HashMap;
use std::fmt::Debug;

/// A variable in a `Scope`.  If the variable is defined in the given `Scope`, it is a
/// `Scalar` or an `Array`; if it is an alias to a variable in a higher scope (e.g., a global)
/// then the `Upvar` gives the referenced scope.
#[derive(Eq, PartialEq, Clone)]
enum Var {
    /// A scalar variable, with its value.
    Scalar(Value),

    /// An array variable, with its hash table from names to values.
    Array(HashMap<String, Value>),

    /// An alias to a variable at a higher stack level, with the referenced stack level.
    /// Note that aliases can chain.
    Upvar(usize),
}

impl Var {
    /// This is an upvar'd variable?
    fn is_upvar(&self) -> bool {
        matches!(self, Var::Upvar(_))
    }
}

impl Debug for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Var::Scalar(value) => write!(f, "Var::Scalar({})", value.as_str()),
            Var::Array(map) => write!(f, "Var::Array({} elements)", map.len()),
            Var::Upvar(level) => write!(f, "Var::Upvar({})", level),
        }
    }
}

/// A scope: a level in the `ScopeStack`.  It contains a hash table of `Var`'s by name.
/// Scopes may be pushed onto the stack and popped off later.  Most typically, a scope is
/// pushed on the stack by a `proc` before executing its body, and then popped afterwards.
#[derive(Default, Debug, Clone)]
struct Scope {
    /// Vars in this scope by name.
    map: HashMap<String, Var>,
}

/// The scope stack: a stack of variable scopes corresponding to the Molt `proc`
/// call stack.
#[derive(Debug, Clone)]
pub(crate) struct ScopeStack {
    stack: Vec<Scope>,
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeStack {
    //-------------------------------------------------------------------------------------
    // Public API

    /// Creates a scope stack containing only scope `0`, the global scope.  This is usually
    /// done once, as part of creating an `Interp`.
    pub fn new() -> Self {
        Self { stack: vec![Scope::default()] }
    }

    /// Requires the value of the named scalar variable in the current scope.
    pub fn get(&self, name: &str) -> Result<Value, Exception> {
        match self.var(self.current(), name) {
            Some(Var::Scalar(value)) => Ok(value.clone()),
            Some(Var::Array(_)) => {
                molt_err!("can't read \"{}\": variable is array", name)
            }
            Some(_) => unreachable!(),
            None => molt_err!("can't read \"{}\": no such variable", name),
        }
    }

    /// Requires the value of an array element given its variable name and index.
    pub fn get_elem(&self, name: &str, index: &str) -> Result<Value, Exception> {
        match self.var(self.current(), name) {
            Some(Var::Scalar(_)) => {
                molt_err!("can't read \"{}({})\": variable isn't array", name, index)
            }
            Some(Var::Array(map)) => {
                if let Some(val) = map.get(index) {
                    Ok(val.clone())
                } else {
                    molt_err!(
                        "can't read \"{}({})\": no such element in array",
                        name,
                        index
                    )
                }
            }
            Some(_) => unreachable!(),
            None => molt_err!("can't read \"{}\": no such variable", name),
        }
    }

    /// Sets the value of the named scalar in the global scope, creating the variable
    /// if it doesn't already exist.  It's an error if the variable exists but is an array
    /// variable.
    pub fn set_global(&mut self, name: &str, val: Value) -> Result<(), Exception> {
        self.set_at(0, name, val)
    }

    /// Sets the value of the named scalar in the current scope, creating the variable
    /// if it doesn't already exist.  It's an error if the variable exists but is an array
    /// variable.
    pub fn set(&mut self, name: &str, val: Value) -> Result<(), Exception> {
        self.set_at(self.current(), name, val)
    }

    /// Sets the value of the indexed array element in the current scope, creating the
    /// and/or the element if they don't already exist. It's an error if the variable exists
    /// but is a scalar variable.
    pub fn set_elem(
        &mut self,
        name: &str,
        index: &str,
        val: Value,
    ) -> Result<(), Exception> {
        let level = self.resolved_level(self.current(), name);
        let scope = &mut self.stack[level].map;

        if let Some(var) = scope.get_mut(name) {
            return match var {
                Var::Upvar(_) => unreachable!(),
                Var::Scalar(_) => {
                    molt_err!("can't set \"{}({})\": variable isn't array", name, index)
                }
                Var::Array(map) => {
                    map.insert(index.into(), val);
                    Ok(())
                }
            };
        }

        let mut map = HashMap::new();
        map.insert(index.into(), val);
        scope.insert(name.into(), Var::Array(map));
        Ok(())
    }

    /// Returns true if there's a variable with the given name, of whatever type, and
    /// false otherwise.
    pub fn exists(&self, name: &str) -> bool {
        self.var(self.current(), name).is_some()
    }

    /// Returns true if there's a variable with the given name, of whatever type, and
    /// false otherwise.
    pub fn elem_exists(&self, name: &str, index: &str) -> bool {
        self.get_elem(name, index).is_ok()
    }

    /// Unsets a variable in the current scope, i.e., removes it from the scope.
    /// If the variable is a reference to another scope, the variable is removed from that
    /// scope as well.
    ///
    /// Note: it's irrelevant whether the variable is a scalar or array; it's going away.
    pub fn unset(&mut self, name: &str) {
        self.unset_at(self.current(), name, false);
    }

    /// Unset a variable at a given level in the stack.  If the variable at that level
    /// is linked to a higher level, follows the chain down, unsetting as it goes.
    fn unset_at(&mut self, level: usize, name: &str, array_only: bool) {
        let mut level = level;

        loop {
            let next = match self.stack[level].map.get(name) {
                Some(Var::Upvar(next)) => Some(*next),
                _ => None,
            };

            if !array_only
                || matches!(self.stack[level].map.get(name), Some(Var::Array(_)))
            {
                self.stack[level].map.remove(name);
            }

            match next {
                Some(next) => level = next,
                None => break,
            }
        }
    }

    /// Links a variable in the current scope to variable at the given level, counting
    /// from `0`, the global scope.
    ///
    /// **Note:** does not try to create the variable at the referenced scope level, if it
    /// does not exist; the variable will be created on the first `set`, if any.  This is
    /// consistent with standard TCL behavior.
    pub fn upvar(&mut self, level: usize, name: &str) {
        assert!(level < self.current(), "Can't upvar to current stack level");
        let top = self.current();
        self.stack[top].map.insert(name.into(), Var::Upvar(level));
    }

    /// Returns the index of the current stack level, counting from 0, the global scope.
    /// The current stack level has the highest index, but is said to be the lowest stack
    /// level.
    pub fn current(&self) -> usize {
        self.stack.len() - 1
    }

    /// Pushes a new scope onto the stack.  The scope contains no variables by default, though
    /// the procedure that is pushing it onto the stack will often add some.
    pub fn push(&mut self) {
        self.stack.push(Scope::default());
    }

    /// Pops the current scope from the stack. Panics if we're at the global scope; this implies an
    /// coding error at the Rust level.
    pub fn pop(&mut self) {
        self.stack.pop();
        assert!(!self.stack.is_empty(), "Popped global scope!");
    }

    /// Gets a list of the names of the variables defined in the current scope.
    pub fn vars_in_scope(&self) -> MoltList {
        self.stack[self.current()].map.keys().map(Value::from).collect()
    }

    /// Gets a list of the local variables defined in the current scope.  Upvar'd variables
    /// are not local; and no variables are local in the global scope.
    pub fn vars_in_local_scope(&self) -> MoltList {
        // If we are at the global scope, there are no local variables.
        if self.current() == 0 {
            return Vec::new();
        }

        self.stack[self.current()]
            .map
            .iter()
            .filter(|(_, v)| !v.is_upvar())
            .map(|(k, _)| Value::from(k))
            .collect()
    }

    /// Gets a list of the variables defined in the global scope.
    pub fn vars_in_global_scope(&self) -> MoltList {
        self.stack[0].map.keys().map(Value::from).collect()
    }

    /// Determines whether the name names an array variable or not.
    pub fn array_exists(&self, name: &str) -> bool {
        matches!(self.var(self.current(), name), Some(Var::Array(_)))
    }

    /// Gets a list of the array indices for the named array.  Returns the empty list
    /// if `name` doesn't name an array variable.
    pub fn array_indices(&self, name: &str) -> MoltList {
        match self.var(self.current(), name) {
            Some(Var::Array(map)) => map.keys().map(Value::from).collect(),
            _ => Vec::new(),
        }
    }

    /// Gets the size of the named array.  Returns 0 if `name` doesn't name an array variable.
    pub fn array_size(&self, name: &str) -> usize {
        match self.var(self.current(), name) {
            Some(Var::Array(map)) => map.len(),
            _ => 0,
        }
    }

    /// Gets the content of an array as a flat list of names and values.  If the named
    /// variable is not an array, returns the empty list.
    pub fn array_get(&self, name: &str) -> MoltList {
        match self.var(self.current(), name) {
            Some(Var::Array(map)) => {
                let mut list = Vec::with_capacity(map.len() * 2);

                for (key, value) in map {
                    list.push(Value::from(key));
                    list.push(value.clone());
                }
                list
            }
            _ => Vec::new(),
        }
    }

    /// Unsets the value of the indexed array element in the current scope, if it exists.
    /// Does nothing if the array element doesn't exist, or the variable isn't an array
    /// variable.
    pub fn unset_element(&mut self, name: &str, index: &str) {
        let level = self.resolved_level(self.current(), name);
        if let Some(Var::Array(map)) = self.stack[level].map.get_mut(name) {
            map.remove(index);
        }
    }

    /// Merges a flat list of keys and values into the array variable, creating the variable
    /// if it doesn't exist. It's an error if the variable exists but is a scalar variable.
    pub fn array_set(&mut self, name: &str, kvlist: &[Value]) -> Result<(), Exception> {
        // List must be even.
        assert!(kvlist.len().is_multiple_of(2));

        let level = self.resolved_level(self.current(), name);
        let scope = &mut self.stack[level].map;

        if let Some(var) = scope.get_mut(name) {
            return match var {
                Var::Upvar(_) => unreachable!(),
                Var::Scalar(_) => {
                    molt_err!("can't array set \"{}\": variable isn't array", name)
                }
                Var::Array(map) => {
                    insert_kvlist(map, kvlist);
                    Ok(())
                }
            };
        }

        let mut map = HashMap::new();
        insert_kvlist(&mut map, kvlist);
        scope.insert(name.into(), Var::Array(map));
        Ok(())
    }

    /// Unsets an array variable in the current scope, i.e., removes it from the scope.
    /// If the variable is a reference to another scope, the variable is removed from that
    /// scope as well.
    ///
    /// Only affects array variables.
    pub fn array_unset(&mut self, name: &str) {
        self.unset_at(self.current(), name, true);
    }

    //--------------------------------------------------------------
    // Utilities

    /// Retrieves an immutable borrow of the variable of the given name, searching the
    /// the scope stack for the variable starting at the current level and following the
    /// alias chain as needed.
    ///
    /// This call is the basis for all public APIs that retrieve information about a variable.
    ///
    fn var(&self, level: usize, name: &str) -> Option<&Var> {
        let level = self.resolved_level(level, name);
        self.stack[level].map.get(name)
    }

    /// Resolves an upvar chain without borrowing the final variable mutably.
    fn resolved_level(&self, mut level: usize, name: &str) -> usize {
        while let Some(Var::Upvar(next)) = self.stack[level].map.get(name) {
            level = *next;
        }
        level
    }

    /// Sets a scalar at a specific level after resolving any upvar chain.
    fn set_at(&mut self, level: usize, name: &str, val: Value) -> Result<(), Exception> {
        let level = self.resolved_level(level, name);
        let scope = &mut self.stack[level].map;

        if let Some(var) = scope.get_mut(name) {
            return match var {
                Var::Upvar(_) => unreachable!(),
                Var::Array(_) => molt_err!("can't set \"{}\": variable is array", name),
                Var::Scalar(current) => {
                    *current = val;
                    Ok(())
                }
            };
        }

        scope.insert(name.into(), Var::Scalar(val));
        Ok(())
    }
}

// Insert the flat key-value list into the map.
fn insert_kvlist(map: &mut HashMap<String, Value>, list: &[Value]) {
    for kv in list.chunks(2) {
        map.insert(kv[0].as_str().into(), kv[1].clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let ss = ScopeStack::new();
        assert_eq!(ss.stack.len(), 1);
        assert_eq!(ss.current(), 0);
    }

    #[test]
    fn test_set_get_basic() {
        let mut ss = ScopeStack::new();

        let _ = ss.set("a", Value::from("1"));
        let out = ss.get("a");
        assert_eq!(out.unwrap().as_str(), "1");

        assert_eq!(ss.get("b"), molt_err!("can't read \"b\": no such variable"));

        let _ = ss.set_elem("c", "1", "one".into());
        assert_eq!(ss.get("c"), molt_err!("can't read \"c\": variable is array"));
    }

    #[test]
    fn test_set_get_global() {
        let mut ss = ScopeStack::new();

        let _ = ss.set_global("a", Value::from("1"));
        let out = ss.get("a");
        assert_eq!(out.unwrap().as_str(), "1");

        ss.push();
        let _ = ss.set_global("a", Value::from("2"));
        ss.pop();

        let out = ss.get("a");
        assert_eq!(out.unwrap().as_str(), "2");
    }

    #[test]
    fn test_set_get_elem() {
        let mut ss = ScopeStack::new();

        let _ = ss.set_elem("a", "1", Value::from("one"));
        let out = ss.get_elem("a", "1");
        assert_eq!(out.unwrap().as_str(), "one");

        assert_eq!(
            ss.get_elem("b", "1"),
            molt_err!("can't read \"b\": no such variable")
        );

        let _ = ss.set_elem("c", "1", "one".into());
        assert_eq!(
            ss.get_elem("c", "2"),
            molt_err!("can't read \"c(2)\": no such element in array")
        );

        let _ = ss.set("d", "".into());
        assert_eq!(
            ss.get_elem("d", "1"),
            molt_err!("can't read \"d(1)\": variable isn't array")
        );
    }

    #[test]
    fn test_unset_basic() {
        let mut ss = ScopeStack::new();

        let _ = ss.set("a", Value::from("1"));
        assert!(ss.get("a").is_ok());
        ss.unset("a");
        assert!(ss.get("a").is_err());
    }

    #[test]
    fn test_push() {
        let mut ss = ScopeStack::new();
        ss.push();
        assert_eq!(ss.stack.len(), 2);
        ss.push();
        assert_eq!(ss.stack.len(), 3);
    }

    #[test]
    fn test_pop() {
        let mut ss = ScopeStack::new();
        ss.push();
        ss.push();
        assert_eq!(ss.stack.len(), 3);
        ss.pop();
        assert_eq!(ss.stack.len(), 2);
        ss.pop();
        assert_eq!(ss.stack.len(), 1);
    }

    #[test]
    #[should_panic]
    fn test_pop_global_scope() {
        let mut ss = ScopeStack::new();
        assert_eq!(ss.stack.len(), 1);
        ss.pop();
    }

    #[test]
    fn test_current() {
        let mut ss = ScopeStack::new();
        assert_eq!(ss.current(), 0);
        ss.push();
        assert_eq!(ss.current(), 1);
        ss.push();
        assert_eq!(ss.current(), 2);
        ss.pop();
        assert_eq!(ss.current(), 1);
        ss.pop();
        assert_eq!(ss.current(), 0);
    }

    #[test]
    fn test_set_levels() {
        let mut ss = ScopeStack::new();

        let _ = ss.set("a", Value::from("1"));
        let _ = ss.set("b", Value::from("2"));

        ss.push();
        assert!(ss.get("a").is_err());
        assert!(ss.get("b").is_err());
        assert!(ss.get("c").is_err());

        let _ = ss.set("a", Value::from("3"));
        let _ = ss.set("b", Value::from("4"));
        let _ = ss.set("c", Value::from("5"));
        assert_eq!(ss.get("a").unwrap().as_str(), "3");
        assert_eq!(ss.get("b").unwrap().as_str(), "4");
        assert_eq!(ss.get("c").unwrap().as_str(), "5");

        ss.pop();
        assert_eq!(ss.get("a").unwrap().as_str(), "1");
        assert_eq!(ss.get("b").unwrap().as_str(), "2");
        assert!(ss.get("c").is_err());
    }

    #[test]
    fn test_set_get_upvar() {
        let mut ss = ScopeStack::new();

        let _ = ss.set("a", Value::from("1"));
        let _ = ss.set("b", Value::from("2"));

        ss.push();
        ss.upvar(0, "a");
        assert_eq!(ss.get("a").unwrap().as_str(), "1");
        assert!(ss.get("b").is_err());

        let _ = ss.set("a", Value::from("3"));
        let _ = ss.set("b", Value::from("4"));
        assert_eq!(ss.get("a").unwrap().as_str(), "3");
        assert_eq!(ss.get("b").unwrap().as_str(), "4");

        ss.pop();
        assert_eq!(ss.get("a").unwrap().as_str(), "3");
        assert_eq!(ss.get("b").unwrap().as_str(), "2");
    }

    #[test]
    fn test_unset_levels() {
        let mut ss = ScopeStack::new();

        let _ = ss.set("a", Value::from("1"));
        let _ = ss.set("b", Value::from("2"));

        ss.push();
        let _ = ss.set("a", Value::from("3"));

        ss.unset("a"); // Was set in this scope
        ss.unset("b"); // Was not set in this scope

        ss.pop();
        assert_eq!(ss.get("a").unwrap().as_str(), "1");
        assert_eq!(ss.get("b").unwrap().as_str(), "2");
    }

    #[test]
    fn test_unset_upvar() {
        let mut ss = ScopeStack::new();

        // Set a value at level 0
        let _ = ss.set("a", Value::from("1"));
        assert!(ss.get("a").is_ok());
        ss.push();
        assert!(ss.get("a").is_err());

        // Link a@1 to a@0
        ss.upvar(0, "a");
        assert!(ss.get("a").is_ok());

        // Unset it; it should be unset in both scopes.
        ss.unset("a");

        assert!(ss.get("a").is_err());
        ss.pop();
        assert!(ss.get("a").is_err());
    }

    #[test]
    fn test_vars_in_scope() {
        let mut ss = ScopeStack::new();
        // No vars initially
        assert_eq!(ss.vars_in_scope().len(), 0);

        // Add two vars to current scope
        let _ = ss.set("a", Value::from("1"));
        let _ = ss.set("b", Value::from("2"));
        assert_eq!(ss.vars_in_scope().len(), 2);
        assert!(ss.vars_in_scope().contains(&Value::from("a")));
        assert!(ss.vars_in_scope().contains(&Value::from("b")));

        // Push a scope; no vars initially
        ss.push();
        assert_eq!(ss.vars_in_scope().len(), 0);

        // Add a var
        let _ = ss.set("c", Value::from("3"));
        assert_eq!(ss.vars_in_scope().len(), 1);
        assert!(ss.vars_in_scope().contains(&Value::from("c")));

        // Upvar a var
        ss.upvar(0, "a");
        assert_eq!(ss.vars_in_scope().len(), 2);
        assert!(ss.vars_in_scope().contains(&Value::from("a")));

        // Pop a scope
        ss.pop();
        assert_eq!(ss.vars_in_scope().len(), 2);
        assert!(!ss.vars_in_scope().contains(&Value::from("c")));

        // Unset a var
        ss.unset("b");
        assert_eq!(ss.vars_in_scope().len(), 1);
        assert!(!ss.vars_in_scope().contains(&Value::from("b")));
    }

    #[test]
    fn test_vars_in_local_scope() {
        let mut ss = ScopeStack::new();

        // Add var to global scope.  It isn't local.
        ss.set("a", Value::from("1")).expect("ok");
        assert!(ss.vars_in_local_scope().is_empty());

        // Push a scope; no vars initially
        ss.push();
        assert!(ss.vars_in_scope().is_empty());

        // Add vars to local scope
        ss.set("a", Value::from("1")).expect("ok");
        ss.set_elem("b", "1", Value::from("1")).expect("ok");
        assert_eq!(ss.vars_in_local_scope().len(), 2);
        assert!(ss.vars_in_local_scope().contains(&Value::from("a")));
        assert!(ss.vars_in_local_scope().contains(&Value::from("b")));

        // Upvar a var; it isn't local.
        ss.upvar(0, "c");
        assert_eq!(ss.vars_in_local_scope().len(), 2);
        assert!(!ss.vars_in_local_scope().contains(&Value::from("c")));

        // Push a scope; no local vars
        ss.push();
        assert!(ss.vars_in_scope().is_empty());
    }

    #[test]
    fn test_vars_in_global_scope() {
        let mut ss = ScopeStack::new();

        assert!(ss.vars_in_global_scope().is_empty());

        // Add vars to global scope.
        ss.set("a", Value::from("1")).expect("ok");
        ss.set_elem("b", "1", Value::from("1")).expect("ok");
        assert!(ss.vars_in_global_scope().len() == 2);
        assert!(ss.vars_in_global_scope().contains(&Value::from("a")));
        assert!(ss.vars_in_global_scope().contains(&Value::from("b")));
        assert!(!ss.vars_in_global_scope().contains(&Value::from("c")));

        // Push a scope.  No change.
        ss.push();
        assert!(ss.vars_in_global_scope().len() == 2);
        assert!(ss.vars_in_global_scope().contains(&Value::from("a")));
        assert!(ss.vars_in_global_scope().contains(&Value::from("b")));
        assert!(!ss.vars_in_global_scope().contains(&Value::from("c")));

        // Add a var to local scope. No change.
        ss.set("c", Value::from("1")).expect("ok");

        assert!(ss.vars_in_global_scope().len() == 2);
        assert!(ss.vars_in_global_scope().contains(&Value::from("a")));
        assert!(ss.vars_in_global_scope().contains(&Value::from("b")));
        assert!(!ss.vars_in_global_scope().contains(&Value::from("c")));
    }

    #[test]
    fn test_global() {
        // Verify that we can upvar to a variable that doesn't yet exist.
        // Check both scalars and array elements.
        let mut ss = ScopeStack::new();

        ss.push();
        ss.upvar(0, "a");
        ss.upvar(0, "b");
        ss.set("a", Value::from("1")).unwrap();
        ss.set_elem("b", "1", Value::from("2")).unwrap();
        ss.pop();

        let out = ss.get("a").unwrap();
        assert_eq!(out.as_str(), "1");

        let out = ss.get_elem("b", "1").unwrap();
        assert_eq!(out.as_str(), "2");
    }

    #[test]
    fn test_upvar_chain() {
        let mut ss = ScopeStack::new();
        ss.set("value", "global".into()).unwrap();

        ss.push();
        ss.upvar(0, "value");
        ss.push();
        ss.upvar(1, "value");
        ss.set("value", "updated".into()).unwrap();

        assert_eq!(ss.get("value").unwrap().as_str(), "updated");
        ss.pop();
        assert_eq!(ss.get("value").unwrap().as_str(), "updated");
        ss.pop();
        assert_eq!(ss.get("value").unwrap().as_str(), "updated");
    }

    #[test]
    fn test_array_indices() {
        let mut ss = ScopeStack::new();

        let _ = ss.set("a", "zero".into());
        let _ = ss.set_elem("b", "1", "one".into());
        let _ = ss.set_elem("b", "2", "two".into());

        assert_eq!(ss.array_indices("x"), Vec::new());
        assert_eq!(ss.array_indices("a"), Vec::new());

        let list = ss.array_indices("b");
        assert!(list.len() == 2);
        assert!(list.contains(&"1".into()));
        assert!(list.contains(&"2".into()));
    }

    #[test]
    fn test_array_size() {
        let mut ss = ScopeStack::new();

        let _ = ss.set("a", "zero".into());
        let _ = ss.set_elem("b", "1", "one".into());
        let _ = ss.set_elem("b", "2", "two".into());

        assert_eq!(ss.array_size("x"), 0);
        assert_eq!(ss.array_size("a"), 0);
        assert_eq!(ss.array_size("b"), 2);
    }

    #[test]
    fn test_array_get() {
        let mut ss = ScopeStack::new();

        let _ = ss.set("a", "zero".into());
        let _ = ss.set_elem("b", "1", "one".into());
        let _ = ss.set_elem("b", "2", "two".into());

        assert_eq!(ss.array_get("x"), Vec::new());
        assert_eq!(ss.array_get("a"), Vec::new());

        let list = ss.array_get("b");
        assert!(list.len() == 4);
        assert!(list.contains(&"1".into()));
        assert!(list.contains(&"one".into()));
        assert!(list.contains(&"2".into()));
        assert!(list.contains(&"two".into()));
    }

    #[test]
    fn test_unset_element() {
        let mut ss = ScopeStack::new();

        let _ = ss.set("a", "zero".into());
        let _ = ss.set_elem("b", "1", "one".into());
        let _ = ss.set_elem("b", "2", "two".into());

        // Array unset of an unknown variable has no effect.
        ss.unset_element("x", "1"); // No error

        // Array unset of a scalar has no effect.
        ss.unset_element("a", "1");
        let out = ss.get("a");
        assert!(out.is_ok());
        assert_eq!(out.unwrap().as_str(), "zero");

        // Array unset of an element unsets just that element.
        ss.unset_element("b", "1");
        assert!(ss.get_elem("b", "1").is_err());
        assert!(ss.get_elem("b", "2").is_ok());
    }

    #[test]
    fn test_array_set() {
        let kvlist: MoltList = vec!["a".into(), "1".into(), "b".into(), "2".into()];

        let mut ss = ScopeStack::new();

        // Can create variable
        assert!(ss.array_set("x", &kvlist).is_ok());
        assert_eq!(ss.get_elem("x", "a").unwrap().as_str(), "1");
        assert_eq!(ss.get_elem("x", "b").unwrap().as_str(), "2");
        assert!(ss.get_elem("x", "c").is_err());

        // Can merge into  variable
        assert!(ss.set_elem("y", "a", "0".into()).is_ok());
        assert!(ss.set_elem("y", "b", "0".into()).is_ok());
        assert!(ss.set_elem("y", "c", "0".into()).is_ok());
        assert!(ss.array_set("y", &kvlist).is_ok());
        assert_eq!(ss.get_elem("y", "a").unwrap().as_str(), "1");
        assert_eq!(ss.get_elem("y", "b").unwrap().as_str(), "2");
        assert_eq!(ss.get_elem("y", "c").unwrap().as_str(), "0");

        // Can't update scalar
        assert!(ss.set("z", "0".into()).is_ok());
        assert_eq!(
            ss.array_set("z", &kvlist),
            molt_err!("can't array set \"z\": variable isn't array")
        );
    }

    #[test]
    fn test_exists() {
        let mut ss = ScopeStack::new();
        ss.set("a", "1".into()).expect("success");
        ss.set_elem("b", "1", "2".into()).expect("success");

        assert!(!ss.exists("nonesuch"));
        assert!(!ss.elem_exists("nonesuch", "1"));
        assert!(!ss.elem_exists("b", "2"));

        assert!(ss.exists("a"));
        assert!(ss.exists("b"));
        assert!(ss.elem_exists("b", "1"));
    }
}
