mod callable;
mod evaluate;
mod execute;

use std::{cell::RefCell, collections::HashMap};

use crate::{
    environment::{Environment, EnvironmentRef},
    expr::Expr,
    lox,
    stmt::Stmt,
    token::Token,
};

pub(crate) use self::callable::LoxFunction;
use self::callable::install_native_globals;

#[derive(Debug, Clone)]
enum ControlFlow {
    None,
    Break,
    Return(crate::runtime::Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedBinding {
    Local(usize),
    Global,
    Unresolved,
}

pub struct Interpreter {
    // Fixed handle to the outermost global scope so resolved global lookups
    // do not depend on whatever the current environment happens to be.
    globals: EnvironmentRef,
    environment: RefCell<EnvironmentRef>,
    // Maps a variable-use token id to the lexical distance computed by the
    // resolver, or records that the name was resolved as global.
    // TODO(ch11-challenge4): This still stores only scope distance. The
    // Chapter 11 challenge to assign per-scope local indexes and access locals
    // by slot instead of name has not been implemented in this interpreter.
    // TODO(ch13-challenge3): No extra self-chosen language feature from
    // Chapter 13 challenge 3 has been implemented yet. Any such feature will
    // likely require coordinated parser, resolver, runtime, and test updates.
    locals: RefCell<HashMap<u64, ResolvedBinding>>,
}

impl Interpreter {
    pub fn new() -> Self {
        let globals = Environment::new_ref();
        install_native_globals(&globals);

        Self {
            globals: globals.clone(),
            environment: RefCell::new(globals),
            locals: RefCell::new(HashMap::new()),
        }
    }

    pub fn interpret(&self, statements: &[Stmt]) {
        match self.execute_all(statements) {
            Ok(ControlFlow::None) => {}
            Ok(ControlFlow::Break) => {
                unreachable!("parser should reject break statements outside loops");
            }
            Ok(ControlFlow::Return(_)) => {
                unreachable!("parser should reject return statements outside functions");
            }
            Err(error) => lox::runtime_error(&error.token, &error.message),
        }
    }

    pub fn interpret_expression(&self, expr: &Expr) {
        match self.evaluate(expr) {
            Ok(value) => println!("{value}"),
            Err(error) => lox::runtime_error(&error.token, &error.message),
        }
    }

    // Record the resolver's binding decision for a variable-use token so
    // runtime lookup can jump straight to the right environment.
    pub(crate) fn resolve(&self, name: &Token, binding: ResolvedBinding) {
        self.locals.borrow_mut().insert(name.id, binding);
    }

    fn current_environment(&self) -> EnvironmentRef {
        self.environment.borrow().clone()
    }

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
