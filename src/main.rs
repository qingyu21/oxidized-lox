use std::{env, io, process};

#[cfg(test)]
mod ast_printer;
mod diagnostics;
mod environment;
mod frontend;
mod interpreter;
mod lox;
mod resolver;
mod runtime;
#[cfg(test)]
mod test_support;

// Keep the historical `crate::expr`, `crate::parser`, and similar paths
// working even though the concrete files now live under `src/frontend/`.
pub(crate) use frontend::{expr, parser, scanner, stmt, token};

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
