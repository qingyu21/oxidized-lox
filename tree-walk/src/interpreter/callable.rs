use std::{
    fmt,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    environment::{Environment, EnvironmentRef},
    expr::ExprArenaRef,
    runtime::{LoxCallable, LoxInstance, RuntimeError, Value},
    stmt::Stmt,
    token::{Token, TokenType},
};

use super::{ControlFlow, Interpreter};

#[derive(Debug)]
struct ClockFunction;

#[derive(Debug)]
struct LoxFunctionCode {
    name: Token,
    params: Vec<Token>,
    body: Vec<Stmt>,
    // Keeps the shared expression arena alive so this function body can keep
    // resolving nested child-expression handles after the original parse
    // result is dropped.
    expr_arena: ExprArenaRef,
    is_initializer: bool,
}

// Runtime wrapper around a parsed function declaration. Keeping callable
// behavior here prevents the front-end AST from taking on interpreter duties.
pub(crate) struct LoxFunction {
    code: Rc<LoxFunctionCode>,
    closure: EnvironmentRef,
}

impl LoxFunctionCode {
    fn new(
        name: Token,
        params: Vec<Token>,
        body: Vec<Stmt>,
        expr_arena: ExprArenaRef,
        is_initializer: bool,
    ) -> Self {
        Self {
            name,
            params,
            body,
            expr_arena,
            is_initializer,
        }
    }
}

pub(super) fn install_native_globals(globals: &EnvironmentRef) {
    globals
        .borrow_mut()
        .define("clock".to_string(), Value::Callable(Rc::new(ClockFunction)));
}

pub(super) fn make_function(
    expr_arena: ExprArenaRef,
    name: &Token,
    params: &[Token],
    body: &[Stmt],
    closure: EnvironmentRef,
) -> Value {
    Value::Callable(make_function_ref(
        expr_arena, name, params, body, closure, false,
    ))
}

pub(super) fn make_function_ref(
    expr_arena: ExprArenaRef,
    name: &Token,
    params: &[Token],
    body: &[Stmt],
    closure: EnvironmentRef,
    is_initializer: bool,
) -> Rc<LoxFunction> {
    let code = Rc::new(LoxFunctionCode::new(
        name.clone(),
        params.to_vec(),
        body.to_vec(),
        expr_arena,
        is_initializer,
    ));

    Rc::new(LoxFunction::new(code, closure))
}

impl LoxFunction {
    fn new(code: Rc<LoxFunctionCode>, closure: EnvironmentRef) -> Self {
        Self { code, closure }
    }

    pub(crate) fn bind(&self, instance: Rc<LoxInstance>) -> Rc<LoxFunction> {
        let environment = Environment::new_enclosed_ref(self.closure.clone());
        environment
            .borrow_mut()
            .define("this".to_string(), Value::Instance(instance));

        Rc::new(LoxFunction::new(self.code.clone(), environment))
    }

    fn bound_this(&self) -> Value {
        let keyword = Token::new(
            TokenType::This,
            "this".to_string(),
            None,
            self.code.name.line,
        );
        Environment::get_at(&self.closure, 0, 0, &keyword)
            .expect("initializer methods should always have a bound 'this'")
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
        self.code.params.len()
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
        for (param, argument) in self.code.params.iter().zip(arguments) {
            environment
                .borrow_mut()
                .define(param.lexeme.to_rc(), argument);
        }

        // Run the function body in that call environment. An explicit
        // `return` carries its value back out through the control-flow signal;
        // falling off the end of the body still produces `nil`.
        //
        // Initializers are the one exception: whether they fall off the end or
        // execute `return;`, the call still evaluates to the bound instance
        // (`this`). The resolver rejects `return value;` in init methods, and
        // this runtime branch preserves the same rule as a final backstop.
        match interpreter.execute_block(
            &self.code.body,
            self.code.expr_arena.as_ref(),
            environment,
        )? {
            ControlFlow::None => {
                if self.code.is_initializer {
                    Ok(self.bound_this())
                } else {
                    Ok(Value::Nil)
                }
            }
            ControlFlow::Return(value) => {
                if self.code.is_initializer {
                    Ok(self.bound_this())
                } else {
                    Ok(value)
                }
            }
            ControlFlow::Break => {
                unreachable!("parser should reject break statements that escape a function body");
            }
        }
    }
}

impl fmt::Debug for LoxFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoxFunction")
            .field("name", &self.code.name.lexeme)
            .field("arity", &self.code.params.len())
            .finish_non_exhaustive()
    }
}

impl fmt::Display for LoxFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<fn {}>", self.code.name.lexeme)
    }
}

#[cfg(test)]
mod tests;
