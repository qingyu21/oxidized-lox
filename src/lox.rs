use crate::scanner::Scanner;
use std::{
    fs,
    io::{self, Write},
    sync::atomic::{AtomicBool, Ordering},
};

static HAD_ERROR: AtomicBool = AtomicBool::new(false);

pub(crate) fn run_file(path: &str) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    run(&source);

    // if HAD_ERROR.load(Ordering::Relaxed) {
    //     process::exit(65);
    // }

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
        HAD_ERROR.store(false, Ordering::Relaxed);
    }

    Ok(())
}

fn run(source: &str) {
    let scanner = Scanner::new(source);
    let tokens = scanner.scan_tokens();

    for token in tokens {
        println!("{token}");
    }
}

#[allow(dead_code)]
pub(crate) fn error(line: u32, message: &str) {
    report(line, "", message);
}

#[allow(dead_code)]
fn report(line: u32, where_: &str, message: &str) {
    eprintln!("[line {line}] Error{where_}: {message}");
    HAD_ERROR.store(true, Ordering::Relaxed);
}
