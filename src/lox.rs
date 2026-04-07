use crate::{
    diagnostics::{clear_error, clear_runtime_error, had_error, had_runtime_error, runtime_error},
    expr::Expr,
    interpreter::Interpreter,
    parser::Parser,
    resolver::Resolver,
    scanner::Scanner,
    stmt::Stmt,
    token::{Token, TokenType},
};
use std::{
    cell::RefCell,
    fs,
    io::{self, Write},
    process,
};

const EX_DATAERR: i32 = 65;
const EX_SOFTWARE: i32 = 70;

thread_local! {
    static INTERPRETER: RefCell<Interpreter> = RefCell::new(Interpreter::new());
}

pub(crate) fn run_file(path: &str) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    clear_error();
    clear_runtime_error();
    run(&source);

    if had_error() {
        process::exit(EX_DATAERR);
    }

    if had_runtime_error() {
        process::exit(EX_SOFTWARE);
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
    let tokens = scan_tokens(source);
    run_program_tokens(tokens);
}

// REPL input may be either a full statement or a bare expression whose value
// should be echoed back to the user.
fn run_repl(source: &str) {
    // TODO(perf): This pipeline materializes both the full token stream and
    // the full AST before evaluation. A bytecode VM or arena-backed frontend
    // could cut allocation and traversal overhead later on.
    let tokens = scan_tokens(source);

    match classify_repl_input(&tokens) {
        ReplInput::Empty => {}
        ReplInput::Expression => {
            let Some(expr) = parse_repl_expression(tokens) else {
                return;
            };
            resolve_and_interpret_expression(&expr);
        }
        ReplInput::Program => run_program_tokens(tokens),
    }
}

fn scan_tokens(source: &str) -> Vec<Token> {
    Scanner::new(source).scan_tokens()
}

fn run_program_tokens(tokens: Vec<Token>) {
    let Some(statements) = parse_program(tokens) else {
        return;
    };

    resolve_and_interpret_statements(&statements);
}

fn parse_program(tokens: Vec<Token>) -> Option<Vec<Stmt>> {
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    if stop_after_error() {
        None
    } else {
        Some(statements)
    }
}

fn parse_repl_expression(tokens: Vec<Token>) -> Option<Expr> {
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expression_input()?;

    if stop_after_error() { None } else { Some(expr) }
}

fn resolve_and_interpret_statements(statements: &[Stmt]) {
    with_interpreter(|interpreter| {
        if !resolve_statements(interpreter, statements) {
            return;
        }

        if let Err(error) = interpreter.interpret(statements) {
            runtime_error(&error.token, &error.message);
        }
    });
}

fn resolve_and_interpret_expression(expr: &Expr) {
    with_interpreter(|interpreter| {
        if !resolve_expression(interpreter, expr) {
            return;
        }

        match interpreter.interpret_expression(expr) {
            Ok(value) => println!("{value}"),
            Err(error) => runtime_error(&error.token, &error.message),
        }
    });
}

fn with_interpreter<R>(f: impl FnOnce(&Interpreter) -> R) -> R {
    INTERPRETER.with(|interpreter| {
        let interpreter = interpreter.borrow();
        f(&interpreter)
    })
}

fn resolve_statements(interpreter: &Interpreter, statements: &[Stmt]) -> bool {
    let mut resolver = Resolver::new(interpreter);
    resolver.resolve_statements(statements).is_ok() && !had_error()
}

fn resolve_expression(interpreter: &Interpreter, expr: &Expr) -> bool {
    let mut resolver = Resolver::new(interpreter);
    resolver.resolve_expression(expr).is_ok() && !had_error()
}

fn stop_after_error() -> bool {
    had_error()
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

enum ReplInput {
    Empty,
    Expression,
    Program,
}

fn classify_repl_input(tokens: &[Token]) -> ReplInput {
    if is_empty_input(tokens) {
        ReplInput::Empty
    } else if should_eval_repl_expression(tokens) {
        ReplInput::Expression
    } else {
        ReplInput::Program
    }
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

#[cfg(test)]
mod tests;
