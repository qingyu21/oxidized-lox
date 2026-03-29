use crate::{
    interpreter::Interpreter,
    parser::Parser,
    scanner::Scanner,
    token::{Token, TokenType},
};
use std::{
    fs,
    io::{self, Write},
    process,
    sync::atomic::{AtomicBool, Ordering},
};

static HAD_ERROR: AtomicBool = AtomicBool::new(false);
static HAD_RUNTIME_ERROR: AtomicBool = AtomicBool::new(false);

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

        run(line.trim_end());
        clear_error();
        clear_runtime_error();
    }

    Ok(())
}

fn run(source: &str) {
    // TODO(perf): This pipeline materializes both the full token stream and
    // the full AST before evaluation. A bytecode VM or arena-backed frontend
    // could cut allocation and traversal overhead later on.
    let scanner = Scanner::new(source);
    let tokens = scanner.scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    // Stop if there was a syntax error.
    if had_error() {
        return;
    }

    let interpreter = Interpreter;
    interpreter.interpret(&statements);
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
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn run_marks_syntax_errors_without_runtime_errors() {
        with_clean_error_state(|| {
            run("print ;");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_marks_runtime_errors_without_syntax_errors() {
        with_clean_error_state(|| {
            run("1 / 0;");

            assert!(!had_error());
            assert!(had_runtime_error());
        });
    }

    #[test]
    fn run_stops_before_execution_after_parse_error() {
        with_clean_error_state(|| {
            run("print ; 1 / 0;");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn runtime_error_sets_only_runtime_flag() {
        with_clean_error_state(|| {
            runtime_error(&token(TokenType::Slash, "/", 7), "Division by zero.");

            assert!(!had_error());
            assert!(had_runtime_error());
        });
    }

    fn with_clean_error_state(test: impl FnOnce()) {
        let _guard = TEST_LOCK.lock().expect("test lock should not be poisoned");
        clear_error();
        clear_runtime_error();
        test();
        clear_error();
        clear_runtime_error();
    }

    fn token(type_: TokenType, lexeme: &str, line: u32) -> Token {
        Token::new(type_, lexeme.to_string(), None, line)
    }
}
