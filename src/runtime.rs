mod error;
mod object;
mod value;

pub(crate) use self::{
    error::RuntimeError,
    object::{LoxCallable, LoxClass, LoxInstance},
    value::Value,
};
