use std::{fmt, rc::Rc};

use crate::token::Literal;

use super::{LoxCallable, LoxClass, LoxInstance};

#[derive(Debug, Clone)]
pub(crate) enum Value {
    String(Rc<str>),
    Number(f64),
    Bool(bool),
    Nil,
    Callable(Rc<dyn LoxCallable>),
    Class(Rc<LoxClass>),
    Instance(Rc<LoxInstance>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(left), Value::String(right)) => left == right,
            (Value::Number(left), Value::Number(right)) => left == right,
            (Value::Bool(left), Value::Bool(right)) => left == right,
            (Value::Nil, Value::Nil) => true,
            (Value::Callable(left), Value::Callable(right)) => Rc::ptr_eq(left, right),
            (Value::Class(left), Value::Class(right)) => Rc::ptr_eq(left, right),
            (Value::Instance(left), Value::Instance(right)) => Rc::ptr_eq(left, right),
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
            Value::Class(class) => write!(f, "{class}"),
            Value::Instance(instance) => write!(f, "{instance}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::Value;
    use crate::token::Literal;

    #[test]
    fn converting_a_string_literal_into_a_value_reuses_the_same_backing_text() {
        let literal = Literal::String("tea".into());
        let shared = match &literal {
            Literal::String(value) => value.clone(),
            _ => unreachable!("test literal should be a string"),
        };

        let Value::String(value) = Value::from(literal.clone()) else {
            panic!("string literal should convert into a string runtime value");
        };

        assert!(
            Rc::ptr_eq(&shared, &value),
            "string literal evaluation should reuse the shared string backing"
        );
    }
}
