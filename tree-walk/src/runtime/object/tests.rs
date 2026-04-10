use std::rc::Rc;

use crate::{
    interpreter::Interpreter,
    runtime::Value,
    stmt::Stmt,
    test_support::{parse_statements, resolve_statements},
};

#[test]
fn instances_share_the_declared_class_handle() {
    let statements = parse_statements("class Bagel {} var bagel = Bagel(); Bagel; bagel;");
    let interpreter = Interpreter::new();
    resolve_statements(&interpreter, &statements);

    interpreter
        .interpret(&statements[..2])
        .expect("class declaration and instance creation should succeed");

    let class = evaluate_expression_statement(&interpreter, &statements[2]);
    let instance = evaluate_expression_statement(&interpreter, &statements[3]);

    let Value::Class(class) = class else {
        panic!("expected the class expression to evaluate to a class");
    };
    let Value::Instance(instance) = instance else {
        panic!("expected the instance expression to evaluate to an instance");
    };

    assert!(
        Rc::ptr_eq(&instance.klass, &class),
        "instances should keep sharing the original class handle instead of a cloned class copy"
    );
}

fn evaluate_expression_statement(interpreter: &Interpreter, stmt: &Stmt) -> Value {
    match stmt {
        Stmt::Expression { expression } => interpreter
            .interpret_expression(expression)
            .expect("test expression should evaluate successfully"),
        _ => panic!("expected an expression statement"),
    }
}
