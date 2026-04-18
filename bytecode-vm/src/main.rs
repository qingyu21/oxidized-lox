use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args();
    let program = args.next().unwrap_or_else(|| String::from("bytecode-vm"));

    match (args.next(), args.next()) {
        (None, None) => {
            bytecode_vm::repl();
            ExitCode::SUCCESS
        }
        (Some(path), None) => bytecode_vm::run_file(path),
        _ => {
            eprintln!("Usage: {program} [path]");
            ExitCode::from(64)
        }
    }
}
