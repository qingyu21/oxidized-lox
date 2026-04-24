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
    "bytecode-vm scaffold: scanning, bytecode compilation, and arithmetic execution wired"
}

#[cfg(test)]
mod tests {
    use super::{interpret, run_file};
    use crate::vm::InterpretResult;
    use std::{
        env, fs,
        path::PathBuf,
        process::ExitCode,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_temp_path(file_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        env::temp_dir().join(format!("oxidized-lox-{file_name}-{unique}.lox"))
    }

    #[test]
    fn interpret_returns_ok_for_number_literal() {
        assert_eq!(interpret("123"), InterpretResult::Ok);
    }

    #[test]
    fn interpret_returns_ok_for_bool_and_nil_literals() {
        assert_eq!(interpret("true"), InterpretResult::Ok);
        assert_eq!(interpret("false"), InterpretResult::Ok);
        assert_eq!(interpret("nil"), InterpretResult::Ok);
    }

    #[test]
    fn interpret_returns_ok_for_logical_not() {
        assert_eq!(interpret("!true"), InterpretResult::Ok);
        assert_eq!(interpret("!nil"), InterpretResult::Ok);
        assert_eq!(interpret("!0"), InterpretResult::Ok);
    }

    #[test]
    fn run_file_returns_io_error_code_for_missing_files() {
        let missing = unique_temp_path("missing");

        assert_eq!(run_file(&missing), ExitCode::from(74));
    }

    #[test]
    fn run_file_returns_compile_error_exit_code_for_invalid_source() {
        let path = unique_temp_path("compile-error");
        fs::write(&path, "+").expect("should be able to write test source");

        let exit_code = run_file(&path);
        let _ = fs::remove_file(&path);

        assert_eq!(exit_code, ExitCode::from(65));
    }
}
