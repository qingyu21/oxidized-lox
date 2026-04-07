use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::runtime::{RuntimeError, Value};
use crate::token::Token;

pub(crate) type EnvironmentRef = Rc<RefCell<Environment>>;

#[derive(Default)]
pub(crate) struct Environment {
    enclosing: Option<EnvironmentRef>,
    values: HashMap<String, Value>,
}

impl Environment {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    // Helpers for constructing shared handles to the global environment and
    // nested child environments used by block scope.
    pub(crate) fn new_ref() -> EnvironmentRef {
        Rc::new(RefCell::new(Self::new()))
    }

    pub(crate) fn from_enclosing(enclosing: EnvironmentRef) -> Self {
        Self {
            enclosing: Some(enclosing),
            values: HashMap::new(),
        }
    }

    pub(crate) fn new_enclosed_ref(enclosing: EnvironmentRef) -> EnvironmentRef {
        Rc::new(RefCell::new(Self::from_enclosing(enclosing)))
    }

    // Bind a value to a name in the current environment.
    pub(crate) fn define(&mut self, name: String, value: Value) {
        // TODO(perf): Environment keys currently clone each variable name into
        // an owned `String`. String interning or symbol IDs would avoid
        // repeating that allocation across declarations and lookups.
        self.values.insert(name, value);
    }

    // Update the value stored for an existing variable.
    pub(crate) fn assign(&mut self, name: &Token, value: Value) -> Result<(), RuntimeError> {
        if let Some(slot) = self.values.get_mut(&name.lexeme) {
            *slot = value;
            Ok(())
        } else if let Some(enclosing) = &self.enclosing {
            enclosing.borrow_mut().assign(name, value)
        } else {
            Err(RuntimeError::new(
                name.clone(),
                format!("Undefined variable '{}'.", name.lexeme),
            ))
        }
    }

    // Look up the current value stored for a variable name.
    pub(crate) fn get(&self, name: &Token) -> Result<Value, RuntimeError> {
        if let Some(value) = self.values.get(&name.lexeme) {
            // TODO(perf): Returning an owned `Value` clones strings and would
            // also clone any future heap-backed objects. Shared runtime
            // handles would make variable reads cheaper.
            Ok(value.clone())
        } else if let Some(enclosing) = &self.enclosing {
            enclosing.borrow().get(name)
        } else {
            Err(RuntimeError::new(
                name.clone(),
                format!("Undefined variable '{}'.", name.lexeme),
            ))
        }
    }

    // Update a binding in the ancestor environment selected by the resolver's
    // precomputed lexical distance.
    pub(crate) fn assign_at(
        environment: &EnvironmentRef,
        distance: usize,
        name: &Token,
        value: Value,
    ) -> Result<(), RuntimeError> {
        let ancestor = Self::ancestor(environment, distance);
        let mut ancestor = ancestor.borrow_mut();

        if let Some(slot) = ancestor.values.get_mut(&name.lexeme) {
            *slot = value;
            Ok(())
        } else {
            Err(RuntimeError::new(
                name.clone(),
                format!("Undefined variable '{}'.", name.lexeme),
            ))
        }
    }

    // Read a binding from the ancestor environment selected by the resolver's
    // precomputed lexical distance.
    pub(crate) fn get_at(
        environment: &EnvironmentRef,
        distance: usize,
        name: &Token,
    ) -> Result<Value, RuntimeError> {
        let ancestor = Self::ancestor(environment, distance);
        let ancestor = ancestor.borrow();

        if let Some(value) = ancestor.values.get(&name.lexeme) {
            Ok(value.clone())
        } else {
            Err(RuntimeError::new(
                name.clone(),
                format!("Undefined variable '{}'.", name.lexeme),
            ))
        }
    }

    // Walk outward `distance` scopes from `environment` and return that
    // ancestor environment handle.
    fn ancestor(environment: &EnvironmentRef, distance: usize) -> EnvironmentRef {
        let mut environment = environment.clone();

        for _ in 0..distance {
            let enclosing = {
                let environment_ref = environment.borrow();
                environment_ref
                    .enclosing
                    .clone()
                    .expect("resolver should only record valid scope distances")
            };
            environment = enclosing;
        }

        environment
    }
}
