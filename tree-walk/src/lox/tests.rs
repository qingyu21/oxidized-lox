use super::*;
use crate::diagnostics::{
    clear_error, clear_runtime_error, had_error, had_runtime_error, runtime_error,
};
use crate::interpreter::Interpreter;
use crate::runtime::Value;
use crate::token::Token;
use std::sync::{LazyLock, Mutex};

static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[test]
fn run_marks_syntax_errors_without_runtime_errors() {
    assert_run_static_error("print ;");
}

#[test]
fn run_marks_runtime_errors_without_syntax_errors() {
    assert_run_runtime_error("1 / 0;");
}

#[test]
fn run_stops_before_execution_after_parse_error() {
    assert_run_static_error("print ; 1 / 0;");
}

#[test]
fn run_marks_unterminated_string_as_a_syntax_error() {
    assert_run_static_error("\"unterminated");
}

#[test]
fn run_marks_unterminated_block_comment_as_a_syntax_error() {
    assert_run_static_error("/* unterminated");
}

#[test]
fn run_marks_unexpected_characters_as_a_syntax_error() {
    assert_run_static_error("@");
}

#[test]
fn runtime_error_sets_only_runtime_flag() {
    with_clean_error_state(|| {
        runtime_error(&token(TokenType::Slash, "/", 7), "Division by zero.");

        assert_flags(false, true);
    });
}

#[test]
fn repl_style_runs_share_the_same_interpreter_state() {
    with_clean_error_state(|| {
        run_and_reset_flags("var beverage = \"tea\";");

        run("beverage;");

        assert_flags(false, false);
    });
}

#[test]
fn repl_evaluates_bare_expressions() {
    assert_repl_runtime_error("1 / 0");
}

#[test]
fn repl_still_executes_semicolon_terminated_statements() {
    with_clean_error_state(|| {
        run_repl_and_reset_flags("var beverage = \"tea\";");

        run_repl("beverage");

        assert_flags(false, false);
    });
}

#[test]
fn repl_clears_resolved_bindings_after_each_input() {
    with_clean_error_state(|| {
        run_repl_and_reset_flags("var beverage = \"tea\";");
        assert_eq!(with_interpreter(Interpreter::resolved_bindings_len), 0);

        run_repl_and_reset_flags("beverage");
        assert_eq!(with_interpreter(Interpreter::resolved_bindings_len), 0);
    });
}

#[test]
fn repl_keeps_expression_statements_as_statements() {
    assert_repl_runtime_error("1 / 0;");
}

#[test]
fn run_marks_too_many_call_arguments_as_a_syntax_error() {
    let arguments = std::iter::repeat_n("1", 256).collect::<Vec<_>>().join(", ");
    let source = format!("clock({arguments});");
    assert_run_static_error(&source);
}

#[test]
fn run_marks_too_many_function_parameters_as_a_syntax_error() {
    let params = (0..256)
        .map(|index| format!("p{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!("fun tooMany({params}) {{}}");
    assert_run_static_error(&source);
}

#[test]
fn run_marks_top_level_return_as_a_syntax_error() {
    assert_run_static_error("return 1;");
}

#[test]
fn run_marks_local_self_initializer_as_a_syntax_error() {
    assert_run_static_error("{ var a = a; }");
}

#[test]
fn duplicate_local_variable_in_same_scope_is_a_resolver_error() {
    assert_run_static_error("{ var a = 1; var a = 2; }");
}

#[test]
fn duplicate_function_parameter_name_is_a_resolver_error() {
    assert_run_static_error("fun bad(a, a) {}");
}

#[test]
fn run_marks_this_outside_a_class_as_a_resolution_error() {
    assert_run_static_error("print this;");
}

#[test]
fn run_marks_this_inside_a_non_method_function_as_a_resolution_error() {
    assert_run_static_error("fun notAMethod() { print this; }");
}

#[test]
fn run_marks_super_outside_a_class_as_a_resolution_error() {
    assert_run_static_error("super.notEvenInAClass();");
}

#[test]
fn run_marks_super_in_a_class_without_a_superclass_as_a_resolution_error() {
    assert_run_static_error("class Eclair { cook() { super.cook(); } }");
}

#[test]
fn run_marks_returning_a_value_from_an_initializer_as_a_resolution_error() {
    assert_run_static_error("class Foo { init() { return \"something else\"; } }");
}

#[test]
fn run_marks_inheriting_from_self_as_a_resolution_error() {
    assert_run_static_error("class Oops < Oops {}");
}

#[test]
fn run_marks_unused_local_variable_as_a_resolution_error() {
    assert_run_static_error("{ var beverage = \"tea\"; }");
}

#[test]
fn run_allows_used_local_variables() {
    assert_run_success("{ var beverage = \"tea\"; print beverage; }");
}

#[test]
fn run_treats_closure_reads_as_using_the_enclosing_local() {
    assert_run_success(
        "fun outer() {
           var beverage = \"tea\";
           fun inner() {
             print beverage;
           }
         }",
    );
}

#[test]
fn run_does_not_report_unused_function_parameters() {
    assert_run_success("fun show(beverage) {}");
}

#[test]
fn run_still_marks_assignment_only_locals_as_unused() {
    assert_run_static_error("{ var count = 1; count = 2; }");
}

#[test]
fn bare_expressions_are_detected_in_the_repl() {
    assert!(matches!(
        classify_repl_source("1 + 2"),
        ReplInput::Expression
    ));
}

#[test]
fn semicolon_terminated_inputs_are_not_treated_as_bare_repl_expressions() {
    assert!(matches!(classify_repl_source("1 + 2;"), ReplInput::Program));
}

#[test]
fn statement_started_inputs_are_not_treated_as_bare_repl_expressions() {
    let cases = [
        "print 1",
        "var beverage = \"tea\"",
        "{ print 1; }",
        "if (true) print 1;",
        "while (true) break;",
        "for (;;) break;",
        "break",
        "fun greet() {}",
        "class Bagel {}",
        "return 1",
    ];

    for source in cases {
        assert!(
            matches!(classify_repl_source(source), ReplInput::Program),
            "expected `{source}` to be treated as a statement"
        );
    }
}

#[test]
fn repl_buffers_incomplete_multiline_inputs() {
    let cases = [
        "if (true)",
        "{ print 1;",
        "fun greet(name)",
        "class Bagel",
        "1 +",
        "\"tea",
        "/* block comment",
    ];

    for source in cases {
        assert!(
            should_buffer_repl_input(source),
            "expected `{source}` to keep waiting for more REPL input"
        );
    }
}

#[test]
fn repl_runs_multiline_if_blocks_once_the_block_is_closed() {
    with_clean_error_state(|| {
        run_repl_and_reset_flags("var beverage = \"tea\";");

        let mut pending = String::new();
        run_repl_line(&mut pending, "if (true) {");
        assert_eq!(pending, "if (true) {");
        assert_flags(false, false);

        run_repl_line(&mut pending, "  beverage = \"coffee\";");
        assert_eq!(pending, "if (true) {\n  beverage = \"coffee\";");
        assert_flags(false, false);

        run_repl_line(&mut pending, "}");
        assert!(pending.is_empty());
        assert_flags(false, false);
        assert_eq!(
            evaluate_repl_expression("beverage"),
            Value::String("coffee".into())
        );
    });
}

#[test]
fn repl_runs_multiline_function_declarations_once_the_body_is_closed() {
    with_clean_error_state(|| {
        let mut pending = String::new();

        run_repl_line(&mut pending, "fun greet(name) {");
        assert_eq!(pending, "fun greet(name) {");
        assert_flags(false, false);

        run_repl_line(&mut pending, "  return \"hi, \" + name;");
        assert_eq!(pending, "fun greet(name) {\n  return \"hi, \" + name;");
        assert_flags(false, false);

        run_repl_line(&mut pending, "}");
        assert!(pending.is_empty());
        assert_flags(false, false);
        assert_eq!(
            evaluate_repl_expression("greet(\"lox\")"),
            Value::String("hi, lox".into())
        );
    });
}

#[test]
fn repl_waits_to_run_bare_expressions_until_they_are_complete() {
    with_clean_error_state(|| {
        let mut pending = String::new();

        run_repl_line(&mut pending, "1 /");
        assert_eq!(pending, "1 /");
        assert_flags(false, false);

        run_repl_line(&mut pending, "0");
        assert!(pending.is_empty());
        assert_flags(false, true);
    });
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
    // Tests replace the entire interpreter so globals, environments, and
    // resolver caches all return to a clean REPL state at once.
    INTERPRETER.with(|interpreter| {
        *interpreter.borrow_mut() = Interpreter::new();
    });
}

fn assert_run_success(source: &str) {
    assert_run_flags(source, false, false);
}

fn assert_run_static_error(source: &str) {
    assert_run_flags(source, true, false);
}

fn assert_run_runtime_error(source: &str) {
    assert_run_flags(source, false, true);
}

fn assert_run_flags(source: &str, error: bool, runtime_error: bool) {
    with_clean_error_state(|| {
        run(source);
        assert_flags(error, runtime_error);
    });
}

fn assert_repl_runtime_error(source: &str) {
    assert_repl_flags(source, false, true);
}

fn assert_repl_flags(source: &str, error: bool, runtime_error: bool) {
    with_clean_error_state(|| {
        run_repl(source);
        assert_flags(error, runtime_error);
    });
}

fn assert_flags(error: bool, runtime_error: bool) {
    assert_eq!(had_error(), error);
    assert_eq!(had_runtime_error(), runtime_error);
}

fn run_and_reset_flags(source: &str) {
    run(source);
    clear_error();
    clear_runtime_error();
}

fn run_repl_and_reset_flags(source: &str) {
    run_repl(source);
    clear_error();
    clear_runtime_error();
}

fn evaluate_repl_expression(source: &str) -> Value {
    let expr = parse_repl_expression(source).expect("test expression should parse");

    with_interpreter(|interpreter| {
        interpreter.clear_resolved_bindings();
        assert!(resolve_expression(interpreter, &expr));
        let value = interpreter
            .interpret_expression(&expr)
            .expect("test expression should evaluate");
        interpreter.clear_resolved_bindings();
        value
    })
}
