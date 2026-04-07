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
fn run_marks_unterminated_string_as_a_syntax_error() {
    with_clean_error_state(|| {
        run("\"unterminated");

        assert!(had_error());
        assert!(!had_runtime_error());
    });
}

#[test]
fn run_marks_unterminated_block_comment_as_a_syntax_error() {
    with_clean_error_state(|| {
        run("/* unterminated");

        assert!(had_error());
        assert!(!had_runtime_error());
    });
}

#[test]
fn run_marks_unexpected_characters_as_a_syntax_error() {
    with_clean_error_state(|| {
        run("@");

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
fn run_marks_super_outside_a_class_as_a_resolution_error() {
    with_clean_error_state(|| {
        run("super.notEvenInAClass();");

        assert!(had_error());
        assert!(!had_runtime_error());
    });
}

#[test]
fn run_marks_super_in_a_class_without_a_superclass_as_a_resolution_error() {
    with_clean_error_state(|| {
        run("class Eclair { cook() { super.cook(); } }");

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
        let tokens = Scanner::new(source).scan_tokens();
        assert!(
            !should_eval_repl_expression(&tokens),
            "expected `{source}` to be treated as a statement"
        );
    }
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
