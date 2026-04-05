use std::{
    fmt,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    environment::{Environment, EnvironmentRef},
    runtime::{LoxCallable, RuntimeError, Value},
    stmt::Stmt,
    token::Token,
};

use super::{ControlFlow, Interpreter};

#[derive(Debug)]
struct ClockFunction;

// Runtime wrapper around a parsed function declaration. Keeping callable
// behavior here prevents the front-end AST from taking on interpreter duties.
pub(crate) struct LoxFunction {
    name: Token,
    params: Vec<Token>,
    body: Vec<Stmt>,
    closure: EnvironmentRef,
}

pub(super) fn install_native_globals(globals: &EnvironmentRef) {
    globals
        .borrow_mut()
        .define("clock".to_string(), Value::Callable(Rc::new(ClockFunction)));
}

pub(super) fn make_function(
    name: &Token,
    params: &[Token],
    body: &[Stmt],
    closure: EnvironmentRef,
) -> Value {
    Value::Callable(make_function_ref(name, params, body, closure))
}

pub(super) fn make_function_ref(
    name: &Token,
    params: &[Token],
    body: &[Stmt],
    closure: EnvironmentRef,
) -> Rc<LoxFunction> {
    Rc::new(LoxFunction::new(
        name.clone(),
        params.to_vec(),
        body.to_vec(),
        closure,
    ))
}

impl LoxFunction {
    fn new(name: Token, params: Vec<Token>, body: Vec<Stmt>, closure: EnvironmentRef) -> Self {
        Self {
            name,
            params,
            body,
            closure,
        }
    }
}

impl LoxCallable for ClockFunction {
    fn arity(&self) -> usize {
        0
    }

    fn call(
        &self,
        _interpreter: &Interpreter,
        _arguments: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_secs_f64();

        Ok(Value::Number(seconds))
    }
}

impl fmt::Display for ClockFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native fn clock>")
    }
}

impl LoxCallable for LoxFunction {
    fn arity(&self) -> usize {
        self.params.len()
    }

    fn call(
        &self,
        interpreter: &Interpreter,
        arguments: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        // Each call gets a fresh local scope enclosed by the environment where
        // the function was declared, which is what preserves lexical scoping.
        let environment = Environment::new_enclosed_ref(self.closure.clone());

        // Bind evaluated argument values to the function's parameter names.
        for (param, argument) in self.params.iter().zip(arguments) {
            environment
                .borrow_mut()
                .define(param.lexeme.clone(), argument);
        }

        // Run the function body in that call environment. An explicit
        // `return` carries its value back out through the control-flow signal;
        // falling off the end of the body still produces `nil`.
        match interpreter.execute_block(&self.body, environment)? {
            ControlFlow::None => Ok(Value::Nil),
            ControlFlow::Return(value) => Ok(value),
            ControlFlow::Break => {
                unreachable!("parser should reject break statements that escape a function body");
            }
        }
    }
}

impl fmt::Debug for LoxFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoxFunction")
            .field("name", &self.name.lexeme)
            .field("arity", &self.params.len())
            .finish_non_exhaustive()
    }
}

impl fmt::Display for LoxFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<fn {}>", self.name.lexeme)
    }
}
