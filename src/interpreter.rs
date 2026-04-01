use std::{cell::RefCell, fmt};

use crate::environment::{Environment, EnvironmentRef};
use crate::expr::Expr;
use crate::lox;
use crate::stmt::Stmt;
use crate::token::{Literal, Token, TokenType};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Nil,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeError {
    token: Token,
    message: String,
}

pub struct Interpreter {
    environment: RefCell<EnvironmentRef>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            environment: RefCell::new(Environment::new_ref()),
        }
    }

    pub fn interpret(&self, statements: &[Stmt]) {
        if let Err(error) = self.execute_all(statements) {
            lox::runtime_error(&error.token, &error.message);
        }
    }

    pub fn interpret_expression(&self, expr: &Expr) {
        match self.evaluate(expr) {
            Ok(value) => println!("{value}"),
            Err(error) => lox::runtime_error(&error.token, &error.message),
        }
    }

    fn execute_all(&self, statements: &[Stmt]) -> Result<(), RuntimeError> {
        for stmt in statements {
            self.execute(stmt)?;
        }

        Ok(())
    }

    fn execute(&self, stmt: &Stmt) -> Result<(), RuntimeError> {
        match stmt {
            Stmt::Block { statements } => {
                let block_environment = Environment::new_enclosed_ref(self.current_environment());
                self.execute_block(statements, block_environment)
            }
            Stmt::Expression { expression } => {
                self.evaluate(expression)?;
                Ok(())
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => self.execute_if(condition, then_branch, else_branch.as_deref()),
            Stmt::Print { expression } => {
                let value = self.evaluate(expression)?;
                println!("{value}");
                Ok(())
            }
            Stmt::Var { name, initializer } => {
                let value = match initializer {
                    Some(initializer) => self.evaluate(initializer)?,
                    None => Value::Nil,
                };
                self.current_environment()
                    .borrow_mut()
                    .define(name.lexeme.clone(), value);
                Ok(())
            }
            Stmt::While { condition, body } => self.execute_while(condition, body),
        }
    }

    fn execute_block(
        &self,
        statements: &[Stmt],
        environment: EnvironmentRef,
    ) -> Result<(), RuntimeError> {
        // TODO(robustness): Use a guard so the previous environment is
        // restored even if block execution panics.
        let previous = self.environment.replace(environment);
        let result = self.execute_all(statements);
        self.environment.replace(previous);
        result
    }

    fn execute_if(
        &self,
        condition: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
    ) -> Result<(), RuntimeError> {
        if Self::is_truthy(&self.evaluate(condition)?) {
            self.execute(then_branch)
        } else if let Some(else_branch) = else_branch {
            self.execute(else_branch)
        } else {
            Ok(())
        }
    }

    fn execute_while(&self, condition: &Expr, body: &Stmt) -> Result<(), RuntimeError> {
        while Self::is_truthy(&self.evaluate(condition)?) {
            self.execute(body)?;
        }

        Ok(())
    }

    fn current_environment(&self) -> EnvironmentRef {
        self.environment.borrow().clone()
    }

    fn evaluate(&self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Assign { name, value } => self.evaluate_assign(name, value),
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
        }
    }
}

#[cfg(test)]
mod tests;
