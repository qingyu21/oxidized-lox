use crate::{
    diagnostics::{clear_error, clear_runtime_error, had_error, had_runtime_error, runtime_error},
    expr::Expr,
    interpreter::Interpreter,
    parser::{ParsedExpression, ParsedProgram, Parser},
    resolver::Resolver,
    scanner::Scanner,
    stmt::Stmt,
    token::TokenType,
};
use std::{
    cell::RefCell,
    fs,
    io::{self, Write},
    process,
};

const EX_DATAERR: i32 = 65;
const EX_SOFTWARE: i32 = 70;

thread_local! {
    static INTERPRETER: RefCell<Interpreter> = RefCell::new(Interpreter::new());
}

pub fn run_file(path: &str) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    clear_error();
    clear_runtime_error();
    run(source);

    if had_error() {
        process::exit(EX_DATAERR);
    }

    if had_runtime_error() {
        process::exit(EX_SOFTWARE);
    }

    Ok(())
}

pub fn run_prompt() -> io::Result<()> {
    let stdin = io::stdin();
    let mut pending_input = String::new();

    loop {
        print!("{}", repl_prompt(&pending_input));
        io::stdout().flush()?;

        let mut line = String::new();
        let bytes_read = stdin.read_line(&mut line)?;

        if bytes_read == 0 {
            if !pending_input.is_empty() {
                run_repl(std::mem::take(&mut pending_input));
            }
            break;
        }

        run_repl_line(&mut pending_input, trim_repl_line(&line));

        if pending_input.is_empty() {
            clear_error();
            clear_runtime_error();
        }
    }

    Ok(())
}

fn run(source: impl Into<String>) {
    // TODO(perf): This still materializes a full AST before evaluation.
    // A bytecode VM or lower-level IR could cut allocation and traversal
    // overhead further down the line.
    let Some(program) = parse_program(source) else {
        return;
    };

    resolve_and_interpret_statements(&program);
}

// REPL input may be either a full statement or a bare expression whose value
// should be echoed back to the user.
fn run_repl(source: impl Into<String>) {
    // TODO(perf): REPL execution still materializes a full AST before
    // evaluation. A bytecode VM or lower-level IR could trim that remaining
    // frontend overhead later on.
    let source = source.into();

    match classify_repl_source(&source) {
        ReplInput::Empty => {}
        ReplInput::Expression => {
            let Some(expr) = parse_repl_expression(source) else {
                return;
            };
            resolve_and_interpret_expression(&expr);
        }
        ReplInput::Program => {
            let Some(program) = parse_program(source) else {
                return;
            };

            resolve_and_interpret_statements(&program);
        }
    }
}

fn repl_prompt(pending_input: &str) -> &'static str {
    if pending_input.is_empty() {
        "> "
    } else {
        "... "
    }
}

fn trim_repl_line(line: &str) -> &str {
    line.trim_end_matches(&['\r', '\n'][..])
}

fn run_repl_line(pending_input: &mut String, line: &str) {
    append_repl_line(pending_input, line);

    if should_buffer_repl_input(pending_input) {
        return;
    }

    run_repl(std::mem::take(pending_input));
}

fn append_repl_line(pending_input: &mut String, line: &str) {
    if pending_input.is_empty() {
        pending_input.push_str(line);
    } else {
        pending_input.push('\n');
        pending_input.push_str(line);
    }
}

fn should_buffer_repl_input(source: &str) -> bool {
    if source.trim().is_empty() {
        return false;
    }

    if has_unclosed_repl_lexical_context(source) {
        return true;
    }

    repl_summary_needs_more_input(&summarize_repl_tokens(source))
}

fn has_unclosed_repl_lexical_context(source: &str) -> bool {
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut in_block_comment = false;

    while let Some(ch) = chars.next() {
        if in_string {
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                for comment_char in chars.by_ref() {
                    if comment_char == '\n' {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                in_block_comment = true;
            }
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            _ => {}
        }
    }

    in_string || in_block_comment || paren_depth > 0 || brace_depth > 0
}

#[derive(Default)]
struct ReplTokenSummary {
    is_empty: bool,
    first: Option<TokenType>,
    last_significant: Option<TokenType>,
    pending_then_branches: usize,
}

fn summarize_repl_tokens(source: &str) -> ReplTokenSummary {
    let mut scanner = Scanner::new(source);
    let mut summary = ReplTokenSummary {
        is_empty: true,
        ..ReplTokenSummary::default()
    };

    loop {
        let token = scanner.next_token();

        if token.type_ == TokenType::Eof {
            return summary;
        }

        summary.is_empty = false;
        summary.first.get_or_insert(token.type_);
        summary.last_significant = Some(token.type_);

        match token.type_ {
            TokenType::Question => summary.pending_then_branches += 1,
            TokenType::Colon => {
                summary.pending_then_branches = summary.pending_then_branches.saturating_sub(1);
            }
            _ => {}
        }
    }
}

fn parse_program(source: impl Into<String>) -> Option<ParsedProgram> {
    let mut parser = Parser::new(source);
    let statements = parser.parse();

    if stop_after_error() {
        None
    } else {
        Some(statements)
    }
}

fn parse_repl_expression(source: impl Into<String>) -> Option<ParsedExpression> {
    let mut parser = Parser::new(source);
    let expr = parser.parse_expression_input()?;

    if stop_after_error() { None } else { Some(expr) }
}

fn resolve_and_interpret_statements(statements: &ParsedProgram) {
    with_interpreter(|interpreter| {
        interpreter.clear_resolved_bindings();

        if resolve_statements(interpreter, statements)
            && let Err(error) = interpreter.interpret(statements)
        {
            runtime_error(&error.token, &error.message);
        }

        interpreter.clear_resolved_bindings();
    });
}

fn resolve_and_interpret_expression(expr: &ParsedExpression) {
    with_interpreter(|interpreter| {
        interpreter.clear_resolved_bindings();

        if resolve_expression(interpreter, expr) {
            match interpreter.interpret_expression(expr) {
                Ok(value) => println!("{value}"),
                Err(error) => runtime_error(&error.token, &error.message),
            }
        }

        interpreter.clear_resolved_bindings();
    });
}

fn with_interpreter<R>(f: impl FnOnce(&Interpreter) -> R) -> R {
    INTERPRETER.with(|interpreter| {
        let interpreter = interpreter.borrow();
        f(&interpreter)
    })
}

fn resolve_statements(interpreter: &Interpreter, statements: &[Stmt]) -> bool {
    let mut resolver = Resolver::new(interpreter);
    resolver.resolve_statements(statements).is_ok() && !had_error()
}

fn resolve_expression(interpreter: &Interpreter, expr: &Expr) -> bool {
    let mut resolver = Resolver::new(interpreter);
    resolver.resolve_expression(expr).is_ok() && !had_error()
}

fn stop_after_error() -> bool {
    had_error()
}

enum ReplInput {
    Empty,
    Expression,
    Program,
}

fn classify_repl_source(source: &str) -> ReplInput {
    classify_repl_summary(&summarize_repl_tokens(source))
}

fn classify_repl_summary(summary: &ReplTokenSummary) -> ReplInput {
    if summary.is_empty {
        ReplInput::Empty
    } else if should_eval_repl_summary_expression(summary) {
        ReplInput::Expression
    } else {
        ReplInput::Program
    }
}

// Use a small token-summary heuristic so the REPL can accept bare expressions
// without first trying statement parsing and emitting a spurious syntax error.
fn should_eval_repl_summary_expression(summary: &ReplTokenSummary) -> bool {
    if starts_with_statement_type(summary.first) {
        return false;
    }

    !ends_with_semicolon_type(summary.last_significant)
}

fn starts_with_statement_type(first: Option<TokenType>) -> bool {
    matches!(
        first,
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

fn repl_summary_needs_more_input(summary: &ReplTokenSummary) -> bool {
    !summary.is_empty
        && (summary.pending_then_branches > 0
            || ends_with_repl_continuation_type(summary.last_significant)
            || starts_incomplete_multiline_statement_type(summary.first, summary.last_significant))
}

fn ends_with_repl_continuation_type(last_significant: Option<TokenType>) -> bool {
    matches!(
        last_significant,
        Some(
            TokenType::Bang
                | TokenType::BangEqual
                | TokenType::Comma
                | TokenType::Colon
                | TokenType::Dot
                | TokenType::Equal
                | TokenType::EqualEqual
                | TokenType::Greater
                | TokenType::GreaterEqual
                | TokenType::Less
                | TokenType::LessEqual
                | TokenType::Minus
                | TokenType::Or
                | TokenType::And
                | TokenType::Plus
                | TokenType::Question
                | TokenType::Slash
                | TokenType::Star
        )
    )
}

fn starts_incomplete_multiline_statement_type(
    first: Option<TokenType>,
    last_significant: Option<TokenType>,
) -> bool {
    matches!(
        first,
        Some(TokenType::Class | TokenType::For | TokenType::Fun | TokenType::If | TokenType::While)
    ) && !ends_with_semicolon_type(last_significant)
        && !ends_with_right_brace_type(last_significant)
}

fn ends_with_semicolon_type(last_significant: Option<TokenType>) -> bool {
    matches!(last_significant, Some(TokenType::Semicolon))
}

fn ends_with_right_brace_type(last_significant: Option<TokenType>) -> bool {
    matches!(last_significant, Some(TokenType::RightBrace))
}

#[cfg(test)]
mod tests;
