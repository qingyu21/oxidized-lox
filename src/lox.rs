use crate::{
    interpreter::Interpreter,
    parser::Parser,
    resolver::Resolver,
    scanner::Scanner,
    token::{Token, TokenType},
};
use std::{
    cell::RefCell,
    fs,
    io::{self, Write},
    process,
    sync::atomic::{AtomicBool, Ordering},
};

static HAD_ERROR: AtomicBool = AtomicBool::new(false);
static HAD_RUNTIME_ERROR: AtomicBool = AtomicBool::new(false);

thread_local! {
    static INTERPRETER: RefCell<Interpreter> = RefCell::new(Interpreter::new());
}

pub(crate) fn run_file(path: &str) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    clear_error();
    clear_runtime_error();
    run(&source);

    if had_error() {
        process::exit(65);
    }

    if had_runtime_error() {
        process::exit(70);
    }

    Ok(())
}

pub(crate) fn run_prompt() -> io::Result<()> {
    let stdin = io::stdin();

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut line = String::new();
        let bytes_read = stdin.read_line(&mut line)?;

        if bytes_read == 0 {
            break;
        }

        // TODO(repl): Buffer incomplete multi-line input so statements like
        // `if (...)` or blocks can span multiple prompt lines before parsing.
        run_repl(line.trim_end());
        clear_error();
        clear_runtime_error();
    }

    Ok(())
}

fn run(source: &str) {
    // TODO(perf): This pipeline materializes both the full token stream and
    // the full AST before evaluation. A bytecode VM or arena-backed frontend
    // could cut allocation and traversal overhead later on.
    let tokens = Scanner::new(source).scan_tokens();
    run_tokens(tokens);
}

// REPL input may be either a full statement or a bare expression whose value
// should be echoed back to the user.
fn run_repl(source: &str) {
    // TODO(perf): This pipeline materializes both the full token stream and
    // the full AST before evaluation. A bytecode VM or arena-backed frontend
    // could cut allocation and traversal overhead later on.
    let tokens = Scanner::new(source).scan_tokens();

    if is_empty_input(&tokens) {
        return;
    }

    if should_eval_repl_expression(&tokens) {
        let mut parser = Parser::new(tokens);
        let Some(expr) = parser.parse_expression_input() else {
            return;
        };

        // Stop if there was a syntax error.
        if had_error() {
            return;
        }

        INTERPRETER.with(|interpreter| {
            let interpreter = interpreter.borrow();
            let mut resolver = Resolver::new(&interpreter);
            if resolver.resolve_expression(&expr).is_err() || had_error() {
                return;
            }

            interpreter.interpret_expression(&expr);
        });
        return;
    }

    run_tokens(tokens);
}

fn run_tokens(tokens: Vec<Token>) {
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    // Stop if there was a syntax error.
    if had_error() {
        return;
    }

    // The tree-walk pipeline is scanner -> parser -> resolver -> interpreter.
    INTERPRETER.with(|interpreter| {
        let interpreter = interpreter.borrow();
        let mut resolver = Resolver::new(&interpreter);
        if resolver.resolve_statements(&statements).is_err() || had_error() {
            return;
        }

        interpreter.interpret(&statements);
    });
}

fn is_empty_input(tokens: &[Token]) -> bool {
    matches!(
        tokens,
        [Token {
            type_: TokenType::Eof,
            ..
        }]
    )
}

// Use a small token-based heuristic so the REPL can accept bare expressions
// without first trying statement parsing and emitting a spurious syntax error.
fn should_eval_repl_expression(tokens: &[Token]) -> bool {
    if starts_with_statement(tokens) {
        return false;
    }

    !ends_with_semicolon(tokens)
}

fn starts_with_statement(tokens: &[Token]) -> bool {
    matches!(
        tokens.first().map(|token| token.type_),
        Some(
            TokenType::Print
                | TokenType::Var
                | TokenType::LeftBrace
                | TokenType::If
                | TokenType::While
                | TokenType::For
                | TokenType::Break
                | TokenType::Fun
                | TokenType::Class
                | TokenType::Return
        )
    )
}

fn ends_with_semicolon(tokens: &[Token]) -> bool {
    matches!(
        tokens.iter().rev().nth(1).map(|token| token.type_),
        Some(TokenType::Semicolon)
    )
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
    HAD_RUNTIME_ERROR.store(true, Ordering::Relaxed);
}

fn had_error() -> bool {
    HAD_ERROR.load(Ordering::Relaxed)
}

fn had_runtime_error() -> bool {
    HAD_RUNTIME_ERROR.load(Ordering::Relaxed)
}

fn clear_error() {
    HAD_ERROR.store(false, Ordering::Relaxed);
}

fn clear_runtime_error() {
    HAD_RUNTIME_ERROR.store(false, Ordering::Relaxed);
}

fn report(line: u32, where_: &str, message: &str) {
    eprintln!("[line {line}] Error{where_}: {message}");
    HAD_ERROR.store(true, Ordering::Relaxed);
}

#[cfg(test)]
mod tests;
