use std::{fmt, rc::Rc};

use crate::{
    interpreter::Interpreter,
    token::{Literal, Token},
};

pub(crate) trait LoxCallable: fmt::Debug + fmt::Display {
    fn arity(&self) -> usize;
    fn call(&self, interpreter: &Interpreter, arguments: Vec<Value>)
    -> Result<Value, RuntimeError>;
}

#[derive(Debug, Clone)]
pub(crate) struct LoxClass {
    name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct LoxInstance {
    klass: Rc<LoxClass>,
}

#[derive(Debug, Clone)]
pub(crate) enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Nil,
    Callable(Rc<dyn LoxCallable>),
    Class(Rc<LoxClass>),
    Instance(Rc<LoxInstance>),
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeError {
    pub(crate) token: Token,
    pub(crate) message: String,
}

impl RuntimeError {
    pub(crate) fn new(token: Token, message: impl Into<String>) -> Self {
        Self {
            token,
            message: message.into(),
        }
    }
}

impl LoxClass {
    pub(crate) fn new(name: String) -> Self {
        Self { name }
    }

    pub(crate) fn instantiate(class: Rc<LoxClass>) -> Value {
        Value::Instance(Rc::new(LoxInstance::new(class)))
    }
}

impl LoxInstance {
    fn new(klass: Rc<LoxClass>) -> Self {
        Self { klass }
    }
}

impl LoxCallable for LoxClass {
    fn arity(&self) -> usize {
        0
    }

    fn call(
        &self,
        _interpreter: &Interpreter,
        _arguments: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        Ok(Self::instantiate(Rc::new(self.clone())))
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

impl fmt::Display for LoxClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl fmt::Display for LoxInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} instance", self.klass.name)
    }
}
