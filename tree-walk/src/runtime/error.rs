use thiserror::Error;

use crate::token::Token;

#[derive(Debug, Clone, Error)]
#[error("{message}")]
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
