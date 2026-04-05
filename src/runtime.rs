use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use crate::{
    interpreter::{Interpreter, LoxFunction},
    token::{Literal, Token},
};

pub(crate) trait LoxCallable: fmt::Debug + fmt::Display {
    fn arity(&self) -> usize;
    fn call(&self, interpreter: &Interpreter, arguments: Vec<Value>)
    -> Result<Value, RuntimeError>;
}

// TODO(ch12-challenge1): Static methods are not implemented yet. The larger
// metaclass-style solution would need class objects to participate in property
// lookup the way instances do, instead of only carrying an instance-method table.
#[derive(Debug, Clone)]
pub(crate) struct LoxClass {
    name: String,
    methods: HashMap<String, Rc<LoxFunction>>,
}

#[derive(Debug, Clone)]
pub(crate) struct LoxInstance {
    klass: Rc<LoxClass>,
    fields: RefCell<HashMap<String, Value>>,
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
    pub(crate) fn new(name: String, methods: HashMap<String, Rc<LoxFunction>>) -> Self {
        Self { name, methods }
    }

    fn instantiate(class: Rc<LoxClass>) -> Rc<LoxInstance> {
        Rc::new(LoxInstance::new(class))
    }

    pub(crate) fn find_method(&self, name: &str) -> Option<Rc<LoxFunction>> {
        self.methods.get(name).cloned()
    }
}

impl LoxInstance {
    fn new(klass: Rc<LoxClass>) -> Self {
        Self {
            klass,
            fields: RefCell::new(HashMap::new()),
        }
    }

    // TODO(ch12-challenge2): Getter methods are not implemented yet. Property
    // reads currently return stored fields or bound methods, but they do not
    // execute user-defined getter bodies declared without parameter lists.
    pub(crate) fn get(self: &Rc<Self>, name: &Token) -> Result<Value, RuntimeError> {
        if let Some(value) = self.fields.borrow().get(&name.lexeme).cloned() {
            Ok(value)
        } else if let Some(method) = self.klass.find_method(&name.lexeme) {
            Ok(Value::Callable(method.bind(self.clone())))
        } else {
            Err(RuntimeError::new(
                name.clone(),
                format!("Undefined property '{}'.", name.lexeme),
            ))
        }
    }

    pub(crate) fn set(&self, name: &Token, value: Value) {
        self.fields.borrow_mut().insert(name.lexeme.clone(), value);
    }
}

impl LoxCallable for LoxClass {
    fn arity(&self) -> usize {
        self.find_method("init")
            .map_or(0, |initializer| initializer.arity())
    }

    fn call(
        &self,
        interpreter: &Interpreter,
        arguments: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        let instance = Self::instantiate(Rc::new(self.clone()));

        if let Some(initializer) = self.find_method("init") {
            initializer
                .bind(instance.clone())
                .call(interpreter, arguments)?;
        }

        Ok(Value::Instance(instance))
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
