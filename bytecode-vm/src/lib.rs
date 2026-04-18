use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

// These modules are still being wired into the executable path, so we suppress
// dead-code noise while the VM scaffold is taking shape.
#[allow(dead_code)]
pub(crate) mod chunk;
#[allow(dead_code)]
pub(crate) mod compiler;
#[allow(dead_code)]
pub(crate) mod debug;
#[allow(dead_code)]
pub(crate) mod scanner;
#[allow(dead_code)]
pub(crate) mod value;
#[allow(dead_code)]
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
        vm::InterpretResult::InterpretOk => ExitCode::SUCCESS,
        vm::InterpretResult::InterpretCompileError => ExitCode::from(65),
        vm::InterpretResult::InterpretRuntimeError => ExitCode::from(70),
    }
}

/// Opens the front-end pipeline by compiling source text before execution exists.
fn interpret(source: &str) -> vm::InterpretResult {
    compiler::compile(source);
    vm::InterpretResult::InterpretOk
}

/// Returns the current implementation status of the bytecode VM crate.
pub fn status() -> &'static str {
    "bytecode-vm scaffold: compilation pipeline opened; scanner in progress"
}
