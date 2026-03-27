use crate::{
    ast_printer::AstPrinter,
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

pub(crate) fn run_file(path: &str) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    clear_error();
    run(&source);

    if had_error() {
        process::exit(65);
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
    }

    Ok(())
}

fn run(source: &str) {
    let scanner = Scanner::new(source);
    let tokens = scanner.scan_tokens();
    let mut parser = Parser::new(tokens);
    let expression = parser.parse();

    // Stop if there was a syntax error.
    if had_error() {
        return;
    }

    let printer = AstPrinter;
    println!(
        "{}",
        printer.print(
            expression
                .as_ref()
                .expect("parser should return an expression when no error occurred")
        )
    );
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

fn had_error() -> bool {
    HAD_ERROR.load(Ordering::Relaxed)
}

fn clear_error() {
    HAD_ERROR.store(false, Ordering::Relaxed);
}

fn report(line: u32, where_: &str, message: &str) {
    eprintln!("[line {line}] Error{where_}: {message}");
    HAD_ERROR.store(true, Ordering::Relaxed);
}
