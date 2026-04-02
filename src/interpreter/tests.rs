use super::{Interpreter, Value};
use crate::expr::Expr;
use crate::parser::Parser;
use crate::scanner::Scanner;
use crate::stmt::Stmt;
use crate::token::{Literal, Token, TokenType};

#[test]
fn evaluates_numeric_expression() {
    assert_eq!(interpret("1 + 2 * 3"), Value::Number(7.0));
}

#[test]
fn concatenates_strings_with_plus() {
    assert_eq!(
        interpret("\"lox\" + \"!\""),
        Value::String("lox!".to_string())
    );
}

#[test]
fn concatenates_string_and_number_with_plus() {
    assert_eq!(
        interpret("\"scone\" + 4"),
        Value::String("scone4".to_string())
    );
}

#[test]
fn concatenates_number_and_string_with_plus() {
    assert_eq!(
        interpret("4 + \"scone\""),
        Value::String("4scone".to_string())
    );
}

#[test]
fn evaluates_truthiness_for_bang() {
    assert_eq!(interpret("!nil"), Value::Bool(true));
    assert_eq!(interpret("!0"), Value::Bool(false));
}

#[test]
fn evaluates_equality() {
    assert_eq!(interpret("1 == 1"), Value::Bool(true));
    assert_eq!(interpret("nil != false"), Value::Bool(true));
}

#[test]
fn evaluates_conditional_expression() {
    assert_eq!(interpret("false ? 1 : 2"), Value::Number(2.0));
}

#[test]
fn evaluates_logical_or_and_and() {
    assert_eq!(
        interpret("nil or \"yes\""),
        Value::String("yes".to_string())
    );
    assert_eq!(interpret("\"hi\" or 2"), Value::String("hi".to_string()));
    assert_eq!(interpret("nil and \"yes\""), Value::Nil);
    assert_eq!(interpret("\"hi\" and 2"), Value::Number(2.0));
}

#[test]
fn logical_and_short_circuits_unselected_right_operand() {
    assert_eq!(interpret("false and 1 / 0"), Value::Bool(false));
}

#[test]
fn logical_or_short_circuits_unselected_right_operand() {
    assert_eq!(interpret("true or 1 / 0"), Value::Bool(true));
}

#[test]
fn calls_native_clock_function() {
    let value = evaluate_result("clock()").expect("clock() should be callable");

    match value {
        Value::Number(value) => assert!(value >= 0.0),
        _ => panic!("expected clock() to return a number"),
    }
}

#[test]
fn reports_runtime_error_for_non_callable_callee() {
    let error = evaluate_result("\"totally not a function\"()")
        .expect_err("strings should not be callable values");
    assert_eq!(error.message, "Can only call functions and classes.");
}

#[test]
fn call_evaluates_arguments_before_arity_checks() {
    let error = evaluate_result("clock(1 / 0)")
        .expect_err("arguments should be evaluated before the call is attempted");
    assert_eq!(error.message, "Division by zero.");
}

#[test]
fn reports_runtime_error_for_wrong_call_arity() {
    let error =
        evaluate_result("clock(1)").expect_err("clock() should reject unexpected arguments");
    assert_eq!(error.message, "Expected 0 arguments but got 1.");
}

#[test]
fn executes_if_then_branch_when_condition_is_truthy() {
    let statements =
        parse_statements("var beverage = \"before\";\nif (true) beverage = \"after\";\nbeverage;");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());
    assert!(interpreter.execute(&statements[1]).is_ok());

    let value = match &statements[2] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("truthy if branch should update the variable"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::String("after".to_string()));
}

#[test]
fn executes_if_else_branch_when_condition_is_falsey() {
    let statements = parse_statements(
        "var beverage = \"before\";\nif (false) beverage = \"then\"; else beverage = \"else\";\nbeverage;",
    );
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());
    assert!(interpreter.execute(&statements[1]).is_ok());

    let value = match &statements[2] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("falsey if branch should execute the else branch"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::String("else".to_string()));
}

#[test]
fn executes_while_loop_until_condition_becomes_false() {
    let statements =
        parse_statements("var count = 0;\nwhile (count < 3) count = count + 1;\ncount;");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());
    assert!(interpreter.execute(&statements[1]).is_ok());

    let value = match &statements[2] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("while loop should keep updating the variable"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::Number(3.0));
}

#[test]
fn skips_while_body_when_condition_is_falsey() {
    let statements = parse_statements("while (false) print missing;");
    let interpreter = Interpreter::new();

    assert!(
        interpreter.execute(&statements[0]).is_ok(),
        "false condition should skip the erroneous while body"
    );
}

#[test]
fn break_exits_the_nearest_enclosing_loop() {
    let statements = parse_statements(
        "var count = 0;\nwhile (true) { count = count + 1; if (count == 3) { { break; } } }\ncount;",
    );
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());
    assert!(interpreter.execute(&statements[1]).is_ok());

    let value = match &statements[2] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("break should exit the loop after the nested if/block"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::Number(3.0));
}

#[test]
fn executes_for_loop_desugared_by_the_parser() {
    let statements = parse_statements(
        "var history = 0;\nfor (var i = 0; i < 3; i = i + 1) history = history * 10 + i;\nhistory;",
    );
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());
    assert!(interpreter.execute(&statements[1]).is_ok());

    let value = match &statements[2] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("for loop should update the outer variable through each iteration"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::Number(12.0));
}

#[test]
fn break_in_a_for_loop_skips_the_increment_clause() {
    let statements = parse_statements(
        "var count = 0;\nfor (; true; count = count + 100) { count = count + 1; break; }\ncount;",
    );
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());
    assert!(interpreter.execute(&statements[1]).is_ok());

    let value = match &statements[2] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("breaking from a for loop should bypass the increment clause"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::Number(1.0));
}

#[test]
fn for_initializer_is_scoped_to_the_loop() {
    let statements = parse_statements("for (var i = 0; i < 1; i = i + 1) print i;\ni;");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());

    let error = match &statements[1] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect_err("for initializer variable should not leak outside the loop"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(error.message, "Undefined variable 'i'.");
}

#[test]
fn for_loop_without_initializer_reuses_existing_binding() {
    let statements = parse_statements(
        "var count = 0;\nfor (; count < 3; count = count + 1) print count;\ncount;",
    );
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());
    assert!(interpreter.execute(&statements[1]).is_ok());

    let value = match &statements[2] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("for loop without initializer should keep using the existing variable"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::Number(3.0));
}

#[test]
fn skips_unselected_if_branch_runtime_errors() {
    let statements = parse_statements("if (false) print missing; else print 1;");
    let interpreter = Interpreter::new();

    assert!(
        interpreter.execute(&statements[0]).is_ok(),
        "false condition should skip the erroneous then branch"
    );
}

#[test]
fn skips_unselected_else_branch_runtime_errors() {
    let statements = parse_statements("if (true) print 1; else print missing;");
    let interpreter = Interpreter::new();

    assert!(
        interpreter.execute(&statements[0]).is_ok(),
        "true condition should skip the erroneous else branch"
    );
}

#[test]
fn evaluates_comma_expression() {
    assert_eq!(interpret("1, 2 + 3"), Value::Number(5.0));
}

#[test]
fn conditional_skips_unselected_else_branch_errors() {
    assert_eq!(interpret("true ? 1 : 1 / 0"), Value::Number(1.0));
}

#[test]
fn conditional_skips_unselected_then_branch_errors() {
    assert_eq!(interpret("false ? 1 / 0 : 2"), Value::Number(2.0));
}

#[test]
fn comma_still_evaluates_left_operand() {
    let error = evaluate_result("1 / 0, 2").expect_err("comma should evaluate its left operand");
    assert_eq!(error.message, "Division by zero.");
}

#[test]
fn reports_runtime_error_for_non_numeric_comparison() {
    let error = evaluate_result("\"a\" < \"b\"")
        .expect_err("string comparison should currently be rejected");
    assert_eq!(error.message, "Operands must be numbers.");
}

#[test]
fn reports_runtime_error_for_division_by_zero() {
    let error = evaluate_result("1 / 0").expect_err("division by zero should fail");
    assert_eq!(error.message, "Division by zero.");
}

#[test]
fn executes_var_declaration_and_reads_back_the_value() {
    let statements = parse_statements("var beverage = \"tea\";\nbeverage;");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());

    let value = match &statements[1] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("variable lookup should succeed after declaration"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::String("tea".to_string()));
}

#[test]
fn block_can_read_a_variable_from_its_enclosing_scope() {
    let statements = parse_statements("var beverage = \"tea\";\n{ beverage; }");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());
    assert!(
        interpreter.execute(&statements[1]).is_ok(),
        "inner block should be able to read the outer variable"
    );
}

#[test]
fn initializes_variables_to_nil_when_no_initializer_is_present() {
    let statements = parse_statements("var beverage;\nbeverage;");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());

    let value = match &statements[1] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("variable lookup should succeed after declaration"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::Nil);
}

#[test]
fn redeclaring_a_global_variable_overwrites_the_previous_value() {
    let statements =
        parse_statements("var beverage = \"before\";\nvar beverage = \"after\";\nbeverage;");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());
    assert!(interpreter.execute(&statements[1]).is_ok());

    let value = match &statements[2] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("variable lookup should use the most recent binding"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::String("after".to_string()));
}

#[test]
fn block_assignment_updates_an_enclosing_variable() {
    let statements =
        parse_statements("var beverage = \"before\";\n{ beverage = \"after\"; }\nbeverage;");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());
    assert!(interpreter.execute(&statements[1]).is_ok());

    let value = match &statements[2] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("assignment in an inner block should update the outer binding"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::String("after".to_string()));
}

#[test]
fn block_local_variable_shadows_outer_variable_without_leaking() {
    let statements =
        parse_statements("var beverage = \"outer\";\n{ var beverage = \"inner\"; }\nbeverage;");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());
    assert!(interpreter.execute(&statements[1]).is_ok());

    let value = match &statements[2] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("outer variable should still be visible after the block ends"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::String("outer".to_string()));
}

#[test]
fn block_local_variable_is_not_visible_after_the_block_ends() {
    let statements = parse_statements("{ var beverage = \"tea\"; }\nbeverage;");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());

    let error = match &statements[1] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect_err("block-local variables should not outlive their block"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(error.message, "Undefined variable 'beverage'.");
}

#[test]
fn assignment_updates_an_existing_variable_and_returns_the_new_value() {
    let statements =
        parse_statements("var beverage = \"before\";\nbeverage = \"after\";\nbeverage;");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute(&statements[0]).is_ok());

    let assigned = match &statements[1] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("assignment should succeed for an existing variable"),
        _ => panic!("expected an assignment expression statement"),
    };

    assert_eq!(assigned, Value::String("after".to_string()));

    let value = match &statements[2] {
        Stmt::Expression { expression } => interpreter
            .evaluate(expression)
            .expect("variable lookup should see the assigned value"),
        _ => panic!("expected a variable expression statement"),
    };

    assert_eq!(value, Value::String("after".to_string()));
}

#[test]
fn reports_runtime_error_for_assignment_to_undefined_variable() {
    let error = evaluate_result("beverage = \"tea\"")
        .expect_err("assigning an undefined variable should fail at runtime");
    assert_eq!(error.message, "Undefined variable 'beverage'.");
}

#[test]
fn reports_runtime_error_for_undefined_variable_access() {
    let error = evaluate_result("beverage")
        .expect_err("reading an undefined variable should fail at runtime");
    assert_eq!(error.message, "Undefined variable 'beverage'.");
}

#[test]
fn executes_multiple_statements_in_order() {
    let statements = parse_statements("1 + 2;\nprint 3;");
    let interpreter = Interpreter::new();

    assert!(interpreter.execute_all(&statements).is_ok());
}

#[test]
fn stops_executing_after_the_first_runtime_error() {
    let mut statements = parse_statements("1 + 2;\n1 / 0;");
    statements.push(invalid_statement(3));
    let interpreter = Interpreter::new();

    let error = interpreter
        .execute_all(&statements)
        .expect_err("execution should stop at division by zero");

    assert_eq!(error.message, "Division by zero.");
    assert_eq!(error.token.line, 2);
}

fn interpret(source: &str) -> Value {
    evaluate_result(source).expect("interpreter should successfully evaluate the test input")
}

fn parse_statements(source: &str) -> Vec<Stmt> {
    let tokens = Scanner::new(source).scan_tokens();
    let mut parser = Parser::new(tokens);
    parser.parse()
}

fn evaluate_result(source: &str) -> Result<Value, super::RuntimeError> {
    let source = format!("{source};");
    let statements = parse_statements(&source);
    let interpreter = Interpreter::new();
    let expr = match statements.as_slice() {
        [Stmt::Expression { expression }] => expression,
        _ => panic!("expected a single expression statement"),
    };

    interpreter.evaluate(expr)
}

fn invalid_statement(line: u32) -> Stmt {
    Stmt::expression(Expr::Binary {
        left: Box::new(Expr::literal(Literal::Number(1.0))),
        operator: Token::new(TokenType::Print, "print".to_string(), None, line),
        right: Box::new(Expr::literal(Literal::Number(2.0))),
    })
}
