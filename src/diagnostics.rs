use std::cell::Cell;

use crate::token::{Token, TokenType};

thread_local! {
    // Diagnostics are tracked per thread so tests and future parallel entry
    // points do not leak syntax/runtime flags into one another.
    static HAD_ERROR: Cell<bool> = const { Cell::new(false) };
    static HAD_RUNTIME_ERROR: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn error(line: u32, message: &str) {
    report(line, "", message);
}

pub(crate) fn token_error(token: &Token, message: &str) {
    let where_ = if token.type_ == TokenType::Eof {
        " at end".to_string()
    } else {
        format!(" at '{}'", token.lexeme)
    };

    report(token.line, &where_, message);
}

pub(crate) fn runtime_error(token: &Token, message: &str) {
    eprintln!("{message}\n[line {}]", token.line);
    HAD_RUNTIME_ERROR.set(true);
}

pub(crate) fn had_error() -> bool {
    HAD_ERROR.get()
}

pub(crate) fn had_runtime_error() -> bool {
    HAD_RUNTIME_ERROR.get()
}

pub(crate) fn clear_error() {
    HAD_ERROR.set(false);
}

pub(crate) fn clear_runtime_error() {
    HAD_RUNTIME_ERROR.set(false);
}

fn report(line: u32, where_: &str, message: &str) {
    eprintln!("[line {line}] Error{where_}: {message}");
    HAD_ERROR.set(true);
}
