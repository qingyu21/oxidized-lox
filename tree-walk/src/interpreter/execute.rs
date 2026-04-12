use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    environment::{Environment, EnvironmentRef},
    expr::{Expr, ExprArena},
    runtime::{LoxClass, RuntimeError, Value},
    stmt::{FunctionDecl, Stmt},
    token::Token,
};

use super::{
    ControlFlow, Interpreter,
    callable::{make_function, make_function_ref},
};

struct EnvironmentGuard<'a> {
    slot: &'a RefCell<EnvironmentRef>,
    previous: Option<EnvironmentRef>,
}

impl<'a> EnvironmentGuard<'a> {
    fn replace(slot: &'a RefCell<EnvironmentRef>, environment: EnvironmentRef) -> Self {
        let previous = slot.replace(environment);
        Self {
            slot,
            previous: Some(previous),
        }
    }
}

impl Drop for EnvironmentGuard<'_> {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            self.slot.replace(previous);
        }
    }
}

impl Interpreter {
    pub(super) fn execute_all(
        &self,
        statements: &[Stmt],
        expr_arena: &ExprArena,
    ) -> Result<ControlFlow, RuntimeError> {
        for stmt in statements {
            match self.execute(stmt, expr_arena)? {
                ControlFlow::None => {}
                ControlFlow::Break => return Ok(ControlFlow::Break),
                ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
            }
        }

        Ok(ControlFlow::None)
    }

    pub(super) fn execute(
        &self,
        stmt: &Stmt,
        expr_arena: &ExprArena,
    ) -> Result<ControlFlow, RuntimeError> {
        match stmt {
            Stmt::Block { statements } => {
                let block_environment = Environment::new_enclosed_ref(self.current_environment());
                self.execute_block(statements, expr_arena, block_environment)
            }
            Stmt::Break => Ok(ControlFlow::Break),
            Stmt::Class {
                name,
                superclass,
                methods,
            } => self.execute_class_declaration(name, superclass.as_ref(), methods, expr_arena),
            Stmt::Expression { expression } => {
                self.evaluate(expression, expr_arena)?;
                Ok(ControlFlow::None)
            }
            Stmt::Function(function) => self.execute_function_declaration(function),
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => self.execute_if(condition, then_branch, else_branch.as_deref(), expr_arena),
            Stmt::Print { expression } => {
                let value = self.evaluate(expression, expr_arena)?;
                println!("{value}");
                Ok(ControlFlow::None)
            }
            Stmt::Return { keyword, value } => {
                self.execute_return(keyword, value.as_ref(), expr_arena)
            }
            Stmt::Var { name, initializer } => {
                let value = match initializer {
                    Some(initializer) => self.evaluate(initializer, expr_arena)?,
                    None => Value::Nil,
                };
                self.current_environment()
                    .borrow_mut()
                    .define(name.lexeme.to_rc(), value);
                Ok(ControlFlow::None)
            }
            Stmt::While { condition, body } => self.execute_while(condition, body, expr_arena),
        }
    }

    pub(super) fn execute_block(
        &self,
        statements: &[Stmt],
        expr_arena: &ExprArena,
        environment: EnvironmentRef,
    ) -> Result<ControlFlow, RuntimeError> {
        let _guard = EnvironmentGuard::replace(&self.environment, environment);
        self.execute_all(statements, expr_arena)
    }

    fn execute_function_declaration(
        &self,
        function: &FunctionDecl,
    ) -> Result<ControlFlow, RuntimeError> {
        // Function declarations are executable statements: evaluating one
        // creates a callable runtime value and binds it in the current scope.
        let value = make_function(
            function.expr_arena_ref(),
            &function.name,
            &function.params,
            &function.body,
            self.current_environment(),
        );
        self.current_environment()
            .borrow_mut()
            .define(function.name.lexeme.to_rc(), value);
        Ok(ControlFlow::None)
    }

    fn execute_class_declaration(
        &self,
        name: &Token,
        superclass_expr: Option<&Expr>,
        methods: &[FunctionDecl],
        expr_arena: &ExprArena,
    ) -> Result<ControlFlow, RuntimeError> {
        let superclass = if let Some(superclass_expr) = superclass_expr {
            let Expr::Variable {
                name: superclass_name,
            } = superclass_expr
            else {
                unreachable!("parser should only build variable-shaped superclasses");
            };

            match self.evaluate(superclass_expr, expr_arena)? {
                Value::Class(superclass) => Some(superclass),
                _ => {
                    return Err(RuntimeError::new(
                        superclass_name.clone(),
                        "Superclass must be a class.",
                    ));
                }
            }
        } else {
            None
        };

        // Bind the class name before creating the runtime class object so
        // later class chapters can support self-references from methods.
        self.current_environment()
            .borrow_mut()
            .define(name.lexeme.to_rc(), Value::Nil);

        // Subclass methods capture an extra environment where `super` points
        // at the declared superclass. Methods on classes without a superclass
        // keep closing over the surrounding environment directly.
        let method_closure = if let Some(superclass) = &superclass {
            let environment = Environment::new_enclosed_ref(self.current_environment());
            environment
                .borrow_mut()
                .define("super", Value::Class(superclass.clone()));
            environment
        } else {
            self.current_environment()
        };

        // Each parsed method becomes a runtime function closed over the
        // environment where the class declaration executes.
        let mut method_table = HashMap::new();
        for method in methods {
            let function = make_function_ref(
                method.expr_arena_ref(),
                &method.name,
                &method.params,
                &method.body,
                method_closure.clone(),
                method.name.lexeme.as_ref() == "init",
            );
            method_table.insert(method.name.lexeme.to_rc(), function);
        }

        // The runtime class object stores behavior in its method table, then
        // replaces the temporary nil binding we inserted above.
        let klass = Value::Class(Rc::new(LoxClass::new(
            name.lexeme.to_rc(),
            superclass,
            method_table,
        )));
        self.current_environment()
            .borrow_mut()
            .assign(name, klass)?;
        Ok(ControlFlow::None)
    }

    fn execute_return(
        &self,
        _keyword: &Token,
        value_expr: Option<&Expr>,
        expr_arena: &ExprArena,
    ) -> Result<ControlFlow, RuntimeError> {
        // Evaluate the optional return value and turn it into an internal
        // control-flow signal that enclosing statements can propagate upward.
        let value = match value_expr {
            Some(value_expr) => self.evaluate(value_expr, expr_arena)?,
            None => Value::Nil,
        };

        Ok(ControlFlow::Return(value))
    }

    fn execute_if(
        &self,
        condition: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
        expr_arena: &ExprArena,
    ) -> Result<ControlFlow, RuntimeError> {
        if Self::is_truthy(&self.evaluate(condition, expr_arena)?) {
            self.execute(then_branch, expr_arena)
        } else if let Some(else_branch) = else_branch {
            self.execute(else_branch, expr_arena)
        } else {
            Ok(ControlFlow::None)
        }
    }

    fn execute_while(
        &self,
        condition: &Expr,
        body: &Stmt,
        expr_arena: &ExprArena,
    ) -> Result<ControlFlow, RuntimeError> {
        while Self::is_truthy(&self.evaluate(condition, expr_arena)?) {
            match self.execute(body, expr_arena)? {
                ControlFlow::None => {}
                ControlFlow::Break => break,
                ControlFlow::Return(value) => return Ok(ControlFlow::Return(value)),
            }
        }

        Ok(ControlFlow::None)
    }
}
