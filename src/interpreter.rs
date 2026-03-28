use std::fmt;

use crate::expr::Expr;
use crate::lox;
use crate::token::{Literal, Token, TokenType};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Nil,
}

#[derive(Debug, Clone)]
struct RuntimeError {
    token: Token,
    message: String,
}

pub struct Interpreter;

impl Interpreter {
    pub fn interpret(&self, expr: &Expr) -> Option<Value> {
        match self.evaluate(expr) {
            Ok(value) => Some(value),
            Err(error) => {
                lox::runtime_error(&error.token, &error.message);
                None
            }
        }
    }

    fn evaluate(&self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            // TODO(perf): Cloning string literals here allocates a fresh
            // runtime string. A shared string representation could avoid
            // copying literal text into `Value`.
            Expr::Literal { value } => Ok(value.clone().into()),
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
            TokenType::Slash => {
                Self::apply_numeric_binary(operator, &left, &right, |left, right| left / right)
            }
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
    fn new(token: Token, message: &str) -> Self {
        Self {
            token,
            message: message.to_string(),
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
mod tests {
    use super::{Interpreter, Value};
    use crate::parser::Parser;
    use crate::scanner::Scanner;

    #[test]
    fn evaluates_numeric_expression() {
        assert_eq!(interpret("1 + 2 * 3"), Value::Number(7.0));
    }

    #[test]
    fn concatenates_strings_with_plus() {
        assert_eq!(
            interpret("\"lox\" + \"!\""),
            Value::String("lox!".to_string())
        );
    }

    #[test]
    fn concatenates_string_and_number_with_plus() {
        assert_eq!(
            interpret("\"scone\" + 4"),
            Value::String("scone4".to_string())
        );
    }

    #[test]
    fn concatenates_number_and_string_with_plus() {
        assert_eq!(
            interpret("4 + \"scone\""),
            Value::String("4scone".to_string())
        );
    }

    #[test]
    fn evaluates_truthiness_for_bang() {
        assert_eq!(interpret("!nil"), Value::Bool(true));
        assert_eq!(interpret("!0"), Value::Bool(false));
    }

    #[test]
    fn evaluates_equality() {
        assert_eq!(interpret("1 == 1"), Value::Bool(true));
        assert_eq!(interpret("nil != false"), Value::Bool(true));
    }

    #[test]
    fn evaluates_conditional_expression() {
        assert_eq!(interpret("false ? 1 : 2"), Value::Number(2.0));
    }

    #[test]
    fn evaluates_comma_expression() {
        assert_eq!(interpret("1, 2 + 3"), Value::Number(5.0));
    }

    fn interpret(source: &str) -> Value {
        let tokens = Scanner::new(source).scan_tokens();
        let mut parser = Parser::new(tokens);
        let expr = parser
            .parse()
            .expect("parser should successfully parse the test input");

        Interpreter
            .evaluate(&expr)
            .expect("interpreter should successfully evaluate the test input")
    }
}
