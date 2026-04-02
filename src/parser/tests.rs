use super::Parser;
use crate::ast_printer::AstPrinter;
use crate::scanner::Scanner;
use crate::stmt::Stmt;

#[test]
fn parses_binary_precedence() {
    assert_eq!(parse_expression_to_string("1 + 2 * 3;"), "(+ 1 (* 2 3))");
}

#[test]
fn parses_comma_with_lowest_precedence() {
    assert_eq!(
        parse_expression_to_string("1 + 2, 3 * 4;"),
        "(, (+ 1 2) (* 3 4))"
    );
}

#[test]
fn parses_comma_as_left_associative() {
    assert_eq!(parse_expression_to_string("1, 2, 3;"), "(, (, 1 2) 3)");
}

#[test]
fn parses_conditional_as_right_associative() {
    assert_eq!(
        parse_expression_to_string("false ? 1 : true ? 2 : 3;"),
        "(?: false 1 (?: true 2 3))"
    );
}

#[test]
fn parses_full_expression_in_then_branch() {
    assert_eq!(
        parse_expression_to_string("true ? 1, 2 : 3;"),
        "(?: true (, 1 2) 3)"
    );
}

#[test]
fn parses_logical_or_and_and_with_expected_precedence() {
    assert_eq!(
        parse_expression_to_string("true or false and nil;"),
        "(or true (and false nil))"
    );
}

#[test]
fn parses_logical_or_as_left_associative() {
    assert_eq!(
        parse_expression_to_string("a or b or c;"),
        "(or (or a b) c)"
    );
}

#[test]
fn parses_logical_and_as_left_associative() {
    assert_eq!(
        parse_expression_to_string("a and b and c;"),
        "(and (and a b) c)"
    );
}

#[test]
fn parses_conditional_after_logical_or() {
    assert_eq!(
        parse_expression_to_string("false or true ? 1 : 2;"),
        "(?: (or false true) 1 2)"
    );
}

#[test]
fn parses_assignment_after_logical_or() {
    assert_eq!(
        parse_expression_to_string("beverage = false or true;"),
        "(= beverage (or false true))"
    );
}

#[test]
fn parses_unary_and_grouping() {
    assert_eq!(
        parse_expression_to_string("!(false == true);"),
        "(! (group (== false true)))"
    );
}

#[test]
fn parses_grouped_binary_expression() {
    assert_eq!(
        parse_expression_to_string("(1 + 2) * 3;"),
        "(* (group (+ 1 2)) 3)"
    );
}

#[test]
fn parses_call_with_arguments() {
    assert_eq!(
        parse_expression_to_string("average(1, 2);"),
        "(call average 1 2)"
    );
}

#[test]
fn parses_zero_argument_and_chained_calls() {
    assert_eq!(
        parse_expression_to_string("getCallback()();"),
        "(call (call getCallback))"
    );
}

#[test]
fn parses_call_with_higher_precedence_than_unary() {
    assert_eq!(parse_expression_to_string("-clock();"), "(- (call clock))");
}

#[test]
fn parses_grouped_comma_expression_as_a_single_call_argument() {
    assert_eq!(
        parse_expression_to_string("log((1, 2));"),
        "(call log (group (, 1 2)))"
    );
}

#[test]
fn parses_print_statement() {
    assert_eq!(parse_print_to_string("print 1 + 2;"), "(+ 1 2)");
}

#[test]
fn parses_break_statement_inside_a_loop() {
    let statements = parse_statements("while (true) { break; }");

    match statements.as_slice() {
        [Stmt::While { condition, body }] => {
            assert_eq!(AstPrinter.print(condition), "true");
            match body.as_ref() {
                Stmt::Block { statements } => match statements.as_slice() {
                    [Stmt::Break] => {}
                    _ => panic!("expected a single break statement inside the while block"),
                },
                _ => panic!("expected a block statement in the while body"),
            }
        }
        _ => panic!("expected a single while statement"),
    }
}

#[test]
fn parses_if_statement_without_else() {
    let statements = parse_statements("if (true) print 1;");

    match statements.as_slice() {
        [
            Stmt::If {
                condition,
                then_branch,
                else_branch: None,
            },
        ] => {
            assert_eq!(AstPrinter.print(condition), "true");
            match then_branch.as_ref() {
                Stmt::Print { expression } => {
                    assert_eq!(AstPrinter.print(expression), "1");
                }
                _ => panic!("expected a print statement in the then branch"),
            }
        }
        _ => panic!("expected a single if statement without else"),
    }
}

#[test]
fn parses_if_statement_with_else() {
    let statements = parse_statements("if (true) print 1; else print 2;");

    match statements.as_slice() {
        [
            Stmt::If {
                condition,
                then_branch,
                else_branch: Some(else_branch),
            },
        ] => {
            assert_eq!(AstPrinter.print(condition), "true");
            match then_branch.as_ref() {
                Stmt::Print { expression } => assert_eq!(AstPrinter.print(expression), "1"),
                _ => panic!("expected a print statement in the then branch"),
            }
            match else_branch.as_ref() {
                Stmt::Print { expression } => assert_eq!(AstPrinter.print(expression), "2"),
                _ => panic!("expected a print statement in the else branch"),
            }
        }
        _ => panic!("expected a single if statement with else"),
    }
}

#[test]
fn parses_while_statement_with_expression_body() {
    let statements = parse_statements("while (true) print 1;");

    match statements.as_slice() {
        [Stmt::While { condition, body }] => {
            assert_eq!(AstPrinter.print(condition), "true");
            match body.as_ref() {
                Stmt::Print { expression } => assert_eq!(AstPrinter.print(expression), "1"),
                _ => panic!("expected a print statement in the while body"),
            }
        }
        _ => panic!("expected a single while statement"),
    }
}

#[test]
fn parses_while_statement_with_block_body() {
    let statements = parse_statements("while (beverage) { print 1; }");

    match statements.as_slice() {
        [Stmt::While { condition, body }] => {
            assert_eq!(AstPrinter.print(condition), "beverage");
            match body.as_ref() {
                Stmt::Block { statements } => match statements.as_slice() {
                    [Stmt::Print { expression }] => assert_eq!(AstPrinter.print(expression), "1"),
                    _ => panic!("expected one print statement inside the while block"),
                },
                _ => panic!("expected a block statement in the while body"),
            }
        }
        _ => panic!("expected a single while statement"),
    }
}

#[test]
fn parses_for_statement_by_desugaring_to_block_and_while() {
    let statements = parse_statements("for (var i = 0; i < 3; i = i + 1) print i;");

    match statements.as_slice() {
        [
            Stmt::Block {
                statements: outer_statements,
            },
        ] => match outer_statements.as_slice() {
            [
                Stmt::Var {
                    name,
                    initializer: Some(initializer),
                },
                Stmt::While { condition, body },
            ] => {
                assert_eq!(name.lexeme, "i");
                assert_eq!(AstPrinter.print(initializer), "0");
                assert_eq!(AstPrinter.print(condition), "(< i 3)");

                match body.as_ref() {
                    Stmt::Block { statements } => match statements.as_slice() {
                        [
                            Stmt::Print { expression },
                            Stmt::Expression {
                                expression: increment,
                            },
                        ] => {
                            assert_eq!(AstPrinter.print(expression), "i");
                            assert_eq!(AstPrinter.print(increment), "(= i (+ i 1))");
                        }
                        _ => panic!(
                            "expected the while body to contain the original body plus increment"
                        ),
                    },
                    _ => panic!("expected the desugared while body to be a block"),
                }
            }
            _ => panic!("expected initializer plus while loop in the outer block"),
        },
        _ => panic!("expected the for loop to desugar to a single block statement"),
    }
}

#[test]
fn parses_for_statement_without_clauses_as_infinite_while() {
    let statements = parse_statements("for (;;) print 1;");

    match statements.as_slice() {
        [Stmt::While { condition, body }] => {
            assert_eq!(AstPrinter.print(condition), "true");
            match body.as_ref() {
                Stmt::Print { expression } => assert_eq!(AstPrinter.print(expression), "1"),
                _ => panic!("expected the while body to be the original loop body"),
            }
        }
        _ => panic!("expected clause-free for loop to desugar directly to while"),
    }
}

#[test]
fn dangling_else_binds_to_the_nearest_if() {
    let statements = parse_statements("if (first) if (second) print 1; else print 2;");

    match statements.as_slice() {
        [
            Stmt::If {
                condition,
                then_branch,
                else_branch: None,
            },
        ] => {
            assert_eq!(AstPrinter.print(condition), "first");
            match then_branch.as_ref() {
                Stmt::If {
                    condition,
                    then_branch,
                    else_branch: Some(else_branch),
                } => {
                    assert_eq!(AstPrinter.print(condition), "second");
                    match then_branch.as_ref() {
                        Stmt::Print { expression } => assert_eq!(AstPrinter.print(expression), "1"),
                        _ => panic!("expected a print statement in the inner then branch"),
                    }
                    match else_branch.as_ref() {
                        Stmt::Print { expression } => assert_eq!(AstPrinter.print(expression), "2"),
                        _ => panic!("expected a print statement in the inner else branch"),
                    }
                }
                _ => panic!("expected the outer then branch to be another if statement"),
            }
        }
        _ => panic!("expected the dangling else to bind to the inner if"),
    }
}

#[test]
fn parses_block_statement() {
    let statements = parse_statements("{ var beverage = 1; print beverage; }");

    match statements.as_slice() {
        [
            Stmt::Block {
                statements: block_statements,
            },
        ] => match block_statements.as_slice() {
            [
                Stmt::Var {
                    name,
                    initializer: Some(initializer),
                },
                Stmt::Print { expression },
            ] => {
                assert_eq!(name.lexeme, "beverage");
                assert_eq!(AstPrinter.print(initializer), "1");
                assert_eq!(AstPrinter.print(expression), "beverage");
            }
            _ => panic!("expected a variable declaration followed by a print inside the block"),
        },
        _ => panic!("expected a single block statement"),
    }
}

#[test]
fn parses_var_declaration_with_initializer() {
    let statements = parse_statements("var beverage = 1 + 2;");

    match statements.as_slice() {
        [
            Stmt::Var {
                name,
                initializer: Some(initializer),
            },
        ] => {
            assert_eq!(name.lexeme, "beverage");
            assert_eq!(AstPrinter.print(initializer), "(+ 1 2)");
        }
        _ => panic!("expected a single variable declaration with an initializer"),
    }
}

#[test]
fn parses_var_declaration_without_initializer() {
    let statements = parse_statements("var beverage;");

    match statements.as_slice() {
        [
            Stmt::Var {
                name,
                initializer: None,
            },
        ] => {
            assert_eq!(name.lexeme, "beverage");
        }
        _ => panic!("expected a single variable declaration without an initializer"),
    }
}

#[test]
fn parses_variable_expression_statement() {
    assert_eq!(parse_expression_to_string("beverage;"), "beverage");
}

#[test]
fn parses_assignment_expression_statement() {
    assert_eq!(
        parse_expression_to_string("beverage = 1;"),
        "(= beverage 1)"
    );
}

#[test]
fn parses_assignment_as_right_associative() {
    assert_eq!(parse_expression_to_string("a = b = 1;"), "(= a (= b 1))");
}

#[test]
fn discards_factor_expression_after_missing_left_operand() {
    assert_parse_error_consumes_to_end("+ 1 * 2;");
}

#[test]
fn discards_comparison_expression_after_missing_left_operand() {
    assert_parse_error_consumes_to_end("== 1 < 2;");
}

#[test]
fn discards_conditional_expression_after_missing_left_comma() {
    assert_parse_error_consumes_to_end(", false ? 1 : 2;");
}

#[test]
fn discards_logical_expression_after_missing_left_operand() {
    assert_parse_error_consumes_to_end("or false and true;");
}

#[test]
fn synchronizes_to_next_statement_after_error() {
    let tokens = Scanner::new("print 1 + ; print 2;").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    let expr = match statements.as_slice() {
        [Stmt::Print { expression }] => expression,
        _ => panic!("expected the parser to recover to the next print statement"),
    };

    assert_eq!(AstPrinter.print(expr), "2");
}

#[test]
fn synchronizes_to_var_declaration_after_error() {
    let tokens = Scanner::new("print 1 + ; var beverage = \"tea\";").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    match statements.as_slice() {
        [
            Stmt::Var {
                name,
                initializer: Some(initializer),
            },
        ] => {
            assert_eq!(name.lexeme, "beverage");
            assert_eq!(AstPrinter.print(initializer), "tea");
        }
        _ => panic!("expected the parser to recover to the next variable declaration"),
    }
}

#[test]
fn reports_break_outside_loop_and_recovers() {
    let tokens = Scanner::new("break; print 1;").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    match statements.as_slice() {
        [Stmt::Print { expression }] => assert_eq!(AstPrinter.print(expression), "1"),
        _ => panic!("expected the parser to recover after break outside a loop"),
    }
}

#[test]
fn synchronizes_to_while_statement_after_error() {
    let tokens = Scanner::new("print 1 + ; while (true) print 2;").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    match statements.as_slice() {
        [Stmt::While { condition, body }] => {
            assert_eq!(AstPrinter.print(condition), "true");
            match body.as_ref() {
                Stmt::Print { expression } => assert_eq!(AstPrinter.print(expression), "2"),
                _ => panic!("expected a print statement in the recovered while body"),
            }
        }
        _ => panic!("expected the parser to recover to the next while statement"),
    }
}

#[test]
fn synchronizes_to_break_statement_inside_a_loop_block() {
    let tokens = Scanner::new("while (true) { print 1 + ; break; }").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    match statements.as_slice() {
        [Stmt::While { body, .. }] => match body.as_ref() {
            Stmt::Block { statements } => match statements.as_slice() {
                [Stmt::Break] => {}
                _ => panic!("expected recovery to preserve the following break statement"),
            },
            _ => panic!("expected a block statement in the while body"),
        },
        _ => panic!("expected a single while statement after recovery"),
    }
}

#[test]
fn synchronizes_to_for_statement_after_error() {
    let tokens = Scanner::new("print 1 + ; for (;;) print 2;").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    match statements.as_slice() {
        [Stmt::While { condition, body }] => {
            assert_eq!(AstPrinter.print(condition), "true");
            match body.as_ref() {
                Stmt::Print { expression } => assert_eq!(AstPrinter.print(expression), "2"),
                _ => panic!("expected a print statement in the recovered for body"),
            }
        }
        _ => panic!("expected the parser to recover to the next for statement"),
    }
}

#[test]
fn reports_invalid_assignment_target_and_recovers() {
    let tokens = Scanner::new("a + b = c; print 1;").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    match statements.as_slice() {
        [
            Stmt::Expression { expression },
            Stmt::Print {
                expression: printed,
            },
        ] => {
            assert_eq!(AstPrinter.print(expression), "(+ a b)");
            assert_eq!(AstPrinter.print(printed), "1");
        }
        _ => panic!("expected the parser to continue after an invalid assignment target"),
    }
}

#[test]
fn keeps_valid_statements_before_and_after_an_invalid_one() {
    let tokens = Scanner::new("print 1; print ; print 2;").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    let printed = statements
        .iter()
        .map(|stmt| match stmt {
            Stmt::Print { expression } => AstPrinter.print(expression),
            _ => panic!("expected only print statements"),
        })
        .collect::<Vec<_>>();

    assert_eq!(printed, vec!["1".to_string(), "2".to_string()]);
}

#[test]
fn synchronizes_to_next_print_after_missing_semicolon() {
    let tokens = Scanner::new("print 1 print 2;").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    let printed = statements
        .iter()
        .map(|stmt| match stmt {
            Stmt::Print { expression } => AstPrinter.print(expression),
            _ => panic!("expected only print statements"),
        })
        .collect::<Vec<_>>();

    assert_eq!(printed, vec!["2".to_string()]);
}

#[test]
fn synchronizes_after_missing_right_paren() {
    let tokens = Scanner::new("print (1 + 2; print 3;").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    let printed = statements
        .iter()
        .map(|stmt| match stmt {
            Stmt::Print { expression } => AstPrinter.print(expression),
            _ => panic!("expected only print statements"),
        })
        .collect::<Vec<_>>();

    assert_eq!(printed, vec!["3".to_string()]);
}

#[test]
fn synchronizes_to_next_expression_statement_after_missing_semicolon() {
    let tokens = Scanner::new("print 1 2;").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    let expr = match statements.as_slice() {
        [Stmt::Expression { expression }] => expression,
        _ => panic!("expected recovery to the next expression statement"),
    };

    assert_eq!(AstPrinter.print(expr), "2");
}

#[test]
fn synchronizes_to_supported_expression_statement_starts() {
    let cases = [
        ("false", "false"),
        ("true", "true"),
        ("nil", "nil"),
        ("2", "2"),
        ("\"lox\"", "lox"),
        ("!false", "(! false)"),
    ];

    for (next_statement, expected) in cases {
        let source = format!("print 1 {next_statement};");
        assert_eq!(
            recover_to_expression_statement_string(&source),
            expected,
            "failed for {source}"
        );
    }
}

#[test]
fn synchronizes_to_grouped_expression_statement_after_operator_error() {
    assert_eq!(
        recover_to_expression_statement_string("print 1 + ; (2 + 3);"),
        "(group (+ 2 3))"
    );
}

#[test]
fn synchronizes_to_minus_started_expression_statement() {
    assert_eq!(
        recover_to_expression_statement_string("print (1 + ) -2;"),
        "(- 2)"
    );
}

#[test]
fn synchronizes_within_block_without_skipping_the_closing_brace() {
    let tokens = Scanner::new("{ print 1 + ; } print 2;").scan_tokens();
    let mut parser = Parser::new(tokens);
    let statements = parser.parse();

    match statements.as_slice() {
        [
            Stmt::Block {
                statements: block_statements,
            },
            Stmt::Print { expression },
        ] => {
            assert!(block_statements.is_empty());
            assert_eq!(AstPrinter.print(expression), "2");
        }
        _ => panic!("expected recovery to preserve the enclosing block boundary"),
    }
}

fn parse_expression_to_string(source: &str) -> String {
    let statements = parse_statements(source);
    let expr = match statements.as_slice() {
        [Stmt::Expression { expression }] => expression,
        _ => panic!("expected a single expression statement"),
    };

    AstPrinter.print(expr)
}

fn parse_print_to_string(source: &str) -> String {
    let statements = parse_statements(source);
    let expr = match statements.as_slice() {
        [Stmt::Print { expression }] => expression,
        _ => panic!("expected a single print statement"),
    };

    AstPrinter.print(expr)
}

fn assert_parse_error_consumes_to_end(source: &str) {
    let tokens = Scanner::new(source).scan_tokens();
    let mut parser = Parser::new(tokens);
    let _ = parser.parse();
    assert!(parser.is_at_end());
}

fn recover_to_expression_statement_string(source: &str) -> String {
    let statements = parse_statements(source);

    match statements.as_slice() {
        [Stmt::Expression { expression }] => AstPrinter.print(expression),
        _ => panic!("expected recovery to a single expression statement"),
    }
}

fn parse_statements(source: &str) -> Vec<Stmt> {
    let tokens = Scanner::new(source).scan_tokens();
    let mut parser = Parser::new(tokens);
    parser.parse()
}
