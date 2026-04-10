use std::rc::Rc;

use crate::{
    interpreter::Interpreter,
    runtime::Value,
    stmt::Stmt,
    test_support::{parse_statements, resolve_statements},
};

#[test]
fn bound_methods_share_the_same_function_definition() {
    let statements = parse_statements(
        "class Greeter {
           greet(name) {
             return \"hi, \" + name;
           }
         }

         var greeter = Greeter();
         Greeter;
         greeter;",
    );
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
    let method = class
        .find_method("greet")
        .expect("class should expose the declared greet method");
    let first = method.bind(instance.clone());
    let second = method.bind(instance);

    assert!(
        Rc::ptr_eq(&first.code, &second.code),
        "bound methods should share their immutable function definition"
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
