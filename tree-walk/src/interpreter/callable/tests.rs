use std::rc::Rc;

use crate::{
    interpreter::Interpreter,
    parser::Parser,
    runtime::Value,
    test_support::{parse_statements, resolve_statements},
};

#[test]
fn bound_methods_share_the_same_function_definition() {
    let interpreter = Interpreter::new();

    let setup = parse_statements(
        "class Greeter {
           greet(name) {
             return \"hi, \" + name;
           }
         }

         var greeter = Greeter();",
    );
    resolve_statements(&interpreter, &setup);
    interpreter
        .interpret(&setup)
        .expect("class declaration and instance creation should succeed");

    let class = evaluate_expression(&interpreter, "Greeter");
    let instance = evaluate_expression(&interpreter, "greeter");

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

fn evaluate_expression(interpreter: &Interpreter, source: &str) -> Value {
    let mut parser = Parser::new(source);
    let expression = parser
        .parse_expression_input()
        .expect("test expression should parse");

    interpreter
        .interpret_expression(&expression)
        .expect("test expression should evaluate successfully")
}
