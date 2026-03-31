use std::{env, io, process};

#[cfg(test)]
mod ast_printer;
mod environment;
mod expr;
mod interpreter;
mod lox;
mod parser;
mod scanner;
mod stmt;
mod token;

fn main() -> io::Result<()> {
    let args = env::args().skip(1).collect::<Vec<String>>();

    match args.as_slice() {
        [] => lox::run_prompt()?,
        [script] => lox::run_file(script)?,
        _ => {
            eprintln!("Usage: oxidized-lox [script]");
            process::exit(64);
        }
    }

    Ok(())
}
