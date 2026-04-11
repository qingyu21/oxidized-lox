use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::runtime::{RuntimeError, Value};
use crate::token::Token;

pub(crate) type EnvironmentRef = Rc<RefCell<Environment>>;

#[derive(Default)]
pub(crate) struct Environment {
    enclosing: Option<EnvironmentRef>,
    values: HashMap<Rc<str>, Value>,
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
    pub(crate) fn define(&mut self, name: impl Into<Rc<str>>, value: Value) {
        self.values.insert(name.into(), value);
    }

    // Update the value stored for an existing variable.
    pub(crate) fn assign(&mut self, name: &Token, value: Value) -> Result<(), RuntimeError> {
        if let Some(slot) = self.values.get_mut(name.lexeme.as_ref()) {
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
        if let Some(value) = self.values.get(name.lexeme.as_ref()) {
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

        if let Some(slot) = ancestor.values.get_mut(name.lexeme.as_ref()) {
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

        if let Some(value) = ancestor.values.get(name.lexeme.as_ref()) {
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

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::Environment;
    use crate::{
        runtime::Value,
        token::{Token, TokenType},
    };

    #[test]
    fn string_reads_share_the_same_backing_text() {
        let mut environment = Environment::new();
        environment.define("tea", Value::String("earl grey".into()));
        let name = Token::new(TokenType::Identifier, "tea", None, 1);

        let Value::String(first) = environment
            .get(&name)
            .expect("defined variable should be readable")
        else {
            panic!("expected the binding to contain a string");
        };
        let Value::String(second) = environment
            .get(&name)
            .expect("repeated reads should still succeed")
        else {
            panic!("expected the binding to contain a string");
        };

        assert!(
            Rc::ptr_eq(&first, &second),
            "environment reads should clone only the shared string handle"
        );
    }
}
