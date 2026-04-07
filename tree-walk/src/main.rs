use std::{env, io, process};

use tree_walk::{run_file, run_prompt};

fn main() -> io::Result<()> {
    let args = env::args().skip(1).collect::<Vec<String>>();

    match args.as_slice() {
        [] => run_prompt()?,
        [script] => run_file(script)?,
        _ => {
            eprintln!("Usage: tree-walk [script]");
            process::exit(64);
        }
    }

    Ok(())
}
