use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::interpreter::{RuntimeError, Value};
use crate::token::Token;

pub type EnvironmentRef = Rc<RefCell<Environment>>;

#[derive(Default)]
pub struct Environment {
    enclosing: Option<EnvironmentRef>,
    values: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }

    // Helpers for constructing shared handles to the global environment and
    // nested child environments used by block scope.
    pub fn new_ref() -> EnvironmentRef {
        Rc::new(RefCell::new(Self::new()))
    }

    pub fn from_enclosing(enclosing: EnvironmentRef) -> Self {
        Self {
            enclosing: Some(enclosing),
            values: HashMap::new(),
        }
    }

    pub fn new_enclosed_ref(enclosing: EnvironmentRef) -> EnvironmentRef {
        Rc::new(RefCell::new(Self::from_enclosing(enclosing)))
    }

    // Bind a value to a name in the current environment.
    pub fn define(&mut self, name: String, value: Value) {
        // TODO(perf): Environment keys currently clone each variable name into
        // an owned `String`. String interning or symbol IDs would avoid
        // repeating that allocation across declarations and lookups.
        self.values.insert(name, value);
    }

    // Update the value stored for an existing variable.
    pub fn assign(&mut self, name: &Token, value: Value) -> Result<(), RuntimeError> {
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
    pub fn get(&self, name: &Token) -> Result<Value, RuntimeError> {
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
}
