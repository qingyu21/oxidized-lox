use std::rc::Rc;

use crate::{
    environment::Environment,
    expr::{Expr, ExprArena, ExprRef},
    runtime::{LoxClass, RuntimeError, Value},
    token::{Token, TokenType},
};

use super::{Interpreter, ResolvedBinding};

enum PlusValue {
    Number(f64),
    Text(ConcatenatedString),
    Other(Value),
}

struct ConcatenatedString {
    segments: Vec<StringSegment>,
    len: usize,
}

enum StringSegment {
    Shared(Rc<str>),
    Owned(String),
}

impl From<Value> for PlusValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Number(value) => PlusValue::Number(value),
            Value::String(value) => PlusValue::Text(ConcatenatedString::from_shared(value)),
            other => PlusValue::Other(other),
        }
    }
}

impl PlusValue {
    fn into_value(self) -> Value {
        match self {
            PlusValue::Number(value) => Value::Number(value),
            PlusValue::Text(value) => Value::String(value.finish().into()),
            PlusValue::Other(value) => value,
        }
    }
}

impl ConcatenatedString {
    fn from_shared(value: Rc<str>) -> Self {
        let len = value.len();
        Self {
            segments: vec![StringSegment::Shared(value)],
            len,
        }
    }

    fn push_plus_value(&mut self, value: PlusValue) {
        match value {
            PlusValue::Number(value) => self.push_owned(value.to_string()),
            PlusValue::Text(value) => self.append(value),
            PlusValue::Other(value) => self.push_owned(value.to_string()),
        }
    }

    fn prepend_plus_value(&mut self, value: PlusValue) {
        match value {
            PlusValue::Number(value) => self.prepend_owned(value.to_string()),
            PlusValue::Text(value) => self.prepend(value),
            PlusValue::Other(value) => self.prepend_owned(value.to_string()),
        }
    }

    fn append(&mut self, mut other: Self) {
        self.len += other.len;
        self.segments.append(&mut other.segments);
    }

    fn prepend(&mut self, other: Self) {
        self.len += other.len;
        let mut segments = other.segments;
        segments.append(&mut self.segments);
        self.segments = segments;
    }

    fn push_owned(&mut self, value: String) {
        self.len += value.len();
        if !value.is_empty() {
            self.segments.push(StringSegment::Owned(value));
        }
    }

    fn prepend_owned(&mut self, value: String) {
        self.len += value.len();
        if !value.is_empty() {
            self.segments.insert(0, StringSegment::Owned(value));
        }
    }

    fn finish(self) -> String {
        let mut result = String::with_capacity(self.len);
        for segment in self.segments {
            match segment {
                StringSegment::Shared(value) => result.push_str(value.as_ref()),
                StringSegment::Owned(value) => result.push_str(&value),
            }
        }
        result
    }
}

impl Interpreter {
    pub(super) fn evaluate(
        &self,
        expr: &Expr,
        expr_arena: &ExprArena,
    ) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Assign { name, value } => {
                self.evaluate_assign(name, self.expr(expr_arena, *value), expr_arena)
            }
            Expr::Call {
                callee,
                paren,
                arguments,
            } => self.evaluate_call(self.expr(expr_arena, *callee), paren, arguments, expr_arena),
            Expr::Get { object, name } => {
                self.evaluate_get(self.expr(expr_arena, *object), name, expr_arena)
            }
            Expr::Literal { value } => Ok(value.clone().into()),
            Expr::Logical {
                left,
                operator,
                right,
            } => self.evaluate_logical(
                self.expr(expr_arena, *left),
                operator,
                self.expr(expr_arena, *right),
                expr_arena,
            ),
            Expr::Set {
                object,
                name,
                value,
            } => self.evaluate_set(
                self.expr(expr_arena, *object),
                name,
                self.expr(expr_arena, *value),
                expr_arena,
            ),
            Expr::Super { keyword, method } => self.evaluate_super(keyword, method),
            Expr::Variable { name } => self.look_up_variable(name),
            Expr::Grouping { expression } => {
                self.evaluate(self.expr(expr_arena, *expression), expr_arena)
            }
            Expr::Conditional {
                condition,
                then_branch,
                else_branch,
            } => self.evaluate_conditional(
                self.expr(expr_arena, *condition),
                self.expr(expr_arena, *then_branch),
                self.expr(expr_arena, *else_branch),
                expr_arena,
            ),
            Expr::This { keyword } => self.look_up_variable(keyword),
            Expr::Unary { operator, right } => {
                self.evaluate_unary(operator, self.expr(expr_arena, *right), expr_arena)
            }
            Expr::Binary {
                left,
                operator,
                right,
            } => self.evaluate_binary(
                self.expr(expr_arena, *left),
                operator,
                self.expr(expr_arena, *right),
                expr_arena,
            ),
        }
    }

    fn evaluate_assign(
        &self,
        name: &Token,
        value_expr: &Expr,
        expr_arena: &ExprArena,
    ) -> Result<Value, RuntimeError> {
        let value = self.evaluate(value_expr, expr_arena)?;
        match self.resolved_binding(name) {
            ResolvedBinding::Local { distance, slot } => Environment::assign_at(
                &self.current_environment(),
                distance,
                slot,
                name,
                value.clone(),
            )?,
            ResolvedBinding::Global => self.globals.borrow_mut().assign(name, value.clone())?,
            ResolvedBinding::Unresolved => self
                .current_environment()
                .borrow_mut()
                .assign(name, value.clone())?,
        }
        Ok(value)
    }

    fn evaluate_get(
        &self,
        object_expr: &Expr,
        name: &Token,
        expr_arena: &ExprArena,
    ) -> Result<Value, RuntimeError> {
        let object = self.evaluate(object_expr, expr_arena)?;

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
        expr_arena: &ExprArena,
    ) -> Result<Value, RuntimeError> {
        let object = self.evaluate(object_expr, expr_arena)?;

        if let Value::Instance(instance) = object {
            let value = self.evaluate(value_expr, expr_arena)?;
            instance.set(name, value.clone());
            Ok(value)
        } else {
            Err(RuntimeError::new(
                name.clone(),
                "Only instances have fields.",
            ))
        }
    }

    fn evaluate_super(&self, keyword: &Token, method_name: &Token) -> Result<Value, RuntimeError> {
        // Resolver records `super` as a local binding, so start by finding
        // the captured superclass for the enclosing subclass declaration.
        let ResolvedBinding::Local { distance, slot } = self.resolved_binding(keyword) else {
            return Err(RuntimeError::new(
                keyword.clone(),
                "Undefined variable 'super'.",
            ));
        };

        let environment = self.current_environment();
        let superclass = match Environment::get_at(&environment, distance, slot, keyword)? {
            Value::Class(superclass) => superclass,
            _ => unreachable!("resolver should bind 'super' to a class value"),
        };

        // The environment that binds `this` sits immediately inside the one
        // that binds `super`, so one fewer hop recovers the current receiver.
        let this_keyword = Token::new(TokenType::This, "this".to_string(), None, keyword.line);
        let object = match Environment::get_at(
            &environment,
            distance
                .checked_sub(1)
                .expect("resolver should place 'this' inside the 'super' scope"),
            0,
            &this_keyword,
        )? {
            Value::Instance(object) => object,
            _ => unreachable!("methods using 'super' should always have a bound 'this'"),
        };

        // Look up the method starting at the superclass, then bind it back to
        // the current instance before returning it to the caller.
        let Some(method) = superclass.find_method(method_name.lexeme.as_ref()) else {
            return Err(RuntimeError::new(
                method_name.clone(),
                format!("Undefined property '{}'.", method_name.lexeme),
            ));
        };

        Ok(Value::Callable(method.bind(object)))
    }

    // Read a variable using the resolver's precomputed lexical distance when
    // available, falling back to dynamic lookup only for unresolved tests and
    // legacy call sites that bypass the resolver pass.
    fn look_up_variable(&self, name: &Token) -> Result<Value, RuntimeError> {
        match self.resolved_binding(name) {
            ResolvedBinding::Local { distance, slot } => {
                Environment::get_at(&self.current_environment(), distance, slot, name)
            }
            ResolvedBinding::Global => self.globals.borrow().get(name),
            ResolvedBinding::Unresolved => self.current_environment().borrow().get(name),
        }
    }

    fn evaluate_call(
        &self,
        callee_expr: &Expr,
        paren: &Token,
        argument_exprs: &[ExprRef],
        expr_arena: &ExprArena,
    ) -> Result<Value, RuntimeError> {
        // Evaluate the callee expression first. This may be a simple variable
        // lookup like `clock`, but the grammar allows any higher-precedence
        // expression to appear before the call parentheses.
        let callee = self.evaluate(callee_expr, expr_arena)?;

        // Lox evaluates call arguments from left to right before dispatching
        // to the callee.
        let mut arguments = Vec::with_capacity(argument_exprs.len());
        for argument_expr in argument_exprs {
            arguments.push(self.evaluate(self.expr(expr_arena, *argument_expr), expr_arena)?);
        }

        // Convert the runtime value into the callable interface or report a
        // user-facing runtime error instead of crashing on a host-language
        // type mismatch.
        match callee {
            Value::Callable(callable) => {
                Self::check_call_arity(paren, callable.arity(), arguments.len())?;

                // Hand off to the concrete callable implementation.
                callable.call(self, arguments)
            }
            Value::Class(class) => {
                Self::check_call_arity(paren, class.arity(), arguments.len())?;

                LoxClass::call(class, self, arguments)
            }
            _ => Err(RuntimeError::new(
                paren.clone(),
                "Can only call functions and classes.",
            )),
        }
    }

    fn check_call_arity(
        paren: &Token,
        expected_arguments: usize,
        actual_arguments: usize,
    ) -> Result<(), RuntimeError> {
        if actual_arguments == expected_arguments {
            Ok(())
        } else {
            Err(RuntimeError::new(
                paren.clone(),
                format!(
                    "Expected {} arguments but got {}.",
                    expected_arguments, actual_arguments
                ),
            ))
        }
    }

    fn evaluate_logical(
        &self,
        left_expr: &Expr,
        operator: &Token,
        right_expr: &Expr,
        expr_arena: &ExprArena,
    ) -> Result<Value, RuntimeError> {
        let left = self.evaluate(left_expr, expr_arena)?;

        match operator.type_ {
            TokenType::Or if Self::is_truthy(&left) => Ok(left),
            TokenType::And if !Self::is_truthy(&left) => Ok(left),
            TokenType::Or | TokenType::And => self.evaluate(right_expr, expr_arena),
            _ => unreachable!("parser should only build valid logical operators"),
        }
    }

    fn evaluate_conditional(
        &self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: &Expr,
        expr_arena: &ExprArena,
    ) -> Result<Value, RuntimeError> {
        if Self::is_truthy(&self.evaluate(condition, expr_arena)?) {
            self.evaluate(then_branch, expr_arena)
        } else {
            self.evaluate(else_branch, expr_arena)
        }
    }

    fn evaluate_unary(
        &self,
        operator: &Token,
        right_expr: &Expr,
        expr_arena: &ExprArena,
    ) -> Result<Value, RuntimeError> {
        let right = self.evaluate(right_expr, expr_arena)?;

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
        expr_arena: &ExprArena,
    ) -> Result<Value, RuntimeError> {
        if operator.type_ == TokenType::Plus {
            return self.evaluate_plus(operator, left_expr, right_expr, expr_arena);
        }

        let left = self.evaluate(left_expr, expr_arena)?;
        let right = self.evaluate(right_expr, expr_arena)?;

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

    fn evaluate_plus(
        &self,
        operator: &Token,
        left_expr: &Expr,
        right_expr: &Expr,
        expr_arena: &ExprArena,
    ) -> Result<Value, RuntimeError> {
        Ok(Self::combine_plus_values(
            operator,
            self.evaluate_plus_operand(left_expr, expr_arena)?,
            self.evaluate_plus_operand(right_expr, expr_arena)?,
        )?
        .into_value())
    }

    fn evaluate_plus_operand(
        &self,
        expr: &Expr,
        expr_arena: &ExprArena,
    ) -> Result<PlusValue, RuntimeError> {
        // Fold nested `+` subtrees into `PlusValue` segments so string-heavy
        // chains can postpone allocation until the final materialization step.
        //
        // This still preserves the parsed tree shape and left-to-right
        // evaluation order: we recurse through the existing left and right
        // children exactly as they appear in the AST, rather than flattening
        // across grouping boundaries. The optimization is only sound because
        // the current `+` rules remain equivalent under that tree-preserving
        // fold. If the language later adds richer concatenation behavior,
        // revisit this path together with its grouping-sensitive tests.
        if let Expr::Binary {
            left,
            operator,
            right,
        } = expr
            && operator.type_ == TokenType::Plus
        {
            return Self::combine_plus_values(
                operator,
                self.evaluate_plus_operand(self.expr(expr_arena, *left), expr_arena)?,
                self.evaluate_plus_operand(self.expr(expr_arena, *right), expr_arena)?,
            );
        }

        Ok(PlusValue::from(self.evaluate(expr, expr_arena)?))
    }

    fn combine_plus_values(
        operator: &Token,
        left: PlusValue,
        right: PlusValue,
    ) -> Result<PlusValue, RuntimeError> {
        match (left, right) {
            (PlusValue::Number(left), PlusValue::Number(right)) => {
                Ok(PlusValue::Number(left + right))
            }
            (PlusValue::Text(mut text), right) => {
                text.push_plus_value(right);
                Ok(PlusValue::Text(text))
            }
            (left, PlusValue::Text(mut text)) => {
                text.prepend_plus_value(left);
                Ok(PlusValue::Text(text))
            }
            _ => Err(RuntimeError::new(
                operator.clone(),
                "Operands must be two numbers or at least one string.",
            )),
        }
    }

    // Divide two numeric operands using the host `f64` / IEEE 754 rules.
    // This matches the book's Java-style numeric behavior, so non-zero / 0.0
    // yields signed infinity and 0.0 / 0.0 yields NaN instead of a runtime
    // error.
    fn apply_divide(operator: &Token, left: &Value, right: &Value) -> Result<Value, RuntimeError> {
        let (left, right) = Self::expect_number_operands(operator, left, right)?;
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
            // Numeric equality intentionally follows Rust `f64` / IEEE 754
            // behavior through `Value::PartialEq`: `NaN != NaN`, while
            // `+0.0 == -0.0`.
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
