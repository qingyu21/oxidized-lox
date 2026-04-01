use crate::expr::Expr;
use crate::token::Token;
#[derive(Debug, Clone)]
pub enum Stmt {
    Block {
        statements: Vec<Stmt>,
    },
    Break,
    Expression {
        expression: Expr,
    },
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    Print {
        expression: Expr,
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

impl Stmt {
    // Construct a block statement with its nested declarations and statements.
    pub fn block(statements: Vec<Stmt>) -> Self {
        Stmt::Block { statements }
    }

    // Construct a break statement that exits the nearest enclosing loop.
    pub fn break_stmt() -> Self {
        Stmt::Break
    }

    // Construct a statement that evaluates an expression for its side effects.
    pub fn expression(expression: Expr) -> Self {
        Stmt::Expression { expression }
    }

    // Construct an if statement with an optional else branch.
    pub fn if_stmt(condition: Expr, then_branch: Stmt, else_branch: Option<Stmt>) -> Self {
        Stmt::If {
            condition,
            then_branch: Box::new(then_branch),
            else_branch: else_branch.map(Box::new),
        }
    }

    // Construct a statement that evaluates and prints an expression.
    pub fn print(expression: Expr) -> Self {
        Stmt::Print { expression }
    }

    // Construct a variable declaration with an optional initializer.
    pub fn var(name: Token, initializer: Option<Expr>) -> Self {
        Stmt::Var { name, initializer }
    }

    // Construct a while statement with a condition and loop body.
    pub fn while_stmt(condition: Expr, body: Stmt) -> Self {
        Stmt::While {
            condition,
            body: Box::new(body),
        }
    }
}
