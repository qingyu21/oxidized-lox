use std::collections::HashMap;

use crate::interpreter::{RuntimeError, Value};
use crate::token::Token;

#[derive(Default)]
pub struct Environment {
    values: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Self::default()
    }

    // Bind a value to a name in the current environment.
    pub fn define(&mut self, name: String, value: Value) {
        // TODO(perf): Environment keys currently clone each variable name into
        // an owned `String`. String interning or symbol IDs would avoid
        // repeating that allocation across declarations and lookups.
        self.values.insert(name, value);
    }

    // Look up the current value stored for a variable name.
    pub fn get(&self, name: &Token) -> Result<Value, RuntimeError> {
        self.values
            .get(&name.lexeme)
            // TODO(perf): Returning an owned `Value` clones strings and would
            // also clone any future heap-backed objects. Shared runtime
            // handles would make variable reads cheaper.
            .cloned()
            .ok_or_else(|| {
                RuntimeError::new(
                    name.clone(),
                    format!("Undefined variable '{}'.", name.lexeme),
                )
            })
    }
}
