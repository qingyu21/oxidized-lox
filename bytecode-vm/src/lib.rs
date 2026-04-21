use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

use crate::{chunk::Chunk, vm::Vm};

// These modules are still being wired into the executable path, so we suppress
// dead-code noise while the VM scaffold is taking shape.
#[allow(dead_code)]
pub(crate) mod chunk;
pub(crate) mod compiler;
#[allow(dead_code)]
pub(crate) mod debug;
pub(crate) mod scanner;
pub(crate) mod value;
pub(crate) mod vm;

/// Runs a minimal line-at-a-time REPL.
pub fn repl() {
    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("> ");
        let _ = io::stdout().flush();

        line.clear();
        match stdin.read_line(&mut line) {
            Ok(0) => {
                println!();
                break;
            }
            Ok(_) => {
                let _ = interpret(&line);
            }
            Err(error) => {
                eprintln!("Failed to read REPL input: {error}");
                break;
            }
        }
    }
}

/// Loads a script from disk and returns the process exit code the caller should use.
pub fn run_file(path: impl AsRef<Path>) -> ExitCode {
    let path = path.as_ref();
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(_) => {
            eprintln!("Could not read file \"{}\".", path.display());
            return ExitCode::from(74);
        }
    };

    match interpret(&source) {
        vm::InterpretResult::Ok => ExitCode::SUCCESS,
        vm::InterpretResult::CompileError => ExitCode::from(65),
        vm::InterpretResult::RuntimeError => ExitCode::from(70),
    }
}

/// Compiles source into a fresh chunk and executes it when compilation succeeds.
fn interpret(source: &str) -> vm::InterpretResult {
    let mut chunk = Chunk::new();
    if !compiler::compile(source, &mut chunk) {
        return vm::InterpretResult::CompileError;
    }

    let mut vm = Vm::new();
    vm.interpret(&chunk)
}

/// Returns the current implementation status of the bytecode VM crate.
pub fn status() -> &'static str {
    "bytecode-vm scaffold: execution pipeline wired; compiler parser skeleton in progress"
}
