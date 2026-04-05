use crate::{
    environment::Environment,
    expr::Expr,
    runtime::{LoxCallable, LoxClass, RuntimeError, Value},
    token::{Token, TokenType},
};

use super::{Interpreter, ResolvedBinding};

impl Interpreter {
    pub(super) fn evaluate(&self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Assign { name, value } => self.evaluate_assign(name, value),
            Expr::Call {
                callee,
                paren,
                arguments,
            } => self.evaluate_call(callee, paren, arguments),
            Expr::Get { object, name } => self.evaluate_get(object, name),
            // TODO(perf): Cloning string literals here allocates a fresh
            // runtime string. A shared string representation could avoid
            // copying literal text into `Value`.
            Expr::Literal { value } => Ok(value.clone().into()),
            Expr::Logical {
                left,
                operator,
                right,
            } => self.evaluate_logical(left, operator, right),
            Expr::Set {
                object,
                name,
                value,
            } => self.evaluate_set(object, name, value),
            Expr::Variable { name } => self.look_up_variable(name),
            Expr::Grouping { expression } => self.evaluate(expression),
            Expr::Conditional {
                condition,
                then_branch,
                else_branch,
            } => self.evaluate_conditional(condition, then_branch, else_branch),
            Expr::This { keyword } => self.look_up_variable(keyword),
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
        match self.resolved_binding(name) {
            ResolvedBinding::Local(distance) => {
                Environment::assign_at(&self.current_environment(), distance, name, value.clone())?
            }
            ResolvedBinding::Global => self.globals.borrow_mut().assign(name, value.clone())?,
            ResolvedBinding::Unresolved => self
                .current_environment()
                .borrow_mut()
                .assign(name, value.clone())?,
        }
        Ok(value)
    }

    fn evaluate_get(&self, object_expr: &Expr, name: &Token) -> Result<Value, RuntimeError> {
        let object = self.evaluate(object_expr)?;

        if let Value::Instance(instance) = object {
            instance.get(name)
        } else {
            Err(RuntimeError::new(
                name.clone(),
                "Only instances have properties.",
            ))
        }
    }

    fn evaluate_set(
        &self,
        object_expr: &Expr,
        name: &Token,
        value_expr: &Expr,
    ) -> Result<Value, RuntimeError> {
        let object = self.evaluate(object_expr)?;

        if let Value::Instance(instance) = object {
            let value = self.evaluate(value_expr)?;
            instance.set(name, value.clone());
            Ok(value)
        } else {
            Err(RuntimeError::new(
                name.clone(),
                "Only instances have fields.",
            ))
        }
    }

    // Read a variable using the resolver's precomputed lexical distance when
    // available, falling back to dynamic lookup only for unresolved tests and
    // legacy call sites that bypass the resolver pass.
    fn look_up_variable(&self, name: &Token) -> Result<Value, RuntimeError> {
        match self.resolved_binding(name) {
            ResolvedBinding::Local(distance) => {
                Environment::get_at(&self.current_environment(), distance, name)
            }
            ResolvedBinding::Global => self.globals.borrow().get(name),
            ResolvedBinding::Unresolved => self.current_environment().borrow().get(name),
        }
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
        match callee {
            Value::Callable(callable) => {
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
            Value::Class(class) => {
                if arguments.len() != class.arity() {
                    return Err(RuntimeError::new(
                        paren.clone(),
                        format!(
                            "Expected {} arguments but got {}.",
                            class.arity(),
                            arguments.len()
                        ),
                    ));
                }

                Ok(LoxClass::instantiate(class))
            }
            _ => Err(RuntimeError::new(
                paren.clone(),
                "Can only call functions and classes.",
            )),
        }
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
    pub(super) fn is_truthy(value: &Value) -> bool {
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
