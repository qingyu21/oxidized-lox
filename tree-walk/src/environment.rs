use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::runtime::{RuntimeError, Value};
use crate::token::Token;

pub(crate) type EnvironmentRef = Rc<RefCell<Environment>>;

#[derive(Default)]
pub(crate) struct Environment {
    enclosing: Option<EnvironmentRef>,
    slots_by_name: HashMap<Rc<str>, usize>,
    values: Vec<Value>,
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
            slots_by_name: HashMap::new(),
            values: Vec::new(),
        }
    }

    pub(crate) fn new_enclosed_ref(enclosing: EnvironmentRef) -> EnvironmentRef {
        Rc::new(RefCell::new(Self::from_enclosing(enclosing)))
    }

    // Bind a value to a name in the current environment.
    //
    // The global environment intentionally allows redefinition so top-level
    // declarations can overwrite earlier bindings, matching Lox's globals.
    // Nested scopes rely on the resolver to reject duplicate declarations
    // before execution reaches this storage layer, so redefining a local here
    // would indicate an internal bug or a caller bypassing that contract.
    pub(crate) fn define(&mut self, name: impl Into<Rc<str>>, value: Value) {
        let name = name.into();
        debug_assert!(
            self.enclosing.is_none() || self.slot_for_name(name.as_ref()).is_none(),
            "nested scope should not redefine '{}'",
            name.as_ref()
        );

        if let Some(slot) = self.slot_for_name(name.as_ref()) {
            self.values[slot] = value;
        } else {
            let slot = self.values.len();
            self.values.push(value);
            self.slots_by_name.insert(name, slot);
        }
    }

    // Update the value stored for an existing variable.
    pub(crate) fn assign(&mut self, name: &Token, value: Value) -> Result<(), RuntimeError> {
        if let Some(slot) = self.slot_for_name(name.lexeme.as_ref()) {
            self.values[slot] = value;
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
        if let Some(slot) = self.slot_for_name(name.lexeme.as_ref()) {
            Ok(self.values[slot].clone())
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
    // precomputed lexical distance and slot index.
    pub(crate) fn assign_at(
        environment: &EnvironmentRef,
        distance: usize,
        slot: usize,
        name: &Token,
        value: Value,
    ) -> Result<(), RuntimeError> {
        let ancestor = Self::ancestor(environment, distance);
        let mut ancestor = ancestor.borrow_mut();

        if let Some(target) = ancestor.values.get_mut(slot) {
            *target = value;
            Ok(())
        } else {
            Err(RuntimeError::new(
                name.clone(),
                format!("Undefined variable '{}'.", name.lexeme),
            ))
        }
    }

    // Read a binding from the ancestor environment selected by the resolver's
    // precomputed lexical distance and slot index.
    pub(crate) fn get_at(
        environment: &EnvironmentRef,
        distance: usize,
        slot: usize,
        name: &Token,
    ) -> Result<Value, RuntimeError> {
        let ancestor = Self::ancestor(environment, distance);
        let ancestor = ancestor.borrow();

        if let Some(value) = ancestor.values.get(slot) {
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

    fn slot_for_name(&self, name: &str) -> Option<usize> {
        self.slots_by_name.get(name).copied()
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

    #[test]
    fn ancestor_slot_access_reads_and_writes_the_expected_binding() {
        let outer = Environment::new_ref();
        outer
            .borrow_mut()
            .define("tea", Value::String("earl grey".into()));
        outer.borrow_mut().define("count", Value::Number(1.0));

        let inner = Environment::new_enclosed_ref(outer.clone());
        let name = Token::new(TokenType::Identifier, "count", None, 1);

        assert_eq!(
            Environment::get_at(&inner, 1, 1, &name)
                .expect("slot lookup should read the outer binding"),
            Value::Number(1.0)
        );

        Environment::assign_at(&inner, 1, 1, &name, Value::Number(2.0))
            .expect("slot update should write the outer binding");

        assert_eq!(
            outer
                .borrow()
                .get(&name)
                .expect("named lookup should see the slot update"),
            Value::Number(2.0)
        );
    }
}
