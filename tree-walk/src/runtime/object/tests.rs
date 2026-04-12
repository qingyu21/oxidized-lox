use std::rc::Rc;

use crate::{
    interpreter::Interpreter,
    parser::Parser,
    runtime::Value,
    test_support::{parse_statements, resolve_statements},
};

#[test]
fn instances_share_the_declared_class_handle() {
    let interpreter = Interpreter::new();

    let setup = parse_statements("class Bagel {} var bagel = Bagel();");
    resolve_statements(&interpreter, &setup);
    interpreter
        .interpret(&setup)
        .expect("class declaration and instance creation should succeed");

    let class = evaluate_expression(&interpreter, "Bagel");
    let instance = evaluate_expression(&interpreter, "bagel");

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

fn evaluate_expression(interpreter: &Interpreter, source: &str) -> Value {
    let mut parser = Parser::new(source);
    let expression = parser
        .parse_expression_input()
        .expect("test expression should parse");

    interpreter
        .interpret_expression(&expression)
        .expect("test expression should evaluate successfully")
}
