use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use crate::{
    interpreter::{Interpreter, LoxFunction},
    token::Token,
};

use super::{RuntimeError, Value};

pub(crate) trait LoxCallable: fmt::Debug + fmt::Display {
    fn arity(&self) -> usize;
    fn call(&self, interpreter: &Interpreter, arguments: Vec<Value>)
    -> Result<Value, RuntimeError>;
}

// TODO(ch12-challenge1): Static methods are not implemented yet. The larger
// metaclass-style solution would need class objects to participate in property
// lookup the way instances do, instead of only carrying an instance-method table.
// TODO(ch13-challenge1): Class reuse still goes through a single superclass
// link only. Alternative capability-sharing features like mixins, traits, or
// multiple inheritance have not been implemented.
#[derive(Debug, Clone)]
pub(crate) struct LoxClass {
    name: Rc<str>,
    // Subclasses follow this chain when a method is not found on the class
    // itself, which gives instances inherited behavior.
    superclass: Option<Rc<LoxClass>>,
    methods: HashMap<Rc<str>, Rc<LoxFunction>>,
}

#[derive(Debug, Clone)]
pub(crate) struct LoxInstance {
    // The tree-walk runtime deliberately keeps ordinary object edges strong:
    // fields own the `Value`s stored in them, classes own methods, and bound
    // methods own their receiver through captured `this`. That keeps object
    // identity and escaped-method behavior unsurprising, but it also means
    // cyclic graphs are retained in long-lived sessions.
    //
    // Common examples are `instance.self = instance`, mutually-referential
    // instances, and storing a bound method back onto the instance. A local
    // `Weak` swap is not enough here: weakening field values or bound `this`
    // would change observable Lox semantics by letting regular object
    // references or escaped methods go dead unexpectedly.
    //
    // For now this interpreter documents and tests that limitation rather
    // than partially hiding it. The real fix is a tracing GC, or a broader
    // runtime-handle redesign that can preserve those semantics while still
    // breaking cycles internally.
    klass: Rc<LoxClass>,
    fields: RefCell<HashMap<Rc<str>, Value>>,
}

impl LoxClass {
    pub(crate) fn new(
        name: impl Into<Rc<str>>,
        superclass: Option<Rc<LoxClass>>,
        methods: HashMap<Rc<str>, Rc<LoxFunction>>,
    ) -> Self {
        Self {
            name: name.into(),
            superclass,
            methods,
        }
    }

    fn instantiate(class: Rc<LoxClass>) -> Rc<LoxInstance> {
        Rc::new(LoxInstance::new(class))
    }

    pub(crate) fn arity(&self) -> usize {
        self.find_method("init")
            .map_or(0, |initializer| initializer.arity())
    }

    pub(crate) fn call(
        class: Rc<LoxClass>,
        interpreter: &Interpreter,
        arguments: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        let instance = Self::instantiate(class.clone());

        if let Some(initializer) = class.find_method("init") {
            initializer
                .bind(instance.clone())
                .call(interpreter, arguments)?;
        }

        Ok(Value::Instance(instance))
    }

    // TODO(ch13-challenge2): Method lookup still prefers the lowest class in
    // the inheritance chain and relies on `super` for upward dispatch. The
    // BETA-style `inner` chaining model from Chapter 13 challenge 2 is not
    // implemented.
    pub(crate) fn find_method(&self, name: &str) -> Option<Rc<LoxFunction>> {
        self.methods
            .get(name)
            .cloned()
            .or_else(|| self.superclass.as_ref()?.find_method(name))
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
        if let Some(value) = self.fields.borrow().get(name.lexeme.as_ref()).cloned() {
            Ok(value)
        } else if let Some(method) = self.klass.find_method(name.lexeme.as_ref()) {
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

#[cfg(test)]
mod tests;
