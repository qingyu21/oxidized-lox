mod error;
mod object;
mod value;

// Re-export the small runtime submodules behind a single `crate::runtime::*`
// surface so interpreter/environment code does not need to care how runtime
// types are split across files.
pub(crate) use self::{
    error::RuntimeError,
    object::{LoxCallable, LoxClass, LoxInstance},
    value::Value,
};
