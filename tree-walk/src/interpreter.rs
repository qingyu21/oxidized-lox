mod callable;
mod evaluate;
mod execute;

use std::{cell::RefCell, collections::HashMap};

use crate::{
    environment::{Environment, EnvironmentRef},
    expr::{Expr, ExprArena},
    parser::{ParsedExpression, ParsedProgram},
    runtime::{RuntimeError, Value},
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
    Local { distance: usize, slot: usize },
    Global,
    Unresolved,
}

pub(crate) struct Interpreter {
    // Fixed handle to the outermost global scope so resolved global lookups
    // do not depend on whatever the current environment happens to be.
    globals: EnvironmentRef,
    environment: RefCell<EnvironmentRef>,
    // Maps a variable-use token id to the lexical binding chosen by the
    // resolver so local reads and writes can jump straight to an environment
    // slot instead of re-hashing the variable name at runtime.
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
    pub(crate) fn interpret(&self, program: &ParsedProgram) -> Result<(), RuntimeError> {
        match self.execute_all(program.as_slice(), program.expr_arena()) {
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
    pub(crate) fn interpret_expression(
        &self,
        expr: &ParsedExpression,
    ) -> Result<Value, RuntimeError> {
        self.evaluate(expr.as_expr(), expr.expr_arena())
    }

    // Record the resolver's binding decision for a variable-use token so
    // runtime lookup can jump straight to the right environment.
    pub(crate) fn resolve(&self, name: &Token, binding: ResolvedBinding) {
        self.locals.borrow_mut().insert(name.id, binding);
    }

    // Each top-level driver pass repopulates this cache from scratch for the
    // current AST, so old resolver entries can be dropped once execution ends.
    pub(crate) fn clear_resolved_bindings(&self) {
        self.locals.borrow_mut().clear();
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

    fn expr<'a>(&self, expr_arena: &'a ExprArena, expr_ref: crate::expr::ExprRef) -> &'a Expr {
        expr_arena.get(expr_ref)
    }
}

#[cfg(test)]
mod tests;
