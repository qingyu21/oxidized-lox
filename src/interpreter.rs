use std::{cell::RefCell, collections::HashMap, fmt};

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
struct RuntimeError {
    token: Token,
    message: String,
}

#[derive(Default)]
pub struct Interpreter {
    environment: RefCell<HashMap<String, Value>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn interpret(&self, statements: &[Stmt]) {
        if let Err(error) = self.execute_all(statements) {
            lox::runtime_error(&error.token, &error.message);
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
            Stmt::Expression { expression } => {
                self.evaluate(expression)?;
                Ok(())
            }
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
                self.define(name, value);
                Ok(())
            }
        }
    }

    fn evaluate(&self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            // TODO(perf): Cloning string literals here allocates a fresh
            // runtime string. A shared string representation could avoid
            // copying literal text into `Value`.
            Expr::Literal { value } => Ok(value.clone().into()),
            Expr::Variable { name } => self.lookup_variable(name),
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

    // Bind a value to a name in the current environment.
    fn define(&self, name: &Token, value: Value) {
        // TODO(perf): Environment keys currently clone each variable name into
        // an owned `String`. String interning or symbol IDs would avoid
        // repeating that allocation across declarations and lookups.
        self.environment
            .borrow_mut()
            .insert(name.lexeme.clone(), value);
    }

    // Look up the current value stored for a variable name.
    fn lookup_variable(&self, name: &Token) -> Result<Value, RuntimeError> {
        self.environment
            .borrow()
            .get(&name.lexeme)
            // TODO(perf): Returning an owned `Value` clones strings and would
            // also clone any future heap-backed objects. Shared runtime
            // handles would make variable reads cheaper.
            .cloned()
            .ok_or_else(|| {
                RuntimeError::new(
                    name.clone(),
                    format!("Undefined variable '{}'.", name.lexeme),
                )
            })
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
    fn new(token: Token, message: impl Into<String>) -> Self {
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
mod tests {
    use super::{Interpreter, Value};
    use crate::expr::Expr;
    use crate::parser::Parser;
    use crate::scanner::Scanner;
    use crate::stmt::Stmt;
    use crate::token::{Literal, Token, TokenType};

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

    #[test]
    fn conditional_skips_unselected_else_branch_errors() {
        assert_eq!(interpret("true ? 1 : 1 / 0"), Value::Number(1.0));
    }

    #[test]
    fn conditional_skips_unselected_then_branch_errors() {
        assert_eq!(interpret("false ? 1 / 0 : 2"), Value::Number(2.0));
    }

    #[test]
    fn comma_still_evaluates_left_operand() {
        let error = evaluate_result("1 / 0, 2").expect_err("comma should evaluate its left operand");
        assert_eq!(error.message, "Division by zero.");
    }

    #[test]
    fn reports_runtime_error_for_non_numeric_comparison() {
        let error = evaluate_result("\"a\" < \"b\"")
            .expect_err("string comparison should currently be rejected");
        assert_eq!(error.message, "Operands must be numbers.");
    }

    #[test]
    fn reports_runtime_error_for_division_by_zero() {
        let error = evaluate_result("1 / 0").expect_err("division by zero should fail");
        assert_eq!(error.message, "Division by zero.");
    }

    #[test]
    fn executes_var_declaration_and_reads_back_the_value() {
        let statements = parse_statements("var beverage = \"tea\";\nbeverage;");
        let interpreter = Interpreter::new();

        assert!(interpreter.execute(&statements[0]).is_ok());

        let value = match &statements[1] {
            Stmt::Expression { expression } => interpreter
                .evaluate(expression)
                .expect("variable lookup should succeed after declaration"),
            _ => panic!("expected a variable expression statement"),
        };

        assert_eq!(value, Value::String("tea".to_string()));
    }

    #[test]
    fn initializes_variables_to_nil_when_no_initializer_is_present() {
        let statements = parse_statements("var beverage;\nbeverage;");
        let interpreter = Interpreter::new();

        assert!(interpreter.execute(&statements[0]).is_ok());

        let value = match &statements[1] {
            Stmt::Expression { expression } => interpreter
                .evaluate(expression)
                .expect("variable lookup should succeed after declaration"),
            _ => panic!("expected a variable expression statement"),
        };

        assert_eq!(value, Value::Nil);
    }

    #[test]
    fn reports_runtime_error_for_undefined_variable_access() {
        let error = evaluate_result("beverage")
            .expect_err("reading an undefined variable should fail at runtime");
        assert_eq!(error.message, "Undefined variable 'beverage'.");
    }

    #[test]
    fn executes_multiple_statements_in_order() {
        let statements = parse_statements("1 + 2;\nprint 3;");
        let interpreter = Interpreter::new();

        assert!(interpreter.execute_all(&statements).is_ok());
    }

    #[test]
    fn stops_executing_after_the_first_runtime_error() {
        let mut statements = parse_statements("1 + 2;\n1 / 0;");
        statements.push(invalid_statement(3));
        let interpreter = Interpreter::new();

        let error = interpreter
            .execute_all(&statements)
            .expect_err("execution should stop at division by zero");

        assert_eq!(error.message, "Division by zero.");
        assert_eq!(error.token.line, 2);
    }

    fn interpret(source: &str) -> Value {
        evaluate_result(source).expect("interpreter should successfully evaluate the test input")
    }

    fn parse_statements(source: &str) -> Vec<Stmt> {
        let tokens = Scanner::new(source).scan_tokens();
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    fn evaluate_result(source: &str) -> Result<Value, super::RuntimeError> {
        let source = format!("{source};");
        let statements = parse_statements(&source);
        let interpreter = Interpreter::new();
        let expr = match statements.as_slice() {
            [Stmt::Expression { expression }] => expression,
            _ => panic!("expected a single expression statement"),
        };

        interpreter.evaluate(expr)
    }

    fn invalid_statement(line: u32) -> Stmt {
        Stmt::expression(Expr::Binary {
            left: Box::new(Expr::literal(Literal::Number(1.0))),
            operator: Token::new(TokenType::Print, "print".to_string(), None, line),
            right: Box::new(Expr::literal(Literal::Number(2.0))),
        })
    }
}
