use std::collections::HashMap;

use crate::{interpreter::Interpreter, token::Token};

mod expr;
mod scope;
mod stmt;

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolveError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingKind {
    Variable,
    Parameter,
    Function,
    Class,
    Super,
    This,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Resolver-only state that tracks whether we are currently walking inside a
// class declaration. This exists to validate `this`, not to model runtime
// class objects.
enum ClassType {
    // We are not currently resolving any class body.
    None,
    // We are resolving a class body, so methods may refer to `this`.
    Class,
    // We are resolving a subclass body, so methods may refer to both `this`
    // and `super`.
    Subclass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FunctionType {
    None,
    Function,
    Method,
    Initializer,
}

#[derive(Debug, Clone)]
struct BindingInfo {
    token: Token,
    kind: BindingKind,
    defined: bool,
    used: bool,
}

pub(crate) struct Resolver<'a> {
    interpreter: &'a Interpreter,
    // Stack of lexical scopes being resolved. Each binding tracks whether it is
    // fully defined yet and whether it was ever read before the scope ended.
    scopes: Vec<HashMap<String, BindingInfo>>,
    // Surrounding class context for the current resolver walk. This lets us
    // reject `this` outside classes and restore outer state for nested classes.
    current_class: ClassType,
    current_function: FunctionType,
}

impl<'a> Resolver<'a> {
    pub(crate) fn new(interpreter: &'a Interpreter) -> Self {
        Self {
            interpreter,
            scopes: Vec::new(),
            current_class: ClassType::None,
            current_function: FunctionType::None,
        }
    }
}
