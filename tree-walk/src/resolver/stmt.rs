use super::{BindingKind, ClassType, FunctionType, ResolveError, Resolver};
use crate::{
    expr::Expr,
    stmt::{FunctionDecl, Stmt},
    token::Token,
};

impl<'a> Resolver<'a> {
    // Public entry point for resolving a parsed statement list before execution.
    pub(crate) fn resolve_statements(&mut self, statements: &[Stmt]) -> Result<(), ResolveError> {
        for statement in statements {
            self.resolve_statement_node(statement)?;
        }

        Ok(())
    }

    // Recursively walk one statement node, creating or discarding scopes when
    // the syntax introduces them.
    pub(super) fn resolve_statement_node(&mut self, statement: &Stmt) -> Result<(), ResolveError> {
        match statement {
            Stmt::Block { statements } => {
                self.begin_scope();
                let result = self.resolve_statements(statements);
                self.finish_scope(result)
            }
            Stmt::Break => Ok(()),
            Stmt::Class {
                name,
                superclass,
                methods,
            } => self.resolve_class_statement(name, superclass.as_ref(), methods),
            Stmt::Expression { expression } => self.resolve_expression_node(expression),
            Stmt::Function(function) => {
                self.declare(&function.name, BindingKind::Function)?;
                self.define(&function.name);
                self.resolve_function(function, FunctionType::Function)
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
            Stmt::Return { keyword, value } => {
                if let Some(value) = value {
                    if self.current_function == FunctionType::Initializer {
                        return Err(
                            self.error(keyword, "Can't return a value from an initializer.")
                        );
                    }
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

    fn resolve_class_statement(
        &mut self,
        name: &Token,
        superclass: Option<&Expr>,
        methods: &[FunctionDecl],
    ) -> Result<(), ResolveError> {
        // Class names behave like declarations in the surrounding scope.
        // Class methods then reuse the existing function-body resolver so
        // their local bindings are prepared before run time.
        let enclosing_class = self.current_class;
        self.current_class = if superclass.is_some() {
            ClassType::Subclass
        } else {
            ClassType::Class
        };
        let result = self.resolve_class_declaration(name, superclass, methods);

        self.current_class = enclosing_class;
        result
    }

    fn resolve_class_declaration(
        &mut self,
        name: &Token,
        superclass: Option<&Expr>,
        methods: &[FunctionDecl],
    ) -> Result<(), ResolveError> {
        self.declare(name, BindingKind::Class)?;
        self.define(name);

        self.resolve_declared_superclass(name, superclass)?;

        if superclass.is_some() {
            self.begin_scope();
            self.define_super(name.line);
        }

        self.begin_scope();
        self.define_this(name.line);

        let result = self.resolve_class_methods(methods);
        let result = self.finish_scope(result);

        if superclass.is_some() {
            self.finish_scope(result)
        } else {
            result
        }
    }

    fn resolve_declared_superclass(
        &mut self,
        name: &Token,
        superclass: Option<&Expr>,
    ) -> Result<(), ResolveError> {
        let Some(superclass) = superclass else {
            return Ok(());
        };

        let Expr::Variable {
            name: superclass_name,
        } = superclass
        else {
            unreachable!("parser should only build variable-shaped superclasses");
        };

        if name.lexeme == superclass_name.lexeme {
            return Err(self.error(superclass_name, "A class can't inherit from itself."));
        }

        self.resolve_expression_node(superclass)
    }

    fn resolve_class_methods(&mut self, methods: &[FunctionDecl]) -> Result<(), ResolveError> {
        for method in methods {
            let function_type = if method.name.lexeme.as_ref() == "init" {
                FunctionType::Initializer
            } else {
                FunctionType::Method
            };
            self.resolve_function(method, function_type)?;
        }

        Ok(())
    }

    // Resolve a function body in its own scope, with each parameter behaving
    // like a local variable declared at the start of that body.
    fn resolve_function(
        &mut self,
        function: &FunctionDecl,
        function_type: FunctionType,
    ) -> Result<(), ResolveError> {
        let enclosing_function = self.current_function;
        self.current_function = function_type;
        self.begin_scope();

        let result = (|| {
            for param in &function.params {
                self.declare(param, BindingKind::Parameter)?;
                self.define(param);
            }

            self.resolve_statements(&function.body)
        })();

        let result = self.finish_scope(result);
        self.current_function = enclosing_function;
        result
    }
}
