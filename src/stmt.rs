use crate::expr::Expr;
use crate::token::Token;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Stmt {
    Expression { expression: Expr },
    Print { expression: Expr },
    Var {
        name: Token,
        initializer: Option<Expr>,
    },
}

#[allow(dead_code)]
impl Stmt {
    // Construct a statement that evaluates an expression for its side effects.
    pub fn expression(expression: Expr) -> Self {
        Stmt::Expression { expression }
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
