use std::{
    cell::RefCell,
    fmt,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::environment::{Environment, EnvironmentRef};
use crate::expr::Expr;
use crate::lox;
use crate::stmt::Stmt;
use crate::token::{Literal, Token, TokenType};

pub(crate) trait LoxCallable: fmt::Debug + fmt::Display {
    fn arity(&self) -> usize;
    fn call(&self, interpreter: &Interpreter, arguments: Vec<Value>)
    -> Result<Value, RuntimeError>;
}

// TODO(module-layout): Once more native functions or callable runtime types
// exist, move `LoxCallable` and builtin implementations like `ClockFunction`
// into dedicated callable/native modules.
#[derive(Debug)]
struct ClockFunction;

struct LoxFunction {
    name: Token,
    params: Vec<Token>,
    body: Vec<Stmt>,
    closure: EnvironmentRef,
}

// TODO(module-layout): `Value` is already shared across the interpreter and
// environment. As functions, classes, and instances grow the runtime object
// model, move it and its trait impls into a dedicated value/runtime module.
#[derive(Debug, Clone)]
pub(crate) enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Nil,
    Callable(Rc<dyn LoxCallable>),
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeError {
    token: Token,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlFlow {
    None,
    Break,
}

pub struct Interpreter {
    environment: RefCell<EnvironmentRef>,
}

impl Interpreter {
    // TODO(module-layout): Split execution, evaluation, callable, and class
    // runtime support into submodules once Chapters 10-13 add those domains.
    pub fn new() -> Self {
        let globals = Environment::new_ref();
        // TODO(module-layout): If Lox grows more native globals, extract
        // builtin registration into a dedicated helper/module instead of
        // expanding `Interpreter::new()`.
        globals
            .borrow_mut()
            .define("clock".to_string(), Value::Callable(Rc::new(ClockFunction)));

        Self {
            environment: RefCell::new(globals),
        }
    }

    pub fn interpret(&self, statements: &[Stmt]) {
        match self.execute_all(statements) {
            Ok(ControlFlow::None) => {}
            Ok(ControlFlow::Break) => {
                unreachable!("parser should reject break statements outside loops");
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

    fn execute_all(&self, statements: &[Stmt]) -> Result<ControlFlow, RuntimeError> {
        for stmt in statements {
            match self.execute(stmt)? {
                ControlFlow::None => {}
                ControlFlow::Break => return Ok(ControlFlow::Break),
            }
        }

        Ok(ControlFlow::None)
    }

    fn execute(&self, stmt: &Stmt) -> Result<ControlFlow, RuntimeError> {
        match stmt {
            Stmt::Block { statements } => {
                let block_environment = Environment::new_enclosed_ref(self.current_environment());
                self.execute_block(statements, block_environment)
            }
            Stmt::Break => Ok(ControlFlow::Break),
            Stmt::Expression { expression } => {
                self.evaluate(expression)?;
                Ok(ControlFlow::None)
            }
            Stmt::Function { name, params, body } => {
                self.execute_function_declaration(name, params, body)
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => self.execute_if(condition, then_branch, else_branch.as_deref()),
            Stmt::Print { expression } => {
                let value = self.evaluate(expression)?;
                println!("{value}");
                Ok(ControlFlow::None)
            }
            Stmt::Var { name, initializer } => {
                let value = match initializer {
                    Some(initializer) => self.evaluate(initializer)?,
                    None => Value::Nil,
                };
                self.current_environment()
                    .borrow_mut()
                    .define(name.lexeme.clone(), value);
                Ok(ControlFlow::None)
            }
            Stmt::While { condition, body } => self.execute_while(condition, body),
        }
    }

    fn execute_block(
        &self,
        statements: &[Stmt],
        environment: EnvironmentRef,
    ) -> Result<ControlFlow, RuntimeError> {
        // TODO(robustness): Use a guard so the previous environment is
        // restored even if block execution panics.
        let previous = self.environment.replace(environment);
        let result = self.execute_all(statements);
        self.environment.replace(previous);
        result
    }

    fn execute_function_declaration(
        &self,
        name: &Token,
        params: &[Token],
        body: &[Stmt],
    ) -> Result<ControlFlow, RuntimeError> {
        // Function declarations are executable statements: evaluating one
        // creates a callable runtime value and binds it in the current scope.
        let function = LoxFunction::new(
            name.clone(),
            params.to_vec(),
            body.to_vec(),
            self.current_environment(),
        );
        self.current_environment()
            .borrow_mut()
            .define(name.lexeme.clone(), Value::Callable(Rc::new(function)));
        Ok(ControlFlow::None)
    }

    fn execute_if(
        &self,
        condition: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
    ) -> Result<ControlFlow, RuntimeError> {
        if Self::is_truthy(&self.evaluate(condition)?) {
            self.execute(then_branch)
        } else if let Some(else_branch) = else_branch {
            self.execute(else_branch)
        } else {
            Ok(ControlFlow::None)
        }
    }

    fn execute_while(&self, condition: &Expr, body: &Stmt) -> Result<ControlFlow, RuntimeError> {
        while Self::is_truthy(&self.evaluate(condition)?) {
            match self.execute(body)? {
                ControlFlow::None => {}
                ControlFlow::Break => break,
            }
        }

        Ok(ControlFlow::None)
    }

    fn current_environment(&self) -> EnvironmentRef {
        self.environment.borrow().clone()
    }

    fn evaluate(&self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Assign { name, value } => self.evaluate_assign(name, value),
            Expr::Call {
                callee,
                paren,
                arguments,
            } => self.evaluate_call(callee, paren, arguments),
            // TODO(perf): Cloning string literals here allocates a fresh
            // runtime string. A shared string representation could avoid
            // copying literal text into `Value`.
            Expr::Literal { value } => Ok(value.clone().into()),
            Expr::Logical {
                left,
                operator,
                right,
            } => self.evaluate_logical(left, operator, right),
            Expr::Variable { name } => self.current_environment().borrow().get(name),
            Expr::Grouping { expression } => self.evaluate(expression),
            Expr::Conditional {
                condition,
                then_branch,
                else_branch,
            } => self.evaluate_conditional(condition, then_branch, else_branch),
            Expr::Unary { operator, right } => self.evaluate_unary(operator, right),
            Expr::Binary {
                left,
                operator,
                right,
            } => self.evaluate_binary(left, operator, right),
        }
    }

    fn evaluate_assign(&self, name: &Token, value_expr: &Expr) -> Result<Value, RuntimeError> {
        let value = self.evaluate(value_expr)?;
        self.current_environment()
            .borrow_mut()
            .assign(name, value.clone())?;
        Ok(value)
    }

    fn evaluate_call(
        &self,
        callee_expr: &Expr,
        paren: &Token,
        argument_exprs: &[Expr],
    ) -> Result<Value, RuntimeError> {
        // Evaluate the callee expression first. This may be a simple variable
        // lookup like `clock`, but the grammar allows any higher-precedence
        // expression to appear before the call parentheses.
        let callee = self.evaluate(callee_expr)?;

        // Lox evaluates call arguments from left to right before dispatching
        // to the callee.
        let mut arguments = Vec::with_capacity(argument_exprs.len());
        for argument_expr in argument_exprs {
            arguments.push(self.evaluate(argument_expr)?);
        }

        // Convert the runtime value into the callable interface or report a
        // user-facing runtime error instead of crashing on a host-language
        // type mismatch.
        let callable = match callee {
            Value::Callable(callable) => callable,
            _ => {
                return Err(RuntimeError::new(
                    paren.clone(),
                    "Can only call functions and classes.",
                ));
            }
        };

        // Enforce arity in one shared place so every callable kind gets the
        // same argument-count validation.
        if arguments.len() != callable.arity() {
            return Err(RuntimeError::new(
                paren.clone(),
                format!(
                    "Expected {} arguments but got {}.",
                    callable.arity(),
                    arguments.len()
                ),
            ));
        }

        // Hand off to the concrete callable implementation.
        callable.call(self, arguments)
    }

    fn evaluate_logical(
        &self,
        left_expr: &Expr,
        operator: &Token,
        right_expr: &Expr,
    ) -> Result<Value, RuntimeError> {
        let left = self.evaluate(left_expr)?;

        match operator.type_ {
            TokenType::Or if Self::is_truthy(&left) => Ok(left),
            TokenType::And if !Self::is_truthy(&left) => Ok(left),
            TokenType::Or | TokenType::And => self.evaluate(right_expr),
            _ => unreachable!("parser should only build valid logical operators"),
        }
    }

    fn evaluate_conditional(
        &self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: &Expr,
    ) -> Result<Value, RuntimeError> {
        if Self::is_truthy(&self.evaluate(condition)?) {
            self.evaluate(then_branch)
        } else {
            self.evaluate(else_branch)
        }
    }

    fn evaluate_unary(&self, operator: &Token, right_expr: &Expr) -> Result<Value, RuntimeError> {
        let right = self.evaluate(right_expr)?;

        match operator.type_ {
            TokenType::Bang => Ok(Value::Bool(!Self::is_truthy(&right))),
            TokenType::Minus => Self::apply_negate(operator, &right),
            _ => unreachable!("parser should only build valid unary operators"),
        }
    }

    fn evaluate_binary(
        &self,
        left_expr: &Expr,
        operator: &Token,
        right_expr: &Expr,
    ) -> Result<Value, RuntimeError> {
        let left = self.evaluate(left_expr)?;
        let right = self.evaluate(right_expr)?;

        match operator.type_ {
            TokenType::Comma => Ok(right),
            TokenType::BangEqual => Ok(Value::Bool(!Self::is_equal(&left, &right))),
            TokenType::EqualEqual => Ok(Value::Bool(Self::is_equal(&left, &right))),
            TokenType::Greater => {
                Self::apply_numeric_comparison(operator, &left, &right, |left, right| left > right)
            }
            TokenType::GreaterEqual => {
                Self::apply_numeric_comparison(operator, &left, &right, |left, right| left >= right)
            }
            TokenType::Less => {
                Self::apply_numeric_comparison(operator, &left, &right, |left, right| left < right)
            }
            TokenType::LessEqual => {
                Self::apply_numeric_comparison(operator, &left, &right, |left, right| left <= right)
            }
            TokenType::Minus => {
                Self::apply_numeric_binary(operator, &left, &right, |left, right| left - right)
            }
            TokenType::Plus => Self::apply_plus(operator, &left, &right),
            TokenType::Slash => Self::apply_divide(operator, &left, &right),
            TokenType::Star => {
                Self::apply_numeric_binary(operator, &left, &right, |left, right| left * right)
            }
            _ => unreachable!("parser should only build valid binary operators"),
        }
    }

    fn apply_negate(operator: &Token, value: &Value) -> Result<Value, RuntimeError> {
        Ok(Value::Number(-Self::expect_number_operand(
            operator, value,
        )?))
    }

    fn apply_plus(operator: &Token, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Number(left), Value::Number(right)) => Ok(Value::Number(left + right)),
            (Value::String(_), _) | (_, Value::String(_)) => {
                // TODO(perf): Repeated `+` concatenation allocates and copies
                // into a new `String` each time. A rope or builder-style
                // runtime string could reduce churn.
                Ok(Value::String(format!("{left}{right}")))
            }
            _ => Err(RuntimeError::new(
                operator.clone(),
                "Operands must be two numbers or at least one string.",
            )),
        }
    }

    // Divide two numeric operands and reject division by zero.
    fn apply_divide(operator: &Token, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        let (left, right) = Self::expect_number_operands(operator, left, right)?;

        if right == 0.0 {
            return Err(RuntimeError::new(operator.clone(), "Division by zero."));
        }

        Ok(Value::Number(left / right))
    }

    // Require numeric operands, then apply a numeric binary operator.
    fn apply_numeric_binary<F>(
        operator: &Token,
        left: &Value,
        right: &Value,
        operation: F,
    ) -> Result<Value, RuntimeError>
    where
        F: FnOnce(f64, f64) -> f64,
    {
        let (left, right) = Self::expect_number_operands(operator, left, right)?;
        Ok(Value::Number(operation(left, right)))
    }

    // Require numeric operands, then apply a comparison that yields a boolean.
    fn apply_numeric_comparison<F>(
        operator: &Token,
        left: &Value,
        right: &Value,
        operation: F,
    ) -> Result<Value, RuntimeError>
    where
        F: FnOnce(f64, f64) -> bool,
    {
        let (left, right) = Self::expect_number_operands(operator, left, right)?;
        Ok(Value::Bool(operation(left, right)))
    }

    // In Lox, only `false` and `nil` are falsey; everything else is truthy.
    fn is_truthy(value: &Value) -> bool {
        match value {
            Value::Nil => false,
            Value::Bool(value) => *value,
            _ => true,
        }
    }

    fn is_equal(left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Nil, Value::Nil) => true,
            (Value::Nil, _) | (_, Value::Nil) => false,
            _ => left == right,
        }
    }

    // Extract a single numeric operand or report a runtime type error.
    fn expect_number_operand(operator: &Token, operand: &Value) -> Result<f64, RuntimeError> {
        if let Value::Number(value) = operand {
            Ok(*value)
        } else {
            Err(RuntimeError::new(
                operator.clone(),
                "Operand must be a number.",
            ))
        }
    }

    // Extract two numeric operands or report a runtime type error.
    fn expect_number_operands(
        operator: &Token,
        left: &Value,
        right: &Value,
    ) -> Result<(f64, f64), RuntimeError> {
        match (left, right) {
            (Value::Number(left), Value::Number(right)) => Ok((*left, *right)),
            _ => Err(RuntimeError::new(
                operator.clone(),
                "Operands must be numbers.",
            )),
        }
    }
}

impl RuntimeError {
    pub(crate) fn new(token: Token, message: impl Into<String>) -> Self {
        Self {
            token,
            message: message.into(),
        }
    }
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

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(left), Value::String(right)) => left == right,
            (Value::Number(left), Value::Number(right)) => left == right,
            (Value::Bool(left), Value::Bool(right)) => left == right,
            (Value::Nil, Value::Nil) => true,
            (Value::Callable(left), Value::Callable(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }
}

impl From<Literal> for Value {
    fn from(value: Literal) -> Self {
        match value {
            Literal::String(value) => Value::String(value),
            Literal::Number(value) => Value::Number(value),
            Literal::Bool(value) => Value::Bool(value),
            Literal::Nil => Value::Nil,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(value) => write!(f, "{value}"),
            Value::Number(value) => write!(f, "{value}"),
            Value::Bool(value) => write!(f, "{value}"),
            Value::Nil => write!(f, "nil"),
            Value::Callable(callable) => write!(f, "{callable}"),
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

        // Run the function body in that call environment. Until `return` is
        // implemented, falling off the end of the body produces `nil`.
        // TODO(ch10): Once `return` statements are implemented, thread return
        // values through call boundaries instead of always producing `nil`.
        match interpreter.execute_block(&self.body, environment)? {
            ControlFlow::None => Ok(Value::Nil),
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

#[cfg(test)]
mod tests;
