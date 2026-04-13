use crate::expr::{Expr, ExprArenaRef};
use crate::token::Token;

#[derive(Debug, Clone)]
pub(crate) struct FunctionDecl {
    pub(crate) name: Token,
    pub(crate) params: Vec<Token>,
    pub(crate) body: Vec<Stmt>,
    // Parser construction is two-phase here: it first builds the function body
    // while owning a mutable `ExprArena`, then `attach_exprs_to_function()`
    // fills this handle in before the parsed program escapes the parser.
    // Later passes rely on every parsed function carrying that shared arena so
    // nested `ExprRef` handles in the body can always be resolved safely.
    pub(crate) expr_arena: Option<ExprArenaRef>,
}

#[derive(Debug, Clone)]
pub(crate) enum Stmt {
    Block {
        statements: Vec<Stmt>,
    },
    Break,
    Class {
        name: Token,
        superclass: Option<Expr>,
        methods: Vec<FunctionDecl>,
    },
    Expression {
        expression: Expr,
    },
    Function(FunctionDecl),
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    Print {
        expression: Expr,
    },
    Return {
        keyword: Token,
        value: Option<Expr>,
    },
    Var {
        name: Token,
        initializer: Option<Expr>,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
    },
}

impl FunctionDecl {
    // Construct a function or method declaration with its name, parameters,
    // and body statements.
    pub(crate) fn new(name: Token, params: Vec<Token>, body: Vec<Stmt>) -> Self {
        Self {
            name,
            params,
            body,
            expr_arena: None,
        }
    }

    pub(crate) fn expr_arena(&self) -> &crate::expr::ExprArena {
        self.expr_arena
            .as_deref()
            .expect("parsed function declarations should carry their expression arena")
    }

    pub(crate) fn expr_arena_ref(&self) -> ExprArenaRef {
        self.expr_arena
            .clone()
            .expect("parsed function declarations should carry their expression arena")
    }
}

impl Stmt {
    // Construct a block statement with its nested declarations and statements.
    pub(crate) fn block(statements: Vec<Stmt>) -> Self {
        Stmt::Block { statements }
    }

    // Construct a break statement that exits the nearest enclosing loop.
    pub(crate) fn break_stmt() -> Self {
        Stmt::Break
    }

    // Construct a class declaration with its name and parsed method declarations.
    pub(crate) fn class(name: Token, superclass: Option<Expr>, methods: Vec<FunctionDecl>) -> Self {
        Stmt::Class {
            name,
            superclass,
            methods,
        }
    }

    // Construct a statement that evaluates an expression for its side effects.
    pub(crate) fn expression(expression: Expr) -> Self {
        Stmt::Expression { expression }
    }

    // Construct a function declaration with its name, parameters, and body.
    pub(crate) fn function(declaration: FunctionDecl) -> Self {
        Stmt::Function(declaration)
    }

    // Construct an if statement with an optional else branch.
    pub(crate) fn if_stmt(condition: Expr, then_branch: Stmt, else_branch: Option<Stmt>) -> Self {
        Stmt::If {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
        }
    }

    // Construct a statement that evaluates and prints an expression.
    pub(crate) fn print(expression: Expr) -> Self {
        Stmt::Print { expression }
    }

    // Construct a return statement with an optional value expression.
    pub(crate) fn return_stmt(keyword: Token, value: Option<Expr>) -> Self {
        Stmt::Return { keyword, value }
    }

    // Construct a variable declaration with an optional initializer.
    pub(crate) fn var(name: Token, initializer: Option<Expr>) -> Self {
        Stmt::Var { name, initializer }
    }

    // Construct a while statement with a condition and loop body.
    pub(crate) fn while_stmt(condition: Expr, body: Stmt) -> Self {
        Stmt::While {
            condition,
            body: Box::new(body),
        }
    }
}
