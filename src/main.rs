use crate::scanner::Scanner;
use std::{
    env, fs,
    io::{self, Write},
    process,
};

mod scanner;
mod token;

fn main() -> io::Result<()> {
    let args = env::args().skip(1).collect::<Vec<String>>();

    match args.as_slice() {
        [] => run_prompt()?,
        [script] => run_file(script)?,
        _ => {
            eprintln!("Usage: oxidized-lox [script]");
            process::exit(64);
        }
    }

    Ok(())
}

fn run_file(path: &str) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    run(&source);

    Ok(())
}

fn run_prompt() -> io::Result<()> {
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