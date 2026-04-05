use std::collections::HashMap;

use crate::{
    expr::Expr,
    interpreter::{Interpreter, ResolvedBinding},
    lox,
    stmt::Stmt,
    token::Token,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolveError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingKind {
    Variable,
    Parameter,
    Function,
    Class,
}

#[derive(Debug, Clone)]
struct BindingInfo {
    token: Token,
    kind: BindingKind,
    defined: bool,
    used: bool,
}

pub(crate) struct Resolver<'a> {
    interpreter: &'a Interpreter,
    // Stack of lexical scopes being resolved. Each binding tracks whether it is
    // fully defined yet and whether it was ever read before the scope ended.
    scopes: Vec<HashMap<String, BindingInfo>>,
}

impl<'a> Resolver<'a> {
    pub(crate) fn new(interpreter: &'a Interpreter) -> Self {
        Self {
            interpreter,
            scopes: Vec::new(),
        }
    }

    // Public entry point for resolving a parsed statement list before execution.
    pub(crate) fn resolve_statements(&mut self, statements: &[Stmt]) -> Result<(), ResolveError> {
        for statement in statements {
            self.resolve_statement_node(statement)?;
        }

        Ok(())
    }

    // Public entry point for resolving a standalone expression, mainly for the REPL.
    pub(crate) fn resolve_expression(&mut self, expression: &Expr) -> Result<(), ResolveError> {
        self.resolve_expression_node(expression)
    }

    // Recursively walk one statement node, creating or discarding scopes when
    // the syntax introduces them.
    fn resolve_statement_node(&mut self, statement: &Stmt) -> Result<(), ResolveError> {
        match statement {
            Stmt::Block { statements } => {
                self.begin_scope();
                let result = self.resolve_statements(statements);
                self.finish_scope(result)
            }
            Stmt::Break => Ok(()),
            Stmt::Class { name, methods: _ } => {
                // Class names behave like declarations in the surrounding
                // scope. Method resolution comes in a later class section.
                self.declare(name, BindingKind::Class)?;
                self.define(name);
                Ok(())
            }
            Stmt::Expression { expression } => self.resolve_expression_node(expression),
            Stmt::Function { name, params, body } => {
                self.declare(name, BindingKind::Function)?;
                self.define(name);
                self.resolve_function(params, body)
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expression_node(condition)?;
                self.resolve_statement_node(then_branch)?;
                if let Some(else_branch) = else_branch {
                    self.resolve_statement_node(else_branch)?;
                }
                Ok(())
            }
            Stmt::Print { expression } => self.resolve_expression_node(expression),
            Stmt::Return { value, .. } => {
                if let Some(value) = value {
                    self.resolve_expression_node(value)?;
                }
                Ok(())
            }
            Stmt::Var { name, initializer } => {
                self.declare(name, BindingKind::Variable)?;
                if let Some(initializer) = initializer {
                    self.resolve_expression_node(initializer)?;
                }
                self.define(name);
                Ok(())
            }
            Stmt::While { condition, body } => {
                self.resolve_expression_node(condition)?;
                self.resolve_statement_node(body)
            }
        }
    }

    // Recursively walk one expression node and resolve any variable reads or
    // writes it contains to their lexical scope distance.
    fn resolve_expression_node(&mut self, expression: &Expr) -> Result<(), ResolveError> {
        match expression {
            Expr::Assign { name, value } => {
                self.resolve_expression_node(value)?;
                self.resolve_local(name, false);
                Ok(())
            }
            Expr::Binary { left, right, .. } => {
                self.resolve_expression_node(left)?;
                self.resolve_expression_node(right)
            }
            Expr::Call {
                callee, arguments, ..
            } => {
                self.resolve_expression_node(callee)?;
                for argument in arguments {
                    self.resolve_expression_node(argument)?;
                }
                Ok(())
            }
            Expr::Get { object, .. } => self.resolve_expression_node(object),
            Expr::Grouping { expression } => self.resolve_expression_node(expression),
            Expr::Literal { .. } => Ok(()),
            Expr::Logical { left, right, .. } => {
                self.resolve_expression_node(left)?;
                self.resolve_expression_node(right)
            }
            Expr::Set { object, value, .. } => {
                self.resolve_expression_node(value)?;
                self.resolve_expression_node(object)
            }
            Expr::Variable { name } => {
                if self
                    .scopes
                    .last()
                    .and_then(|scope| scope.get(&name.lexeme))
                    .is_some_and(|binding| !binding.defined)
                {
                    return Err(
                        self.error(name, "Can't read local variable in its own initializer.")
                    );
                }

                self.resolve_local(name, true);
                Ok(())
            }
            Expr::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expression_node(condition)?;
                self.resolve_expression_node(then_branch)?;
                self.resolve_expression_node(else_branch)
            }
            Expr::Unary { right, .. } => self.resolve_expression_node(right),
        }
    }

    // Resolve a function body in its own scope, with each parameter behaving
    // like a local variable declared at the start of that body.
    fn resolve_function(&mut self, params: &[Token], body: &[Stmt]) -> Result<(), ResolveError> {
        self.begin_scope();

        for param in params {
            self.declare(param, BindingKind::Parameter)?;
            self.define(param);
        }

        let result = self.resolve_statements(body);
        self.finish_scope(result)
    }

    // Push a fresh lexical scope for a block or function body.
    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    // Pop the innermost lexical scope once resolution leaves it, reporting a
    // resolver error if a local variable in that scope was never read.
    fn end_scope(&mut self) -> Result<(), ResolveError> {
        let Some(scope) = self.scopes.pop() else {
            return Ok(());
        };

        let unused = scope
            .values()
            .filter(|binding| {
                binding.kind == BindingKind::Variable && binding.defined && !binding.used
            })
            .min_by_key(|binding| binding.token.id);

        if let Some(binding) = unused {
            return Err(self.error(
                &binding.token,
                &format!("Local variable '{}' is never used.", binding.token.lexeme),
            ));
        }

        Ok(())
    }

    fn discard_scope(&mut self) {
        self.scopes.pop();
    }

    fn finish_scope(&mut self, result: Result<(), ResolveError>) -> Result<(), ResolveError> {
        match result {
            Ok(()) => self.end_scope(),
            Err(error) => {
                self.discard_scope();
                Err(error)
            }
        }
    }

    // Record a name in the current scope before its initializer resolves so
    // reads from the variable's own initializer can be rejected.
    fn declare(&mut self, name: &Token, kind: BindingKind) -> Result<(), ResolveError> {
        let Some(scope) = self.scopes.last_mut() else {
            return Ok(());
        };

        if scope.contains_key(&name.lexeme) {
            return Err(self.error(name, "Already a variable with this name in this scope."));
        }

        scope.insert(
            name.lexeme.clone(),
            BindingInfo {
                token: name.clone(),
                kind,
                defined: false,
                used: false,
            },
        );
        Ok(())
    }

    // Mark a previously declared local as fully available for reads.
    fn define(&mut self, name: &Token) {
        if let Some(binding) = self
            .scopes
            .last_mut()
            .and_then(|scope| scope.get_mut(&name.lexeme))
        {
            binding.defined = true;
        }
    }

    // Find how many scopes outward this name resolves to and hand that lexical
    // distance to the interpreter for later fast runtime lookup.
    fn resolve_local(&mut self, name: &Token, mark_used: bool) {
        for (distance, scope) in self.scopes.iter_mut().rev().enumerate() {
            if let Some(binding) = scope.get_mut(&name.lexeme) {
                if mark_used {
                    binding.used = true;
                }
                self.interpreter
                    .resolve(name, ResolvedBinding::Local(distance));
                return;
            }
        }

        self.interpreter.resolve(name, ResolvedBinding::Global);
    }

    // Report a resolver error through the shared Lox error reporter and stop
    // the current resolution walk.
    fn error(&self, token: &Token, message: &str) -> ResolveError {
        lox::token_error(token, message);
        ResolveError
    }
}
