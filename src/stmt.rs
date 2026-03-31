use crate::expr::Expr;
use crate::token::Token;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Stmt {
    Block {
        statements: Vec<Stmt>,
    },
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
}

#[allow(dead_code)]
impl Stmt {
    // Construct a block statement with its nested declarations and statements.
    pub fn block(statements: Vec<Stmt>) -> Self {
        Stmt::Block { statements }
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
}
