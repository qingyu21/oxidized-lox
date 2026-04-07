mod callable;
mod evaluate;
mod execute;

use std::{cell::RefCell, collections::HashMap};

use crate::{
    environment::{Environment, EnvironmentRef},
    expr::Expr,
    runtime::{RuntimeError, Value},
    stmt::Stmt,
    token::Token,
};

pub(crate) use self::callable::LoxFunction;
use self::callable::install_native_globals;

#[derive(Debug, Clone)]
enum ControlFlow {
    None,
    Break,
    Return(Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedBinding {
    Local(usize),
    Global,
    Unresolved,
}

pub(crate) struct Interpreter {
    // Fixed handle to the outermost global scope so resolved global lookups
    // do not depend on whatever the current environment happens to be.
    globals: EnvironmentRef,
    environment: RefCell<EnvironmentRef>,
    // Maps a variable-use token id to the lexical distance computed by the
    // resolver, or records that the name was resolved as global.
    // TODO(ch11-challenge4): This still stores only scope distance. The
    // Chapter 11 challenge to assign per-scope local indexes and access locals
    // by slot instead of name has not been implemented in this interpreter.
    // TODO(memory): The REPL keeps a single Interpreter alive for the whole
    // process, but this cache is never cleared between runs. Repeated REPL
    // inputs therefore grow the map monotonically even after old ASTs and
    // tokens are otherwise unreachable.
    // TODO(ch13-challenge3): No extra self-chosen language feature from
    // Chapter 13 challenge 3 has been implemented yet. Any such feature will
    // likely require coordinated parser, resolver, runtime, and test updates.
    locals: RefCell<HashMap<u64, ResolvedBinding>>,
}

impl Interpreter {
    pub(crate) fn new() -> Self {
        let globals = Environment::new_ref();
        install_native_globals(&globals);

        Self {
            globals: globals.clone(),
            environment: RefCell::new(globals),
            locals: RefCell::new(HashMap::new()),
        }
    }

    // Execute a parsed statement list and return the first runtime error, if any.
    // The outer driver decides how to surface that error to users.
    pub(crate) fn interpret(&self, statements: &[Stmt]) -> Result<(), RuntimeError> {
        match self.execute_all(statements) {
            Ok(ControlFlow::None) => Ok(()),
            Ok(ControlFlow::Break) => {
                unreachable!("parser should reject break statements outside loops");
            }
            Ok(ControlFlow::Return(_)) => {
                unreachable!("parser should reject return statements outside functions");
            }
            Err(error) => Err(error),
        }
    }

    // Evaluate a single expression and return its runtime value to the caller.
    pub(crate) fn interpret_expression(&self, expr: &Expr) -> Result<Value, RuntimeError> {
        self.evaluate(expr)
    }

    // Record the resolver's binding decision for a variable-use token so
    // runtime lookup can jump straight to the right environment.
    pub(crate) fn resolve(&self, name: &Token, binding: ResolvedBinding) {
        self.locals.borrow_mut().insert(name.id, binding);
    }

    // Clone the shared environment handle so nested execution helpers can
    // borrow it independently without keeping the RefCell borrow alive.
    fn current_environment(&self) -> EnvironmentRef {
        self.environment.borrow().clone()
    }

    // Look up the resolver's cached decision for this variable use. A missing
    // entry means resolution never recorded a binding for that token id.
    fn resolved_binding(&self, name: &Token) -> ResolvedBinding {
        self.locals
            .borrow()
            .get(&name.id)
            .copied()
            .unwrap_or(ResolvedBinding::Unresolved)
    }
}

#[cfg(test)]
mod tests;
