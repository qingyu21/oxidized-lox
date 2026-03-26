use std::{env, io, process};

mod ast_printer;
mod expr;
mod lox;
mod scanner;
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
