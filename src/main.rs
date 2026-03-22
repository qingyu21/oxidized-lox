use std::{env, io, process};

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
    println!("run file: {path}");
    Ok(())
}

fn run_prompt() -> io::Result<()> {
    println!("run prompt");
    Ok(())
}
