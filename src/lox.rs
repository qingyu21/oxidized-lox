use crate::{
    interpreter::Interpreter,
    parser::Parser,
    resolver::Resolver,
    scanner::Scanner,
    token::{Token, TokenType},
};
use std::{
    cell::RefCell,
    fs,
    io::{self, Write},
    process,
    sync::atomic::{AtomicBool, Ordering},
};

static HAD_ERROR: AtomicBool = AtomicBool::new(false);
static HAD_RUNTIME_ERROR: AtomicBool = AtomicBool::new(false);

thread_local! {
    static INTERPRETER: RefCell<Interpreter> = RefCell::new(Interpreter::new());
}

pub(crate) fn run_file(path: &str) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    clear_error();
    clear_runtime_error();
    run(&source);

    if had_error() {
        process::exit(65);
    }

    if had_runtime_error() {
        process::exit(70);
    }

    Ok(())
}

pub(crate) fn run_prompt() -> io::Result<()> {
    let stdin = io::stdin();

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut line = String::new();
        let bytes_read = stdin.read_line(&mut line)?;

        if bytes_read == 0 {
            break;
        }

        // TODO(repl): Buffer incomplete multi-line input so statements like
        // `if (...)` or blocks can span multiple prompt lines before parsing.
        run_repl(line.trim_end());
        clear_error();
        clear_runtime_error();
    }

    Ok(())
}

fn run(source: &str) {
    // TODO(perf): This pipeline materializes both the full token stream and
    // the full AST before evaluation. A bytecode VM or arena-backed frontend
    // could cut allocation and traversal overhead later on.
    let tokens = Scanner::new(source).scan_tokens();
    run_tokens(tokens);
}

// REPL input may be either a full statement or a bare expression whose value
// should be echoed back to the user.
fn run_repl(source: &str) {
    // TODO(perf): This pipeline materializes both the full token stream and
    // the full AST before evaluation. A bytecode VM or arena-backed frontend
    // could cut allocation and traversal overhead later on.
    let tokens = Scanner::new(source).scan_tokens();

    if is_empty_input(&tokens) {
        return;
    }

    if should_eval_repl_expression(&tokens) {
        let mut parser = Parser::new(tokens);
        let Some(expr) = parser.parse_expression_input() else {
            return;
        };

        // Stop if there was a syntax error.
        if had_error() {
            return;
        }

        INTERPRETER.with(|interpreter| {
            let interpreter = interpreter.borrow();
            let mut resolver = Resolver::new(&interpreter);
            if resolver.resolve_expression(&expr).is_err() || had_error() {
                return;
            }

            interpreter.interpret_expression(&expr);
        });
        return;
    }

    run_tokens(tokens);
}

fn run_tokens(tokens: Vec<Token>) {
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    // Stop if there was a syntax error.
    if had_error() {
        return;
    }

    // The tree-walk pipeline is scanner -> parser -> resolver -> interpreter.
    INTERPRETER.with(|interpreter| {
        let interpreter = interpreter.borrow();
        let mut resolver = Resolver::new(&interpreter);
        if resolver.resolve_statements(&statements).is_err() || had_error() {
            return;
        }

        interpreter.interpret(&statements);
    });
}

fn is_empty_input(tokens: &[Token]) -> bool {
    matches!(
        tokens,
        [Token {
            type_: TokenType::Eof,
            ..
        }]
    )
}

// Use a small token-based heuristic so the REPL can accept bare expressions
// without first trying statement parsing and emitting a spurious syntax error.
fn should_eval_repl_expression(tokens: &[Token]) -> bool {
    if starts_with_statement(tokens) {
        return false;
    }

    !ends_with_semicolon(tokens)
}

fn starts_with_statement(tokens: &[Token]) -> bool {
    matches!(
        tokens.first().map(|token| token.type_),
        Some(
            TokenType::Print
                | TokenType::Var
                | TokenType::LeftBrace
                | TokenType::If
                | TokenType::While
                | TokenType::For
                | TokenType::Break
                | TokenType::Fun
                | TokenType::Class
                | TokenType::Return
        )
    )
}

fn ends_with_semicolon(tokens: &[Token]) -> bool {
    matches!(
        tokens.iter().rev().nth(1).map(|token| token.type_),
        Some(TokenType::Semicolon)
    )
}

pub(crate) fn error(line: u32, message: &str) {
    report(line, "", message);
}

pub(crate) fn token_error(token: &Token, message: &str) {
    let where_ = if token.type_ == TokenType::Eof {
        " at end".to_string()
    } else {
        format!(" at '{}'", token.lexeme)
    };

    report(token.line, &where_, message);
}

pub(crate) fn runtime_error(token: &Token, message: &str) {
    eprintln!("{message}\n[line {}]", token.line);
    HAD_RUNTIME_ERROR.store(true, Ordering::Relaxed);
}

fn had_error() -> bool {
    HAD_ERROR.load(Ordering::Relaxed)
}

fn had_runtime_error() -> bool {
    HAD_RUNTIME_ERROR.load(Ordering::Relaxed)
}

fn clear_error() {
    HAD_ERROR.store(false, Ordering::Relaxed);
}

fn clear_runtime_error() {
    HAD_RUNTIME_ERROR.store(false, Ordering::Relaxed);
}

fn report(line: u32, where_: &str, message: &str) {
    eprintln!("[line {line}] Error{where_}: {message}");
    HAD_ERROR.store(true, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    #[test]
    fn run_marks_syntax_errors_without_runtime_errors() {
        with_clean_error_state(|| {
            run("print ;");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_marks_runtime_errors_without_syntax_errors() {
        with_clean_error_state(|| {
            run("1 / 0;");

            assert!(!had_error());
            assert!(had_runtime_error());
        });
    }

    #[test]
    fn run_stops_before_execution_after_parse_error() {
        with_clean_error_state(|| {
            run("print ; 1 / 0;");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn runtime_error_sets_only_runtime_flag() {
        with_clean_error_state(|| {
            runtime_error(&token(TokenType::Slash, "/", 7), "Division by zero.");

            assert!(!had_error());
            assert!(had_runtime_error());
        });
    }

    #[test]
    fn repl_style_runs_share_the_same_interpreter_state() {
        with_clean_error_state(|| {
            run("var beverage = \"tea\";");
            clear_error();
            clear_runtime_error();

            run("beverage;");

            assert!(!had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn repl_evaluates_bare_expressions() {
        with_clean_error_state(|| {
            run_repl("1 / 0");

            assert!(!had_error());
            assert!(had_runtime_error());
        });
    }

    #[test]
    fn repl_still_executes_semicolon_terminated_statements() {
        with_clean_error_state(|| {
            run_repl("var beverage = \"tea\";");
            clear_error();
            clear_runtime_error();

            run_repl("beverage");

            assert!(!had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn repl_keeps_expression_statements_as_statements() {
        with_clean_error_state(|| {
            run_repl("1 / 0;");

            assert!(!had_error());
            assert!(had_runtime_error());
        });
    }

    #[test]
    fn run_marks_too_many_call_arguments_as_a_syntax_error() {
        with_clean_error_state(|| {
            let arguments = std::iter::repeat_n("1", 256).collect::<Vec<_>>().join(", ");
            let source = format!("clock({arguments});");

            run(&source);

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_marks_too_many_function_parameters_as_a_syntax_error() {
        with_clean_error_state(|| {
            let params = (0..256)
                .map(|index| format!("p{index}"))
                .collect::<Vec<_>>()
                .join(", ");
            let source = format!("fun tooMany({params}) {{}}");

            run(&source);

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_marks_top_level_return_as_a_syntax_error() {
        with_clean_error_state(|| {
            run("return 1;");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_marks_local_self_initializer_as_a_syntax_error() {
        with_clean_error_state(|| {
            run("{ var a = a; }");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn duplicate_local_variable_in_same_scope_is_a_resolver_error() {
        with_clean_error_state(|| {
            run("{ var a = 1; var a = 2; }");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn duplicate_function_parameter_name_is_a_resolver_error() {
        with_clean_error_state(|| {
            run("fun bad(a, a) {}");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_marks_this_outside_a_class_as_a_resolution_error() {
        with_clean_error_state(|| {
            run("print this;");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_marks_this_inside_a_non_method_function_as_a_resolution_error() {
        with_clean_error_state(|| {
            run("fun notAMethod() { print this; }");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_marks_returning_a_value_from_an_initializer_as_a_resolution_error() {
        with_clean_error_state(|| {
            run("class Foo { init() { return \"something else\"; } }");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_marks_inheriting_from_self_as_a_resolution_error() {
        with_clean_error_state(|| {
            run("class Oops < Oops {}");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_marks_unused_local_variable_as_a_resolution_error() {
        with_clean_error_state(|| {
            run("{ var beverage = \"tea\"; }");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_allows_used_local_variables() {
        with_clean_error_state(|| {
            run("{ var beverage = \"tea\"; print beverage; }");

            assert!(!had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_treats_closure_reads_as_using_the_enclosing_local() {
        with_clean_error_state(|| {
            run("fun outer() {
                   var beverage = \"tea\";
                   fun inner() {
                     print beverage;
                   }
                 }");

            assert!(!had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_does_not_report_unused_function_parameters() {
        with_clean_error_state(|| {
            run("fun show(beverage) {}");

            assert!(!had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn run_still_marks_assignment_only_locals_as_unused() {
        with_clean_error_state(|| {
            run("{ var count = 1; count = 2; }");

            assert!(had_error());
            assert!(!had_runtime_error());
        });
    }

    #[test]
    fn bare_expressions_are_detected_in_the_repl() {
        let tokens = Scanner::new("1 + 2").scan_tokens();
        assert!(should_eval_repl_expression(&tokens));
    }

    #[test]
    fn semicolon_terminated_inputs_are_not_treated_as_bare_repl_expressions() {
        let tokens = Scanner::new("1 + 2;").scan_tokens();
        assert!(!should_eval_repl_expression(&tokens));
    }

    fn with_clean_error_state(test: impl FnOnce()) {
        let _guard = TEST_LOCK.lock().expect("test lock should not be poisoned");
        reset_interpreter();
        clear_error();
        clear_runtime_error();
        test();
        reset_interpreter();
        clear_error();
        clear_runtime_error();
    }

    fn token(type_: TokenType, lexeme: &str, line: u32) -> Token {
        Token::new(type_, lexeme.to_string(), None, line)
    }

    fn reset_interpreter() {
        INTERPRETER.with(|interpreter| {
            *interpreter.borrow_mut() = Interpreter::new();
        });
    }
}
